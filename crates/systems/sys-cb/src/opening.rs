use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{
    Adjustment, BALANCES, CONTRACTS, Contribution, Opening, OpeningPhase, PARTIES, StreamDef, apportion,
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
    depositors: &["firm", "bank"],
    requesters: &["CB"],
};

/// The central bank's claim on the treasury that backs its liabilities until the sovereign's debt is issued as
/// securities it holds.
const CLAIM: LineKindDecl = LineKindDecl {
    name: "central bank credit to the treasury",
    asset: SideDecl { holder_kinds: &[CENTRAL_BANK.name], words: BALANCE, holder_list: true },
    liability: SideDecl { holder_kinds: &[TREASURY.name], words: BALANCE, holder_list: true },
    dated: false,
    transfer_requesters: &["CB"],
};

fn reason(b: &mut Books) -> ReasonId {
    b.ledger.reasons.declare(ReasonDecl {
        name: "CB opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    })
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
            let mut draws = opening.ctx.draws(&OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 0));
            let site = c.site(&mut draws);
            let b = books::of(opening);
            let cb = b.parties.begin(CENTRAL_BANK.name, site, day);
            let treasury = b.parties.begin(TREASURY.name, site, day);
            b.drawn.insert(key(CENTRAL, c.id), vec![(cb, 1)]);
            b.drawn.insert(key(TREASURIES, c.id), vec![(treasury, 1)]);
        }
    }
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
        let reserves = b.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
        let account = b.ledger.lines.declare_means_of_payment(HOLDERS.treasury_account()).index();
        let claim = b.ledger.lines.declare_money(CLAIM).index();
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
            let [held, kept, owed] =
                [reserves, account, claim].map(|kind| b.ledger.lines.open(kind, terms, phx_num::Missing::Absent));
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
/// do not take. Where the reserves exceed the central bank's drawn assets, the claim rises to them, the one change
/// the accounts need, and is reported. Its books close with no equity of its own; the treasury's through its opening
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
            let claim = if drawn_claim < reserves_total {
                report.adjustments.push(Adjustment {
                    what: format!(
                        "country {}: the central bank's claim on the treasury, raised to its reserves",
                        c.id.get()
                    ),
                    drawn: i128::from(drawn_claim),
                    set: i128::from(reserves_total),
                });
                reserves_total
            } else {
                drawn_claim
            };
            let account = claim - reserves_total;
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
