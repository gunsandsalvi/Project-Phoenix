//! Books at a declared scale for measuring settlement, not a world: a central bank, banks, and holders each banking
//! with one bank, holding a deposit there and owing it on loan lines shared with other holders of the bank — rows due
//! weekly on each weekday, so some fall due every business day, and the rest once a month on one day — so a day's
//! settlement reads every holder's run head, scans its rows and pays the ones due, through the real 1b and stage 7.

use phx_core::{
    AuditStream, BusinessDayConvention, Calendar, CountryRules, DayCount, EndOfMonth, GenReport, LegDigest, Period,
    ScheduleDates, WeekendRule,
};
use phx_id::{CountryId, Date, Day, LineId, PartyId, Slot, TableId, TileId, Weekday};
use phx_macros::clause;
use phx_num::{Ccy, Missing, Rate, RatePeriod, UnitId, capacity_exceeded, violation};
use phx_store::Backing;

use crate::algebra::{Leg, Reference, Schedule, Side, Terms};
use crate::books::{Books, BooksSize};
use crate::consts::{
    SYNTHETIC_DEPOSIT, SYNTHETIC_EPOCH_YEAR, SYNTHETIC_LOAN, SYNTHETIC_RATE, SYNTHETIC_ROWS_PER_CHUNK,
    SYNTHETIC_WINDOW_YEAR,
};
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, RowOp};
use crate::line::{LineKindDecl, NewRow, SideDecl};
use crate::money::MoneyHolders;
use crate::rows::{BALANCE, Optional, PENDING};
use crate::terms::TermsId;

const CCY: Ccy = Ccy::new(0);
use crate::consts::{MONTHS_A_YEAR as MONTHS, WEEKDAYS};
const HOLDER: &str = "holder";
const BANK: &str = "bank";
const CENTRAL_BANK: &str = "central bank";
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &[CENTRAL_BANK],
    banks: &[BANK],
    treasuries: &[],
    depositors: &[HOLDER],
    requesters: &["BNK"],
};
const LOAN: LineKindDecl = LineKindDecl {
    name: "loan",
    asset: SideDecl {
        holder_kinds: &[BANK],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &[HOLDER],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    transfer_requesters: &["BNK"],
    dated: true,
};

/// How large the books are: the holders, their loan rows falling due on each business day of a week and once a month,
/// the banks, and the holders sharing each loan line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettlementSize {
    pub holders: u32,
    pub daily_rows: u32,
    pub monthly_rows: u32,
    pub banks: u32,
    pub holders_per_line: u32,
}

/// The books built, their calendar, and the day every monthly row falls due.
#[derive(Debug)]
pub struct Settlement<B: Backing> {
    pub books: Books<B>,
    pub calendar: Calendar,
    pub monthly_due: Day,
}

/// An audit stream that keeps nothing, for measuring what settlement costs without the audit.
#[derive(Debug)]
pub struct Unaudited;

impl AuditStream for Unaudited {
    fn applied(&mut self, _: u64) {}
    fn touched(&mut self, _: TableId, _: Slot) {}
    fn leg(&mut self, _: u64, _: LegDigest) {}
}

fn open_row(party: PartyId, line: LineId, side: Side, count: u32, words: u8) -> LegRec {
    let word = |f: u8| if words & f == 0 { Missing::Absent } else { Missing::Present(0) };
    let optional = Optional { balance: word(BALANCE), pending: word(PENDING), amount: Missing::Absent };
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: i64::from(count),
        denom: Denom::Unit(UnitId::new(0)),
        kind: LegKind::Row(RowOp::Open(NewRow { side, within: 0, count, point: 0, optional })),
    }
}

fn write(party: PartyId, line: LineId, side: Side, qty: i64) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(CCY),
        kind: LegKind::OpeningWrite { identity: 1, cost: 0 },
    }
}

fn narrow(n: usize) -> u32 {
    let Ok(v) = u32::try_from(n) else { capacity_exceeded!("synthetic settlement rows", u32::MAX, n) };
    v
}

/// The calendar the books are dated by: Saturdays and Sundays closed, and the holidays given.
///
/// # Errors
/// Holidays a calendar cannot hold.
pub fn calendar(holidays: &[Date]) -> Result<Calendar, String> {
    let rules = CountryRules {
        weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
        holidays: holidays
            .iter()
            .map(|d| phx_core::HolidayRule::Fixed { name: "holiday".to_owned(), month: d.month(), day: d.day() })
            .collect(),
    };
    let Some(epoch) = Date::new(SYNTHETIC_EPOCH_YEAR, 1, 1) else { return Err("no epoch".to_owned()) };
    Calendar::new(epoch, vec![(CountryId::new(0), rules)], SYNTHETIC_WINDOW_YEAR)
}

