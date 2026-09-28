use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{
    Adjustment, BALANCES, CONTRACTS, Contribution, DECLARATIONS, Opening, OpeningPhase, PARTIES, StreamDef, apportion,
    opening_subject,
};
use phx_id::{CountryId, PartyId};
use phx_ledger::algebra::{Side, Terms};
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, ReasonDecl, ReasonId};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::money::MoneyHolders;
use phx_ledger::opening::{currency, derived, key, open_row, whole, write};
use phx_ledger::rows::BALANCE;
use phx_macros::clause;
use phx_num::violation;

use crate::consts::PERCENT;
use crate::{CENTRAL_BANK, OpeningStream, TREASURY};

const BANKS: &str = "BNK.banks";
const CENTRAL: &str = "CB.central_bank";
const TREASURIES: &str = "CB.treasury";

/// The money lines' holders as the central bank declares them: banks hold reserves, the treasury its account.
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &[CENTRAL_BANK.name],
    banks: &["bank"],
    treasuries: &[TREASURY.name],
    depositors: &["firm", "bank", phx_core::ESTATE_KIND.name],
    requesters: &["CB"],
};

/// The central bank's claim on the treasury that backs its liabilities until the sovereign's debt is issued as
/// securities it holds.
const CLAIM: LineKindDecl = LineKindDecl {
    name: "central bank credit to the treasury",
    asset: SideDecl {
        holder_kinds: &[CENTRAL_BANK.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &[TREASURY.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    dated: false,
    transfer_requesters: &["CB"],
};

/// A standing facility's overnight positions: a bank's deposit at the central bank, or the central bank's loan to a
/// bank, one row for each bank that has used it.
const fn facility(
    name: &'static str,
    lender: &'static [&'static str],
    borrower: &'static [&'static str],
) -> LineKindDecl {
    LineKindDecl { name, asset: side(lender), liability: side(borrower), dated: false, transfer_requesters: &["CB"] }
}

/// A facility's side: one row for each party that has used it.
const fn side(kinds: &'static [&'static str]) -> SideDecl {
    SideDecl { holder_kinds: kinds, words: BALANCE, holder_list: true, holder_roles: &[], exclusive: false, many: true }
}

/// The deposit facility: banks' overnight deposits at the central bank.
pub(crate) const DEPOSIT_FACILITY: LineKindDecl = facility("deposit facility", &["bank"], &[CENTRAL_BANK.name]);
/// The lending facility: the central bank's overnight loans to banks.
pub(crate) const LENDING_FACILITY: LineKindDecl = facility("lending facility", &[CENTRAL_BANK.name], &["bank"]);

/// What the central bank's opening instructions are for: capital on both sides, since they open its books.
const REASON: ReasonDecl = ReasonDecl {
    name: "CB opening",
    order: 0,
    paid: Effect::Equity,
    received: Effect::Equity,
    held: phx_num::Missing::Absent,
};

fn reason(b: &Books) -> ReasonId {
    b.ledger.reasons.named(REASON.name)
}

/// The central bank's declarations in the books: its opening's reason and its money lines' kinds.
#[clause("MON.1", "MON.2")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "central bank declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let b = books::of(opening);
        let _ = b.ledger.reasons.declare(REASON);
        b.ledger.lines.declare_reserves(HOLDERS.reserves());
        b.ledger.lines.declare_means_of_payment(HOLDERS.treasury_account());
        b.ledger.lines.declare_money(CLAIM);
        b.ledger.lines.declare_money(DEPOSIT_FACILITY);
        b.ledger.lines.declare_money(LENDING_FACILITY);
        for r in [crate::MOVED, crate::INTEREST, crate::REMITTED] {
            let _ = b.ledger.reasons.declare(r);
        }
    }
}

fn one(b: &Books, name: &str, country: CountryId) -> PartyId {
    let Some(&[(p, _)]) = b.drawn.get(&key(name, country)).map(Vec::as_slice) else {
        violation!(clause = "GEN.3", "the opening reading a party it has not begun", country = country.get());
    };
    p
}

fn banks(b: &Books, country: CountryId) -> Vec<(PartyId, u64)> {
    let Some(v) = b.drawn.get(&key(BANKS, country)) else {
        violation!(clause = "GEN.3", "the central bank opening before the country's banks", country = country.get());
    };
    v.clone()
}

