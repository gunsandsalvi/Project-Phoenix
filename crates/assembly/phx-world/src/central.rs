//! The central bank's fund stage at 8d on each of its country's business days: each bank's overnight positions of
//! the day before returned with their interest, then each bank's request met — its excess reserves placed at the
//! deposit facility, its shortfall borrowed from the lending facility against its eligible loans; and on a month's
//! first fund stage the central bank's net income remitted to the treasury.

use std::collections::BTreeMap;

use if_credit::central::{CentralKind, Corridor, Request, RequestIn};
use phx_core::calendar::period::ScheduleDates;
use phx_core::{Declarations, OpeningCountry, Register, SubStep};
use phx_id::{CountryId, Day, LineId, PartyId};
use phx_ledger::algebra::{Side, Terms};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::ReasonId;
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};

use crate::world::World;

/// What the fund stage carries across days: each bank's reserves as a share of its deposits when it first came, the
/// target it keeps; each country's facility lines and the day its fund stage last ran; each central bank's net
/// income not yet remitted; and the month it last remitted.
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub(crate) struct CentralBook {
    pub targets: BTreeMap<PartyId, f64>,
    pub lines: BTreeMap<u8, (LineId, LineId)>,
    pub last: BTreeMap<u8, Day>,
    pub income: BTreeMap<PartyId, i64>,
    pub remitted: BTreeMap<PartyId, u32>,
}

/// The day's use of the facilities, for the counters and the checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CentralDay {
    pub uses: u64,
    pub placed: i64,
    pub borrowed: i64,
    pub interest_paid: i64,
    pub interest_received: i64,
    pub remitted: i64,
}

/// The central bank as the world keeps it: its kind, each country's corridor, its book and the day's tally.
#[derive(Debug, Default)]
pub(crate) struct Central {
    pub kind: Option<CentralKind>,
    pub corridors: Vec<Corridor>,
    pub book: CentralBook,
    pub day: CentralDay,
}

/// The central bank's kind the systems declare, with each country's corridor; none when no system declares one.
pub(crate) fn bind(
    d: &Declarations,
    register: &Register,
    countries: &[OpeningCountry],
    book: CentralBook,
) -> Result<Central, Vec<String>> {
    let kinds: Vec<CentralKind> =
        d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<CentralKind>()).copied().collect();
    let [kind] = kinds.as_slice() else {
        return if kinds.is_empty() {
            Ok(Central { book, ..Central::default() })
        } else {
            Err(vec!["more than one central bank's kind".to_owned()])
        };
    };
    let mut errors = Vec::new();
    let corridors = countries
        .iter()
        .filter_map(|c| match (kind.corridor)(register, c) {
            Ok(k) => Some(k),
            Err(e) => {
                errors.push(format!("the corridor in country {}: {e}", c.id.get()));
                None
            }
        })
        .collect();
    if errors.is_empty() {
        Ok(Central { kind: Some(*kind), corridors, book, ..Central::default() })
    } else {
        Err(errors)
    }
}

/// A party's balance on its row of a line of a kind, and the row's line: its first such row.
fn row_of(w: &World, party: PartyId, kind: &str, side: Side) -> Option<(LineId, i64)> {
    let lines = &w.books.ledger.lines;
    let k = lines.kind_index(kind);
    let (place, slot) = w.books.parties.row(party);
    phx_ledger::rows::rows(w.books.parties.holder(place), slot)
        .into_iter()
        .find(|r| r.side() == side && lines.kind_of(r.row.line) == k)
        .map(|r| (r.row.line, if let Missing::Present(b) = r.optional.balance { b } else { 0 }))
}

impl World {
    /// 8d: each central bank's fund stage on its country's business days.
    #[clause("CB.7", "CB.10", "TIME.6", "MKT.8")]
    pub(crate) fn central_fund(&mut self, day: Day) {
        let Some(kind) = self.central.kind else { return };
        let central_banks: Vec<PartyId> = self.books.parties.of_kind(kind.central_bank).collect();
        for cb in central_banks {
            let Missing::Present(country) = self.country_of_party(cb) else { continue };
            if !self.calendar.is_business(country, day) {
                continue;
            }
            self.fund_stage(day, (cb, country), &kind);
        }
    }