/// The books at `size`, dated from `first`, a Monday, the weekly rows anchored on `week_before`, the Monday before
/// it, and the monthly rows due on `monthly`.
///
/// # Errors
/// A calendar that cannot hold the holidays or dates given.
#[clause("REP.31", "MON.5")]
pub fn settlement<B: Backing>(
    size: SettlementSize,
    calendar: Calendar,
    (week_before, first, monthly): (Date, Date, Date),
) -> Result<Settlement<B>, String> {
    let weekly = size.daily_rows * WEEKDAYS;
    let per_holder = weekly + size.monthly_rows + 1;
    let Some(rows) = size.holders.checked_mul(per_holder).and_then(|r| r.checked_mul(2)) else {
        return Err("more synthetic rows than a table holds".to_owned());
    };
    // Every bank's holders fill their lines, the last of each layer partly: a line a layer per bank more than the
    // holders' share, and each bank's deposit line and reserves row.
    let groups = size.holders.div_ceil(size.holders_per_line) + size.banks;
    let lines = groups * (weekly + size.monthly_rows) + size.banks + 1;
    // A kind table holds parties, the holders and the banks; their rows live in the holders' arenas.
    let Some(parties) = size.holders.checked_add(size.banks).and_then(|p| p.checked_add(1)) else {
        return Err("more synthetic parties than a table holds".to_owned());
    };
    let books_size = BooksSize {
        rows: parties.next_power_of_two(),
        rows_per_chunk: SYNTHETIC_ROWS_PER_CHUNK,
        instruments: 1,
        lines: lines.next_power_of_two(),
        per_chunk: SYNTHETIC_ROWS_PER_CHUNK,
        // Each of a holder list's shards holds its share of the rows.
        blocks: rows.div_ceil(crate::consts::HOLDER_SHARDS).next_power_of_two(),
    };
    let mut books: Books<B> = Books::new(&[CENTRAL_BANK, BANK, HOLDER], books_size);
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let loan_kind = books.ledger.lines.declare_money(LOAN).index();
    let (account, weekdays, (monthly_terms, monthly_due)) =
        schedules(&mut books, &calendar, (week_before, first, monthly))?;
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
        held: phx_num::Missing::Absent,
    });
    let mut report = GenReport::default();
    let cb = books.parties.begin(CENTRAL_BANK, TileId::new(0), Day::new(0));
    let banks: Vec<PartyId> = (0..size.banks).map(|_| books.parties.begin(BANK, TileId::new(0), Day::new(0))).collect();
    let holders: Vec<PartyId> =
        (0..size.holders).map(|_| books.parties.begin(HOLDER, TileId::new(0), Day::new(0))).collect();
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let mut legs = vec![open_row(cb, reserves, Side::Liability, size.banks, BALANCE)];
    legs.extend(banks.iter().map(|b| open_row(*b, reserves, Side::Asset, 1, BALANCE)));
    books.open(reason, legs, 0, &mut report);
    // Each bank's holders: every holder banks with the bank its place gives, one deposit line a bank.
    let mut rows_of: Vec<Vec<(LineId, Side)>> = vec![Vec::new(); holders.len()];
    for (b, bank) in banks.iter().enumerate() {
        let mine: Vec<usize> = (0..holders.len()).filter(|h| h % banks.len() == b).collect();
        let deposits = books.ledger.lines.open(deposit_kind, account, Missing::Absent);
        let mut legs = vec![open_row(*bank, deposits, Side::Liability, narrow(mine.len()), BALANCE)];
        for h in &mine {
            let Some(holder) = holders.get(*h) else { continue };
            legs.push(open_row(*holder, deposits, Side::Asset, 1, BALANCE | PENDING));
            if let Some(r) = rows_of.get_mut(*h) {
                r.push((deposits, Side::Asset));
            }
        }
        books.open(reason, legs, 0, &mut report);
        let weekly_layers = weekdays.iter().flat_map(|w| (0..size.daily_rows).map(move |_| *w));
        let layers = weekly_layers.chain((0..size.monthly_rows).map(|_| (monthly_terms, monthly_due)));
        for (terms, due) in layers {
            for group in mine.chunks(usize::try_from(size.holders_per_line).unwrap_or(usize::MAX)) {
                let line = books.ledger.lines.open(loan_kind, terms, Missing::Present((due, 1)));
                let mut legs = vec![open_row(*bank, line, Side::Asset, narrow(group.len()), BALANCE)];
                for h in group {
                    let Some(holder) = holders.get(*h) else { continue };
                    legs.push(open_row(*holder, line, Side::Liability, 1, BALANCE));
                    if let Some(r) = rows_of.get_mut(*h) {
                        r.push((line, Side::Liability));
                    }
                }
                books.open(reason, legs, 0, &mut report);
            }
        }
    }
    balances(&mut books, reason, (&holders, &banks), &rows_of);
    if books.parties.directory().live_count() == 0 {
        violation!(clause = "PTY.9", "synthetic books with no parties");
    }
    Ok(Settlement { books, calendar, monthly_due })
}