/// The site a country's central bank and treasury share, drawn among its land.
pub fn site(ctx: &phx_core::OpeningCtx<'_>, c: &phx_core::OpeningCountry) -> phx_id::TileId {
    let mut draws = ctx.draws(&OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 0));
    c.site(&mut draws)
}

/// Each country's central bank and its treasury, sited together in the country.
#[clause("CB.1", "PTY.9")]
#[derive(Debug)]
pub struct Parties;

impl Contribution for Parties {
    fn name(&self) -> &'static str {
        "central banks and treasuries"
    }
    fn phase(&self) -> OpeningPhase {
        PARTIES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[CENTRAL, TREASURIES]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[CENTRAL, TREASURIES]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (countries, day) = (opening.countries, opening.day);
        for c in countries {
            let site = site(&opening.ctx, c);
            let b = books::of(opening);
            let cb = b.parties.begin(CENTRAL_BANK.name, site, day);
            let treasury = b.parties.begin(TREASURY.name, site, day);
            b.drawn.insert(key(CENTRAL, c.id), vec![(cb, 1)]);
            b.drawn.insert(key(TREASURIES, c.id), vec![(treasury, 1)]);
        }
    }
}

/// A reserve account's terms: a bank may overdraw it intraday by as much as every bank's reserves at the opening,
/// standing for intraday credit given freely against collateral, which the lending facility's rate prices where it
/// stays overnight. A placeholder naming CB until its operations are built.
fn intraday(register: &phx_core::Register, c: &phx_core::OpeningCountry, dates: ScheduleDates) -> Terms {
    let ccy = currency(c.id);
    let bank_assets = derived(c, "GEN.bank_assets") / PERCENT * c.gdp;
    let limit = whole(derived(c, "GEN.liquid_reserves") / PERCENT * bank_assets);
    let Ok(corridor) = crate::central::corridor(register, c) else {
        violation!(clause = "CB.7", "a corridor that does not compile", country = c.id.get());
    };
    let raw = whole(corridor.lending_rate * crate::consts::RATE_ONE);
    let mut t = Terms::account(ccy, dates);
    t.facility = phx_num::Missing::Present(phx_ledger::algebra::Facility {
        limit: phx_num::Money::new(limit, ccy),
        rate: phx_num::Rate::new(raw, phx_num::RatePeriod::Year),
        day_count: phx_core::calendar::daycount::DayCount::Act365F,
    });
    t
}

/// The central bank's lines in each country: reserves, with a row for each bank; the treasury's account; the claim
/// on the treasury.
#[clause("MON.1", "MON.2", "CB.1")]
#[derive(Debug)]
pub struct Lines;