    /// The reason a facility's movements are recorded under.
    fn reason_named(&self, name: &str) -> ReasonId {
        match self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(name)) {
            Missing::Present(r) => r,
            Missing::Absent => violation!(clause = "CB.7", "the facilities under a reason never declared"),
        }
    }

    /// A country's facility lines, opened the first time its fund stage runs.
    fn facility_lines(&mut self, kind: &CentralKind, country: CountryId) -> (LineId, LineId) {
        if let Some(l) = self.central.book.lines.get(&country.get()) {
            return *l;
        }
        let ccy = phx_ledger::opening::currency(country);
        let dates: ScheduleDates = phx_ledger::opening::monthly(self.calendar.date(self.today), country);
        let terms = self.books.ledger.terms.intern(Terms::account(ccy, dates));
        let lines = &mut self.books.ledger.lines;
        let (d, l) = (lines.kind_index(kind.deposit_facility), lines.kind_index(kind.lending_facility));
        let pair = (lines.open(d, terms, Missing::Absent), lines.open(l, terms, Missing::Absent));
        self.central.book.lines.insert(country.get(), pair);
        pair
    }

    /// One country's fund stage: yesterday's positions returned with interest, each bank's request met, and the
    /// month's remittance.
    fn fund_stage(&mut self, day: Day, (cb, country): (PartyId, CountryId), kind: &CentralKind) {
        let Some(corridor) = self.central.corridors.get(usize::from(country.get())).copied() else { return };
        let (df, lf) = self.facility_lines(kind, country);
        let ccy = phx_ledger::opening::currency(country);
        let last = self.central.book.last.insert(country.get(), day);
        let days = last.and_then(|l| self.calendar.days_between(l, day)).map_or(0.0, f64::from);
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S8d));
        let (moved, interest) = (self.reason_named(kind.moved), self.reason_named(kind.interest));
        let banks: Vec<PartyId> = self
            .books
            .parties
            .of_kind(kind.bank)
            .filter(|b| self.country_of_party(*b) == Missing::Present(country))
            .collect();
        for bank in banks {
            let Some((reserves, _)) = row_of(self, bank, kind.reserves, Side::Asset) else { continue };
            let lines = (reserves, df, lf);
            self.return_positions((cb, bank, ccy), lines, (corridor, days), (moved, interest, m));
            let Some((_, held)) = row_of(self, bank, kind.reserves, Side::Asset) else { continue };
            let deposits = self.deposits_of(bank);
            let ratio = *self.central.book.targets.entry(bank).or_insert_with(|| {
                if deposits > 0 { phx_rand::float::from_i64(held) / phx_rand::float::from_i64(deposits) } else { 0.0 }
            });
            let target = phx_ledger::opening::whole(ratio * phx_rand::float::from_i64(deposits));
            let collateral = phx_ledger::opening::whole((1.0 - corridor.haircut) * self.eligible_loans(bank));
            let Some(credit) = self.credit.kind else { continue };
            let r = (credit.request)(&RequestIn { reserves: held, target, collateral });
            self.meet_request((cb, bank, ccy), lines, r, (moved, m));
        }
        self.remit(day, (cb, country, ccy), kind, m);
    }

    /// A bank's overnight positions returned: its deposit at the facility with the day's interest the central bank
    /// pays, its loan from it with the interest it charges.
    fn return_positions(
        &mut self,
        (cb, bank, ccy): (PartyId, PartyId, Ccy),
        (reserves, df, lf): (LineId, LineId, LineId),
        (corridor, days): (Corridor, f64),
        (moved, interest, m): (ReasonId, ReasonId, phx_ledger::transfer::MoveAt),
    ) {
        let year = crate::consts::DAYS_A_YEAR;
        let placed = self.balance_of(bank, df, Side::Asset);
        let owed = -self.balance_of(bank, lf, Side::Liability);
        let post = |w: &mut World, moves: &[(PartyId, LineId, Side, i64)], reason: ReasonId| {
            if !w.post_moves(moves, (cb, ccy), (reason, m)) {
                violation!(clause = "CB.7", "a facility's position that did not settle", bank = bank.get());
            }
        };
        if placed > 0 {
            post(
                self,
                &[
                    (bank, df, Side::Asset, -placed),
                    (cb, df, Side::Liability, placed),
                    (bank, reserves, Side::Asset, placed),
                    (cb, reserves, Side::Liability, -placed),
                ],
                moved,
            );
            let paid =
                phx_ledger::opening::whole(phx_rand::float::from_i64(placed) * corridor.deposit_rate * days / year);
            if paid > 0 {
                post(self, &[(bank, reserves, Side::Asset, paid), (cb, reserves, Side::Liability, -paid)], interest);
                *self.central.book.income.entry(cb).or_insert(0) -= paid;
                self.central.day.interest_paid += paid;
            }
        }
        if owed > 0 {
            post(
                self,
                &[
                    (bank, lf, Side::Liability, owed),
                    (cb, lf, Side::Asset, -owed),
                    (bank, reserves, Side::Asset, -owed),
                    (cb, reserves, Side::Liability, owed),
                ],
                moved,
            );
            let charged =
                phx_ledger::opening::whole(phx_rand::float::from_i64(owed) * corridor.lending_rate * days / year);
            if charged > 0 {
                post(
                    self,
                    &[(bank, reserves, Side::Asset, -charged), (cb, reserves, Side::Liability, charged)],
                    interest,
                );
                *self.central.book.income.entry(cb).or_insert(0) += charged;
                self.central.day.interest_received += charged;
            }
        }
    }

    /// A bank's request met: its excess placed overnight, its shortfall lent overnight.
    fn meet_request(
        &mut self,
        (cb, bank, ccy): (PartyId, PartyId, Ccy),
        (reserves, df, lf): (LineId, LineId, LineId),
        r: Request,
        (moved, m): (ReasonId, phx_ledger::transfer::MoveAt),
    ) {
        let mut legs = Vec::new();
        if r.place > 0 {
            legs.extend([
                (bank, reserves, Side::Asset, -r.place),
                (cb, reserves, Side::Liability, r.place),
                (bank, df, Side::Asset, r.place),
                (cb, df, Side::Liability, -r.place),
            ]);
            self.central.day.placed += r.place;
            self.central.day.uses += 1;
        }
        if r.borrow > 0 {
            legs.extend([
                (cb, lf, Side::Asset, r.borrow),
                (bank, lf, Side::Liability, -r.borrow),
                (bank, reserves, Side::Asset, r.borrow),
                (cb, reserves, Side::Liability, -r.borrow),
            ]);
            self.central.day.borrowed += r.borrow;
            self.central.day.uses += 1;
        }
        if !legs.is_empty() && !self.post_moves(&legs, (cb, ccy), (moved, m)) {
            violation!(clause = "CB.7", "a bank's request that did not settle", bank = bank.get());
        }
    }

    /// Balances moved on facility and reserve rows in one instruction: where a bank opens its row on a facility line,
    /// the central bank's row there gains the member it answers, so both sides count the same contracts.
    fn post_moves(
        &mut self,
        moves: &[(PartyId, LineId, Side, i64)],
        (cb, ccy): (PartyId, Ccy),
        (reason, m): (ReasonId, phx_ledger::transfer::MoveAt),
    ) -> bool {
        let flip = |side: &Side| match side {
            Side::Asset => Side::Liability,
            Side::Liability => Side::Asset,
        };
        let mut owed: BTreeMap<(LineId, bool), u32> = BTreeMap::new();
        let mut rows: Vec<(PartyId, LineId, Side, u32, i64)> = Vec::new();
        for (party, line, side, qty) in moves.iter().filter(|(p, ..)| *p != cb) {
            let opens = self.row_absent(*party, *line, *side);
            rows.push((*party, *line, *side, u32::from(opens), *qty));
            if opens {
                *owed.entry((*line, flip(side) == Side::Asset)).or_insert(0) += 1;
            }
        }
        for (party, line, side, qty) in moves.iter().filter(|(p, ..)| *p == cb) {
            let n = owed.remove(&(*line, *side == Side::Asset)).unwrap_or(0);
            rows.push((*party, *line, *side, n, *qty));
        }
        for ((line, asset), n) in owed {
            rows.push((cb, line, if asset { Side::Asset } else { Side::Liability }, n, 0));
        }
        self.books.move_rows(rows.into_iter(), (ccy, Missing::Absent), (reason, m), self.audit.stream()).is_ok()
    }

    /// Whether a party holds no row on a line's side.
    fn row_absent(&self, party: PartyId, line: LineId, side: Side) -> bool {
        let (place, slot) = self.books.parties.row(party);
        phx_ledger::rows::find(self.books.parties.holder(place), slot, line, side).is_none()
    }

    /// A party's balance on its row of a line, none held as nothing.
    fn balance_of(&self, party: PartyId, line: LineId, side: Side) -> i64 {
        let (place, slot) = self.books.parties.row(party);
        match phx_ledger::rows::find(self.books.parties.holder(place), slot, line, side).map(|r| r.optional.balance) {
            Some(Missing::Present(b)) => b,
            _ => 0,
        }
    }

    /// What a bank owes its depositors: its balances on the deposit lines it issues.
    pub(crate) fn deposits_of(&self, bank: PartyId) -> i64 {
        let lines = &self.books.ledger.lines;
        let (place, slot) = self.books.parties.row(bank);
        phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Liability && lines.is_deposit(r.row.line))
            .map(|r| if let Missing::Present(b) = r.optional.balance { -b } else { 0 })
            .sum()
    }

    /// A bank's loans the lending facility lends against: its firm loans' balances.
    fn eligible_loans(&self, bank: PartyId) -> f64 {
        let Some(credit) = self.credit.kind else { return 0.0 };
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(credit.loan);
        let (place, slot) = self.books.parties.row(bank);
        phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Asset && lines.kind_of(r.row.line) == k)
            .map(|r| if let Missing::Present(b) = r.optional.balance { phx_rand::float::from_i64(b) } else { 0.0 })
            .sum()
    }

    /// On a month's first fund stage, the central bank's net income since its last remittance paid into the
    /// treasury's account; a loss is kept against its equity.
    fn remit(
        &mut self,
        day: Day,
        (cb, country, ccy): (PartyId, CountryId, Ccy),
        kind: &CentralKind,
        m: phx_ledger::transfer::MoveAt,
    ) {
        let date = self.calendar.date(day);
        let month =
            u32::try_from(i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month())).unwrap_or(0);
        if self.central.book.remitted.insert(cb, month) == Some(month) {
            return;
        }
        let Some(income) = self.central.book.income.get(&cb).copied().filter(|i| *i > 0) else { return };
        let treasury =
            self.books.parties.of_kind(kind.treasury).find(|t| self.country_of_party(*t) == Missing::Present(country));
        let Some(treasury) = treasury else { return };
        let Some((account, _)) = row_of(self, treasury, kind.account, Side::Asset) else { return };
        let reason = self.reason_named(kind.remitted);
        let moves = [(treasury, account, Side::Asset, income), (cb, account, Side::Liability, -income)];
        if self.books.move_balances(&moves, ccy, (reason, m), self.audit.stream()).is_ok() {
            self.central.book.income.insert(cb, 0);
            self.central.day.remitted += income;
        }
    }
}