/// Each holder's balances, written against its bank's in one instruction apiece so no instruction is large.
fn balances<B: Backing>(
    books: &mut Books<B>,
    reason: crate::instruction::ReasonId,
    (holders, banks): (&[PartyId], &[PartyId]),
    rows_of: &[Vec<(LineId, Side)>],
) {
    for (h, holder) in holders.iter().enumerate() {
        let Some(bank) = banks.get(h % banks.len()) else { continue };
        let mut legs = Vec::new();
        for (line, side) in rows_of.get(h).map_or(&[][..], Vec::as_slice) {
            match side {
                Side::Asset => {
                    legs.push(write(*holder, *line, Side::Asset, SYNTHETIC_DEPOSIT));
                    legs.push(write(*bank, *line, Side::Liability, -SYNTHETIC_DEPOSIT));
                }
                Side::Liability => {
                    legs.push(write(*holder, *line, Side::Liability, -SYNTHETIC_LOAN));
                    legs.push(write(*bank, *line, Side::Asset, SYNTHETIC_LOAN));
                }
            }
        }
        books.open(reason, legs, 1, &mut GenReport::default());
    }
}

fn dates(anchor: Date, period: Period) -> ScheduleDates {
    ScheduleDates {
        anchor,
        period,
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: CountryId::new(0),
    }
}

/// The deposits' terms, the weekly loans' terms and first due day for each weekday from the first day, a Monday, and
/// the monthly loans' terms and due day. A schedule's first date is its anchor's next, so each is anchored a period
/// before its first due day.
type Schedules = (TermsId, Vec<(TermsId, Day)>, (TermsId, Day));

fn schedules<B: Backing>(
    books: &mut Books<B>,
    calendar: &Calendar,
    (week_before, first, monthly): (Date, Date, Date),
) -> Result<Schedules, String> {
    let (Some(week), Some(month)) = (Period::weeks(1), Period::months(1)) else {
        return Err("no period of a week or a month".to_owned());
    };
    let account = books.ledger.terms.intern(Terms::account(CCY, dates(first, month)));
    let loan_terms = |anchor: Date, period: Period| Terms {
        legs: vec![Leg::RateOnNotional {
            reference: Reference::Fixed(Rate::new(SYNTHETIC_RATE, RatePeriod::Year)),
            day_count: DayCount::Act365F,
        }],
        schedule: Schedule { dates: dates(anchor, period), count: Missing::Absent },
        ..Terms::account(CCY, dates(anchor, period))
    };
    let mut weekdays = Vec::new();
    for w in 0..WEEKDAYS {
        let Some(due) = calendar.day(Day::new(w).date(first)) else {
            return Err("a weekday off the calendar".to_owned());
        };
        weekdays.push((books.ledger.terms.intern(loan_terms(Day::new(w).date(week_before), week)), due));
    }
    let month_before = match monthly.month() {
        1 => Date::new(monthly.year() - 1, MONTHS, monthly.day()),
        m => Date::new(monthly.year(), m - 1, monthly.day()),
    };
    let Some(month_before) = month_before else { return Err("no month before the monthly due day".to_owned()) };
    let Some(monthly_due) = calendar.day(monthly) else { return Err("a monthly due day off the calendar".to_owned()) };
    Ok((account, weekdays, (books.ledger.terms.intern(loan_terms(month_before, month)), monthly_due)))
}