impl Contribution for Lines {
    fn name(&self) -> &'static str {
        "central bank lines"
    }
    fn phase(&self) -> OpeningPhase {
        CONTRACTS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[CENTRAL, TREASURIES, BANKS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &["CB.lines"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["CB.lines"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (countries, register, date) = (opening.countries, opening.register, opening.date);
        let (b, report) = books::split(opening);
        let reason = reason(b);
        let reserves = b.ledger.lines.kind_index(HOLDERS.reserves().name);
        let account = b.ledger.lines.kind_index(HOLDERS.treasury_account().name);
        let claim = b.ledger.lines.kind_index(CLAIM.name);
        let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
        for c in countries {
            let (cb, treasury) = (one(b, CENTRAL, c.id), one(b, TREASURIES, c.id));
            let banks = banks(b, c.id);
            let dates = ScheduleDates {
                anchor: date,
                period: months,
                eom: EndOfMonth::Plain,
                convention: BusinessDayConvention::Following,
                country: c.id,
            };
            let terms = b.ledger.terms.intern(Terms::account(currency(c.id), dates));
            let reserve_terms = b.ledger.terms.intern(intraday(register, c, dates));
            let held = b.ledger.lines.open(reserves, reserve_terms, phx_num::Missing::Absent);
            let [kept, owed] = [account, claim].map(|kind| b.ledger.lines.open(kind, terms, phx_num::Missing::Absent));
            let Ok(count) = u32::try_from(banks.len()) else {
                phx_num::capacity_exceeded!("banks of a country", u32::MAX, banks.len());
            };
            let mut legs = vec![open_row(register, cb, held, Side::Liability, count, BALANCE)];
            legs.extend(banks.iter().map(|(bank, _)| open_row(register, *bank, held, Side::Asset, 1, BALANCE)));
            legs.push(open_row(register, cb, kept, Side::Liability, 1, BALANCE));
            legs.push(open_row(register, treasury, kept, Side::Asset, 1, BALANCE));
            legs.push(open_row(register, cb, owed, Side::Asset, 1, BALANCE));
            legs.push(open_row(register, treasury, owed, Side::Liability, 1, BALANCE));
            b.open(reason, legs, u64::from(c.id.get()), report);
        }
    }
}

/// The central bank's balance sheet in each country: the reserves each bank holds, the liquid part of its assets;
/// the claim on the treasury, the central bank's assets; and the treasury's account, what of the claim the reserves
/// and the banknotes the households hold do not take. Where those exceed the central bank's drawn assets, the claim
/// rises to them, the one change the accounts need, and is reported. Its books close with no equity of its own; the treasury's through its opening
/// equity.
#[clause("GEN.4", "MON.7", "CB.1")]
#[derive(Debug)]
pub struct Balances;

impl Contribution for Balances {
    fn name(&self) -> &'static str {
        "central bank balances"
    }
    fn phase(&self) -> OpeningPhase {
        BALANCES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[CENTRAL, TREASURIES, BANKS, "CB.lines"]
    }
    fn writes(&self) -> &'static [&'static str] {
        &["CB.balances"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["CB.balances"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let countries = opening.countries;
        for c in countries {
            let gdp = c.gdp;
            let bank_assets = derived(c, "GEN.bank_assets") / PERCENT * gdp;
            let reserves_total = whole(derived(c, "GEN.liquid_reserves") / PERCENT * bank_assets);
            let drawn_claim = whole(derived(c, "GEN.central_bank_assets") / PERCENT * gdp);
            let mut lot = opening.ctx.draws(&OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 1));
            let (b, report) = books::split(opening);
            let reason = reason(b);
            let (cb, treasury) = (one(b, CENTRAL, c.id), one(b, TREASURIES, c.id));
            let banks = banks(b, c.id);
            let weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
            let Ok(total) = u64::try_from(reserves_total) else {
                violation!(clause = "GEN.4", "reserves below nothing", country = c.id.get());
            };
            let parts = apportion(total, &weights, &mut lot);
            let ccy = currency(c.id);
            for ((bank, _), part) in banks.iter().zip(&parts) {
                let Some((line, _)) = b.row_on(*bank, "reserves") else {
                    violation!(clause = "GEN.4", "a bank with no reserve account", bank = bank.get());
                };
                let Ok(r) = i64::try_from(*part) else {
                    violation!(clause = "MON.16", "a bank's reserves beyond whole smallest units", bank = bank.get());
                };
                let legs = vec![
                    write(*bank, line, Side::Asset, r, ccy, bank.get()),
                    write(cb, line, Side::Liability, -r, ccy, bank.get()),
                ];
                b.open(reason, legs, bank.get(), report);
            }
            // The banknotes the households that bank nowhere hold, already written, are its liability too.
            let notes = match b.row_on(cb, "banknotes").and_then(|(line, _)| {
                phx_ledger::rows::find(
                    b.parties.holder(b.parties.row(cb).0),
                    b.parties.row(cb).1,
                    line,
                    Side::Liability,
                )
            }) {
                Some(v) => match v.optional.balance {
                    phx_num::Missing::Present(balance) => -balance,
                    phx_num::Missing::Absent => {
                        violation!(clause = "MON.4", "a central bank's notes with no balance", country = c.id.get())
                    }
                },
                // A country none of whose households banks nowhere has no notes out.
                None => 0,
            };
            let owed = reserves_total + notes;
            let claim = if drawn_claim < owed {
                report.adjustments.push(Adjustment {
                    what: format!(
                        "country {}: the central bank's claim on the treasury, raised to its reserves and notes",
                        c.id.get()
                    ),
                    drawn: i128::from(drawn_claim),
                    set: i128::from(owed),
                });
                owed
            } else {
                drawn_claim
            };
            let account = claim - owed;
            let (Some((k, _)), Some((a, _))) =
                (b.row_on(treasury, CLAIM.name), b.row_on(treasury, HOLDERS.treasury_account().name))
            else {
                violation!(clause = "GEN.4", "a treasury with no account or no claim", country = c.id.get());
            };
            let id = treasury.get();
            let legs = vec![
                write(cb, k, Side::Asset, claim, ccy, id),
                write(treasury, k, Side::Liability, -claim, ccy, id),
                write(treasury, a, Side::Asset, account, ccy, id),
                write(cb, a, Side::Liability, -account, ccy, id),
            ];
            b.open(reason, legs, id, report);
        }
    }
}
