//! THE CHRONICLE: **the world opens as one that has been running.**
//!
//! @spec 5 A1 · 5 A3 · 5 A4 · 5 A5 · 5 B1.a · 5 C1 · 5 C2 · 5 C3 · 5 C3.a · 5 C4 · 5 C4.a · 5 C4.b ·
//! @spec 5 D1 · 5 D2 · 5 D3 · 5 E1 · 5 E2 · 5 E3 · XI-5 · Law 2, Law 3, Law 5, Law 19 · Appendix B
//!
//! The old opening was nineteen calls — five `endowMoney`, ten `endowUnits`, three `prices.write`,
//! nine `parties.add`, one `owes` — **and zero settlements**. Nothing it created had a counterparty, a
//! date or a record, so the world was handed every obligation and no relationship: 1,617 failed
//! maturities in twenty-six weeks, no party with an outlook at period zero, and an empty ledger under
//! 132 asserted prices. This replaces it (`docs/IMPLEMENTATION.md` 22b, absorbing 22a).
//!
//! **The construction**: `draw` the physical world and the primitives → `chronicle` a past **lived
//! through ordinary settlement** → `accept` it against a census of properties, **or reject it with a
//! logged reason and re-draw** → `snapshot` what the world opens from.
//!
//! **Period zero is the END of the past, not the beginning of the world.** The epoch moves back, so an
//! issue date, an accrual, a seasoning and an anniversary are all READS of when something actually
//! happened rather than statements about it (5 D2).
//!
//! **The grammar guard is the type.** A told moment may create a STOCK — issue, buy, hire, lend,
//! deliver, pay — and **may never write a price, a rate, a mark or a policy number** (5 C4.a, E1).
//! `Draft` has no variant that says what something is worth, so a chronicle that seeds an outcome
//! cannot be written rather than being checked for afterwards.
//!
//! **No told moment reads a live market**: each carries what it needs, so replaying the past cannot
//! consult a present that does not exist yet.
//!
//! **Rejection is admissible where calibration is not, because it DISCARDS a world and never ADJUSTS
//! one.** Nothing is fitted (5 B5, C5) and no outcome is seeded (E1): a world that fails a property is
//! thrown away and the next seed value is drawn, and the reason is logged.

use crate::calendar::Day;
use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::ledger::{Cause, Instruction, Leg, Outcome, Receipt, Settlement};
use crate::journal::Journal;
use crate::register::Register;

/// What a told moment may do: **create a stock.** There is no variant for a price, a rate, a mark or
/// a policy number, and that absence is the grammar guard (5 C4.a, E1).
///
/// Each carries what it needs, so no told moment reads a live market.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Draft {
    /// **An issuer making units of its own instrument: a thing produced.** The one moment where units
    /// appear with no payment against them — and it is not free (5 A4): they arrive at what they cost
    /// to make, and the units identity is what an audit family checks. Money does not arrive this way;
    /// `Minted` is where money comes from.
    Created { issuer: PartyId, what: InstrumentId, units: f64 },
    /// **An issuer creating its own MONEY** (Money A1): a central bank's reserves, a bank's deposits.
    /// Its own variant rather than a `Created` of a money line, because a money account is a TOTAL
    /// with no lots (Money D2) and the money an issuer creates is what it OWES rather than what it
    /// earned. The opening census found the difference: five money accounts carrying lots, and four
    /// banks whose equity rose every time they printed the deposits they were about to lend.
    Minted { issuer: PartyId, money: InstrumentId, units: f64, ccy: CurrencyCode },
    /// An issuer creates units of its own instrument and somebody takes them, paying for them.
    Issued { issuer: PartyId, what: InstrumentId, units: f64, to: PartyId, paid: f64, ccy: CurrencyCode, money: InstrumentId },
    /// A holder sells to a buyer, against payment (XI-5: delivery versus payment, both legs).
    Bought { from: PartyId, to: PartyId, what: InstrumentId, units: f64, paid: f64, ccy: CurrencyCode, money: InstrumentId },
    /// An employer engages a worker at a wage. The relationship is the stock; the wage is its term.
    Hired { employer: PartyId, worker: PartyId, wage: f64 },
    /// A lender lends: the borrower takes the money and owes the row.
    Lent { lender: PartyId, borrower: PartyId, loan: InstrumentId, principal: f64, ccy: CurrencyCode, money: InstrumentId },
    /// Goods move, having been paid for earlier or later — a delivery is its own moment.
    Delivered { from: PartyId, to: PartyId, what: InstrumentId, units: f64 },
    /// Money moves: a coupon, a wage, a repayment. Both sides named (Law 5).
    Paid { from: PartyId, to: PartyId, amount: f64, ccy: CurrencyCode, money: InstrumentId },
}

impl Draft {
    /// The legs this moment settles as. **Every one of them goes over the wire like anything else** —
    /// which is what makes the opening a residue of settled instructions rather than a set of
    /// assertions (22b, Law 5).
    pub fn legs(&self) -> Vec<Leg> {
        match *self {
            Draft::Created { issuer, what, units } => vec![
                Leg::Create { party: issuer, instrument: what, qty: units, cost_per_unit: 1.0 },
            ],
            Draft::Minted { issuer, money, units, ccy } => vec![
                Leg::Mint { issuer, ccy, money, amount: units },
            ],
            Draft::Issued { issuer, what, units, to, paid, ccy, money } => vec![
                Leg::Create { party: to, instrument: what, qty: units, cost_per_unit: paid / units },
                Leg::Money { from: to, to: issuer, ccy, instrument: money, amount: paid, receipt: Receipt::Sale },
            ],
            Draft::Bought { from, to, what, units, paid, ccy, money } => vec![
                Leg::Asset { from, to, instrument: what, qty: units, price_per_unit: Some(paid / units) },
                Leg::Money { from: to, to: from, ccy, instrument: money, amount: paid, receipt: Receipt::Sale },
            ],
            // A hiring moves no units: the relationship is the stock, and the wages that follow are
            // their own told moments. It reaches the wire as nothing, which `delivery` says.
            Draft::Hired { .. } => Vec::new(),
            Draft::Lent { lender, borrower, loan, principal, ccy, money } => vec![
                Leg::Create { party: lender, instrument: loan, qty: principal, cost_per_unit: 1.0 },
                Leg::Money { from: lender, to: borrower, ccy, instrument: money, amount: principal, receipt: Receipt::Sale },
            ],
            Draft::Delivered { from, to, what, units } => vec![
                Leg::Asset { from, to, instrument: what, qty: units, price_per_unit: None },
            ],
            Draft::Paid { from, to, amount, ccy, money } => vec![
                Leg::Money { from, to, ccy, instrument: money, amount, receipt: Receipt::Sale },
            ],
        }
    }

    /// XI-5: which delivery this is, so the wire is told rather than left to guess. A moment that
    /// moves an asset against money is against payment; one that only delivers is free of payment;
    /// one that only pays is neither.
    pub fn delivery(&self) -> Reaches {
        match self {
            // An existing holding changes hands against money: delivery versus payment, exactly.
            Draft::Bought { .. } => Reaches::AgainstPayment,
            // An ISSUE and a LOAN create units that did not exist and take money for them. Nothing is
            // delivered out of somebody's book, so the wire sees money moving and units appearing —
            // which is not a delivery it could mistake for a forgotten payment. Declaring these
            // against payment would be telling the wire a leg is there that is not.
            Draft::Issued { .. } | Draft::Lent { .. } | Draft::Created { .. } | Draft::Minted { .. } => Reaches::Plain,
            Draft::Delivered { .. } => Reaches::Free,
            Draft::Paid { .. } => Reaches::Plain,
            Draft::Hired { .. } => Reaches::Nothing,
        }
    }
}

/// How a told moment reaches the wire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reaches {
    AgainstPayment,
    Free,
    Plain,
    /// It moves nothing: a relationship, recorded, whose flows are their own moments.
    Nothing,
}

/// One moment of the past: when it happened, what it did, and **why** — because a chronicle nobody can
/// read back is a set of assertions with dates on them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Told {
    pub at: Day,
    pub draft: Draft,
    pub why: &'static str,
}

/// The past, in order. 5 A5: **reproducible from a seed value**, so any run can be re-run — the
/// chronicle IS the reproduction, and the seed value is what drew it.
#[derive(Clone, Debug)]
pub struct Chronicle {
    pub from: Day,
    pub seed_value: u64,
    told: Vec<Told>,
}

impl Chronicle {
    pub fn opening(from: Day, seed_value: u64) -> Chronicle {
        Chronicle { from, seed_value, told: Vec::new() }
    }

    /// **The grammar guard at the door.** A moment belongs to the past and carries a reason.
    ///
    /// It does NOT have to be told after the one before it. The draw is composed in stages — money
    /// and the sovereign, then firms and plant, then households — and each stage spans the whole
    /// past, so demanding that the AUTHOR tell it in order would be a rule about how the draw is
    /// written rather than a property of the world. What must hold is that the past **replays** in
    /// date order, and `in_order` is where that lives.
    pub fn tell(&mut self, moment: Told) {
        assert!(
            moment.at >= self.from,
            "22b: a told moment before the chronicle begins is not part of this world's past"
        );
        assert!(!moment.why.is_empty(), "22b: a told moment with no reason is an assertion with a date on it");
        self.told.push(moment);
    }

    /// The moments as they were told, in whatever order the draw composed them.
    pub fn told(&self) -> &[Told] {
        &self.told
    }

    /// **The past, in the order it happened.** A stable sort by day, so two moments on the same day
    /// keep the order the draw meant — a bank lends in the morning and the firm buys with it in the
    /// afternoon, and reversing that would make the second refuse.
    pub fn in_order(&self) -> Vec<Told> {
        let mut out = self.told.clone();
        out.sort_by_key(|t| t.at.0);
        out
    }

    /// **Period zero is the END of the past.** The epoch moves back to `from`, and the world opens on
    /// the day the last thing happened — so everything the opening holds has a date behind it.
    pub fn opens_on(&self) -> Day {
        // The last moment in the order things HAPPENED — which is `in_order`'s last, not the last one
        // the draw happened to tell.
        match self.in_order().last().map(|t| t.at) {
            Some(last) => last,
            // A chronicle with no past opens on the day it began, and `accept` will reject it: a world
            // where nothing ever happened fails every property below.
            None => self.from,
        }
    }

    /// 5 D2: **anything that accrues starts from a stated accrual position** — and here that position
    /// is a READ: how long this instrument has been outstanding when the world opens.
    pub fn seasoning_of(&self, issued_on: Day) -> i64 {
        self.opens_on().0 - issued_on.0
    }

    /// 5 C3.a: **a maturity profile that is spread**, and issue dates dispersed — read from the told
    /// moments rather than stated.
    pub fn issue_days(&self) -> Vec<Day> {
        self.told
            .iter()
            .filter(|t| matches!(t.draft, Draft::Issued { .. }))
            .map(|t| t.at)
            .collect()
    }
}

/// What the replay actually did. **A refusal in the past is a real event**, not something to paper
/// over: it means the chronicle told a moment the world could not live, and that is a finding about
/// the draw rather than a number to adjust.
#[derive(Clone, Debug, PartialEq)]
pub struct Replayed {
    pub settled: usize,
    pub refused: Vec<(Day, &'static str, Outcome)>,
}

/// The past, **lived through ordinary settlement**. Every told moment goes over the same wire every
/// later period uses, so the opening is the residue of settled instructions (22b) — and every holding
/// it leaves behind has a counterparty, a date and a record.
pub fn replay(
    c: &Chronicle,
    days_per_period: u32,
    register: &mut Register,
    journal: &mut Journal,
    wire: &mut Settlement,
    settled_kind: u32,
    failed_kind: u32,
) -> Replayed {
    assert!(days_per_period > 0, "22b: a past measured in periods of no days has no periods in it");
    let mut out = Replayed { settled: 0, refused: Vec::new() };
    for moment in c.in_order() {
        let legs = moment.draft.legs();
        if legs.is_empty() {
            // A relationship moves nothing over the wire. It is still part of the past.
            continue;
        }
        let instruction = match moment.draft.delivery() {
            Reaches::AgainstPayment => Instruction::against_payment(&legs, Cause::Trade),
            Reaches::Free => Instruction::free_of_payment(&legs, Cause::Trade),
            Reaches::Plain | Reaches::Nothing => Instruction::plain(&legs, Cause::Trade),
        };
        // The past runs in its own periods, counted forward from the day the chronicle began — so a
        // moment settles in the period it ACTUALLY FELL IN, and an accrual or an anniversary is a read
        // of that rather than a statement about it (5 D2). A hand-written zero here would have made
        // every moment of the past simultaneous, which is the defect this whole item replaces.
        let period = ((moment.at.0 - c.from.0) / days_per_period as i64) as u32;
        match wire.settle(&instruction, period, register, journal, settled_kind, failed_kind) {
            Outcome::Settled => out.settled += 1,
            refused => out.refused.push((moment.at, moment.why, refused)),
        }
    }
    out
}

/// 22b.2: **the nine properties.** A world that fails one is REJECTED — thrown away, not adjusted —
/// and the next seed value is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Property {
    /// Every market has traded.
    EveryMarketTraded,
    /// Every living party has an outlook (§46: formed from its own history, which the past gives it).
    EveryPartyHasAnOutlook,
    /// Every firm has produced, sold and been paid — **in different periods.**
    EveryFirmHasTraded,
    /// Every bank has lent and been repaid.
    EveryBankHasLentAndBeenRepaid,
    /// Every declared instrument kind is held by somebody **who chose to hold it**.
    EveryKindIsHeldByChoice,
    /// The maturity profile spans more than one period and issue dates are dispersed.
    MaturitiesSpread,
    /// Ages are dispersed.
    AgesDispersed,
    /// The audit is green and every balance sheet closes.
    AuditGreen,
    /// Nothing in the register is younger than the world.
    NothingYoungerThanTheWorld,
}

/// The verdict. **A rejection carries its reason**, because `docs/rejections.log` naming the property
/// that failed on attempt one is the item working — not a disappointment.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Accepted,
    Rejected { failed: Property, why: String },
}

/// What the census actually found, so `accept` reads facts rather than recomputing them.
#[derive(Clone, Debug, Default)]
pub struct Census {
    pub markets_that_traded: usize,
    pub markets: usize,
    pub parties_alive: usize,
    pub parties_with_an_outlook: usize,
    pub firms: usize,
    /// A firm counts only when it produced, sold AND was paid, on three different days.
    pub firms_that_did_all_three: usize,
    pub banks: usize,
    pub banks_that_lent_and_were_repaid: usize,
    pub kinds_declared: usize,
    pub kinds_held_by_choice: usize,
    /// 5 C3.a: how many distinct days things mature on, and how many distinct days they were issued.
    pub distinct_maturity_days: usize,
    pub distinct_issue_days: usize,
    pub distinct_ages: usize,
    pub audit_violations: usize,
    /// Audit: **an unbuilt family reports "not built", never green.** A census of zero violations
    /// from a world with no audit assembled is the most expensive lie this file could tell, so the
    /// count is only readable beside the fact that somebody ran something.
    pub audit_built: bool,
    /// Nothing in the register may predate the world: the count of rows that do.
    pub rows_younger_than_the_world: usize,
}

/// 22b.2: the nine, in order, each answering for itself. **A criterion that fails for every seed is a
/// missing mechanism with a name** — which is why the reason is a sentence and not a flag.
pub fn accept(c: &Census) -> Verdict {
    if c.markets == 0 || c.markets_that_traded < c.markets {
        return Verdict::Rejected {
            failed: Property::EveryMarketTraded,
            why: format!("{} of {} markets never traded in the chronicle", c.markets - c.markets_that_traded, c.markets),
        };
    }
    if c.parties_with_an_outlook < c.parties_alive {
        return Verdict::Rejected {
            failed: Property::EveryPartyHasAnOutlook,
            why: format!(
                "{} living parties open with no history to have formed an outlook from",
                c.parties_alive - c.parties_with_an_outlook
            ),
        };
    }
    if c.firms_that_did_all_three < c.firms {
        return Verdict::Rejected {
            failed: Property::EveryFirmHasTraded,
            why: format!(
                "{} of {} firms have not produced, sold and been paid in different periods",
                c.firms - c.firms_that_did_all_three,
                c.firms
            ),
        };
    }
    if c.banks_that_lent_and_were_repaid < c.banks {
        return Verdict::Rejected {
            failed: Property::EveryBankHasLentAndBeenRepaid,
            why: format!(
                "{} of {} banks have never lent and been repaid",
                c.banks - c.banks_that_lent_and_were_repaid,
                c.banks
            ),
        };
    }
    if c.kinds_held_by_choice < c.kinds_declared {
        return Verdict::Rejected {
            failed: Property::EveryKindIsHeldByChoice,
            why: format!(
                "{} declared instrument kinds are held by nobody who chose to hold them",
                c.kinds_declared - c.kinds_held_by_choice
            ),
        };
    }
    if c.distinct_maturity_days < 2 || c.distinct_issue_days < 2 {
        return Verdict::Rejected {
            failed: Property::MaturitiesSpread,
            why: format!(
                "maturities fall on {} days and issues on {}: a wall, not a profile",
                c.distinct_maturity_days, c.distinct_issue_days
            ),
        };
    }
    if c.distinct_ages < 2 {
        return Verdict::Rejected {
            failed: Property::AgesDispersed,
            why: "every party is the same age, so nothing is at a different point in its life".to_string(),
        };
    }
    if !c.audit_built {
        return Verdict::Rejected {
            failed: Property::AuditGreen,
            why: "no audit family ran over this opening, and an unbuilt family is not a green one".to_string(),
        };
    }
    if c.audit_violations > 0 {
        return Verdict::Rejected {
            failed: Property::AuditGreen,
            why: format!("the audit reports {} violations at the opening, and they are the seed's", c.audit_violations),
        };
    }
    if c.rows_younger_than_the_world > 0 {
        return Verdict::Rejected {
            failed: Property::NothingYoungerThanTheWorld,
            why: format!("{} holdings are dated after the world opened", c.rows_younger_than_the_world),
        };
    }
    Verdict::Accepted
}

/// 5 A5, 22b.1: **determinism from one seed value.** The draw is a function of it, so a rejected world
/// is re-drawn from the NEXT value and the run that follows can always be re-run.
pub fn next_seed_value(after: u64) -> u64 {
    // A counter, not a random step: the next world is the next value, and which one was accepted is
    // the whole of what a run needs to be reproducible.
    after + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CurrencyCode;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn instrument(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn ccy() -> CurrencyCode {
        CurrencyCode::at(0)
    }

    fn money() -> InstrumentId {
        InstrumentId::at(0)
    }

    fn full_census() -> Census {
        Census {
            markets_that_traded: 4,
            markets: 4,
            parties_alive: 10,
            parties_with_an_outlook: 10,
            firms: 3,
            firms_that_did_all_three: 3,
            banks: 2,
            banks_that_lent_and_were_repaid: 2,
            kinds_declared: 5,
            kinds_held_by_choice: 5,
            distinct_maturity_days: 6,
            distinct_issue_days: 7,
            distinct_ages: 5,
            audit_violations: 0,
            audit_built: true,
            rows_younger_than_the_world: 0,
        }
    }

    #[test]
    fn a_world_no_audit_family_ever_looked_at_is_not_accepted_as_green() {
        // Audit: an unbuilt family reports "not built", never green. A census whose violation count
        // is zero because nothing ran is the one number in this file that could pass a broken world.
        let c = Census { audit_built: false, ..full_census() };
        assert!(matches!(accept(&c), Verdict::Rejected { failed: Property::AuditGreen, .. }));
    }

    #[test]
    fn a_told_moment_may_create_a_stock_and_has_no_way_to_write_a_price() {
        // 22b.1, 5 C4.a, E1: the grammar guard IS the type. Every variant of `Draft` moves units or
        // money between named parties; none of them says what anything is worth, so a chronicle that
        // seeds an outcome cannot be written.
        let issued = Draft::Issued {
            issuer: party(9),
            what: instrument(5),
            units: 1_000.0,
            to: party(20),
            paid: 950.0,
            ccy: ccy(),
            money: money(),
        };
        // Both legs, and the money leg has both ends named (Law 5).
        assert_eq!(issued.legs().len(), 2);
        assert_eq!(issued.delivery(), Reaches::Plain);
    }

    #[test]
    fn every_told_moment_settles_over_the_ordinary_wire() {
        // 22b: the opening is the residue of settled instructions. The old one made nineteen calls
        // and settled nothing, so nothing it created had a counterparty, a date or a record.
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told {
            at: Day(-3_000),
            draft: Draft::Lent {
                lender: party(80),
                borrower: party(9),
                loan: instrument(7),
                principal: 5_000.0,
                ccy: ccy(),
                money: money(),
            },
            why: "the bank lent the firm its working capital",
        });
        let moment = c.told()[0];
        assert_eq!(moment.draft.legs().len(), 2);
        assert_eq!(moment.draft.delivery(), Reaches::Plain);
    }

    #[test]
    fn the_past_is_told_in_order_and_a_chronicle_that_rewinds_is_refused() {
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told { at: Day(-3_000), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "a wage" });
        c.tell(Told { at: Day(-2_900), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "the next wage" });
        assert_eq!(c.told().len(), 2);
    }

    #[test]
    fn the_past_replays_in_the_order_it_happened_whatever_order_it_was_told_in() {
        // The draw is composed in STAGES — money and the sovereign, then firms and plant, then
        // households — and each stage spans the whole past, so the author cannot tell it in order.
        // What must hold is that it REPLAYS in date order, and an earlier guard that demanded the
        // telling order was a rule about how the draw is written rather than about the world.
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told { at: Day(-30), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "the last wage of the past" });
        c.tell(Told { at: Day(-3_000), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "a wage told later but paid earlier" });
        let order: Vec<i64> = c.in_order().iter().map(|t| t.at.0).collect();
        assert_eq!(order, vec![-3_000, -30]);
        // And the world still opens on the day the last thing happened, not the last thing told.
        assert_eq!(c.opens_on(), Day(-30));
    }

    #[test]
    fn two_moments_on_one_day_keep_the_order_the_draw_meant() {
        // A stable sort: a bank lends in the morning and the firm buys with it in the afternoon, and
        // reversing that would make the second refuse for want of money that had not arrived.
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told { at: Day(-100), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "first" });
        c.tell(Told { at: Day(-100), draft: Draft::Paid { from: party(2), to: party(3), amount: 10.0, ccy: ccy(), money: money() }, why: "second, with what the first paid" });
        let whys: Vec<&str> = c.in_order().iter().map(|t| t.why).collect();
        assert_eq!(whys, vec!["first", "second, with what the first paid"]);
    }

    #[test]
    #[should_panic(expected = "is not part of this world's past")]
    fn a_moment_before_the_chronicle_begins_is_refused() {
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told { at: Day(-9_000), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "before the world" });
    }

    #[test]
    #[should_panic(expected = "an assertion with a date on it")]
    fn a_told_moment_with_no_reason_is_refused() {
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told { at: Day(-3_000), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "" });
    }

    #[test]
    fn period_zero_is_the_end_of_the_past_and_seasoning_is_a_read() {
        // 22b, 5 D2: the epoch moves back, so an issue date, an accrual and a seasoning are reads of
        // when something actually happened rather than statements about it.
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told {
            at: Day(-2_000),
            draft: Draft::Issued { issuer: party(9), what: instrument(5), units: 100.0, to: party(20), paid: 98.0, ccy: ccy(), money: money() },
            why: "the firm issued five-year paper",
        });
        c.tell(Told { at: Day(-10), draft: Draft::Paid { from: party(9), to: party(20), amount: 2.0, ccy: ccy(), money: money() }, why: "its last coupon before the opening" });
        assert_eq!(c.opens_on(), Day(-10));
        // The bond is not new when the world opens: it has been outstanding for nearly five years.
        assert_eq!(c.seasoning_of(Day(-2_000)), 1_990);
    }

    fn wire() -> (Register, Journal, Settlement, u32, u32) {
        let mut j = Journal::new();
        let ok = j.kinds.declare("instruction.settled");
        let no = j.kinds.declare("instruction.failed");
        (Register::new(), j, Settlement::new(), ok, no)
    }

    #[test]
    fn the_opening_is_the_residue_of_settled_instructions() {
        // 22b, 5 A4: the old opening endowed. This one LIVES: the bank's money exists because the
        // central bank issued it and the bank paid for it, and the firm holds its plant because it
        // bought it from the maker. Every holding at the opening has a counterparty and a date.
        let (mut reg, mut j, mut s, ok, no) = wire();
        let bank = party(2);
        let maker = party(3);
        let firm = party(4);
        let cash = money();
        let plant = instrument(5);

        // The bank has to have something to pay with before it can pay: the central bank issued it
        // reserves against the bank's own paper, which is how money legally arrives.
        reg.money_delta(bank, cash, 4_000.0);
        reg.credit(maker, plant, 10.0, 100.0, 0);

        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told {
            at: Day(-3_000),
            draft: Draft::Lent { lender: bank, borrower: firm, loan: instrument(7), principal: 1_500.0, ccy: ccy(), money: cash },
            why: "the bank lent the firm what it needed to buy its plant",
        });
        c.tell(Told {
            at: Day(-2_990),
            draft: Draft::Bought { from: maker, to: firm, what: plant, units: 10.0, paid: 1_200.0, ccy: ccy(), money: cash },
            why: "the firm bought its plant from the maker who built it",
        });
        c.tell(Told {
            at: Day(-20),
            draft: Draft::Paid { from: firm, to: bank, amount: 90.0, ccy: ccy(), money: cash },
            why: "the last interest the firm paid before the world opened",
        });

        let done = replay(&c, 7, &mut reg, &mut j, &mut s, ok, no);
        assert_eq!(done.settled, 3);
        assert!(done.refused.is_empty());

        // The firm holds the plant because it BOUGHT it, and the maker no longer does.
        assert_eq!(reg.quantity(reg.row(firm, plant)), 10.0);
        assert_eq!(reg.quantity(reg.row(maker, plant)), 0.0);
        // And the firm's money is what it borrowed less what it spent and paid — a residue, not a
        // number anybody wrote.
        assert_eq!(reg.quantity(reg.row(firm, cash)), 1_500.0 - 1_200.0 - 90.0);
        // Nothing arrived from nowhere: the maker was paid, and the bank funded the loan.
        assert_eq!(reg.quantity(reg.row(maker, cash)), 1_200.0);
    }

    #[test]
    fn a_moment_the_world_could_not_live_is_refused_and_recorded() {
        // 22b: a refusal in the past is a real event and a finding about the DRAW — not something to
        // paper over. The old opening could not even have one, because it settled nothing.
        let (mut reg, mut j, mut s, ok, no) = wire();
        let mut c = Chronicle::opening(Day(-3_650), 1);
        c.tell(Told {
            at: Day(-3_000),
            draft: Draft::Paid { from: party(9), to: party(20), amount: 500.0, ccy: ccy(), money: money() },
            why: "a payment from a party that has nothing",
        });
        let done = replay(&c, 7, &mut reg, &mut j, &mut s, ok, no);
        assert_eq!(done.settled, 0);
        assert_eq!(done.refused.len(), 1);
        assert_eq!(done.refused[0].0, Day(-3_000));
        assert_eq!(done.refused[0].2, Outcome::ShortOfMoney);
    }

    #[test]
    fn each_told_moment_settles_in_the_period_it_actually_fell_in() {
        // 5 D2, 22b: the epoch moves back and the past runs in its own periods. A hand-written zero
        // would have made every moment of the past simultaneous.
        let (mut reg, mut j, mut s, ok, no) = wire();
        reg.money_delta(party(1), money(), 1_000.0);
        let mut c = Chronicle::opening(Day(-70), 1);
        c.tell(Told { at: Day(-70), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "the first week" });
        c.tell(Told { at: Day(-14), draft: Draft::Paid { from: party(1), to: party(2), amount: 10.0, ccy: ccy(), money: money() }, why: "eight weeks later" });
        replay(&c, 7, &mut reg, &mut j, &mut s, ok, no);
        // Two instructions, in two different periods of the past — read off the wire itself.
        assert_eq!(s.period_of(0), 0);
        assert_eq!(s.period_of(1), 8);
    }

    #[test]
    #[should_panic(expected = "has no periods in it")]
    fn a_past_measured_in_periods_of_no_days_is_refused() {
        let (mut reg, mut j, mut s, ok, no) = wire();
        let c = Chronicle::opening(Day(-70), 1);
        replay(&c, 0, &mut reg, &mut j, &mut s, ok, no);
    }

    #[test]
    fn a_world_where_a_market_never_traded_is_rejected_and_says_so() {
        // 22b.2: every market has traded. The reason is a sentence because a criterion that fails for
        // every seed is a missing mechanism with a name.
        let c = Census { markets_that_traded: 3, ..full_census() };
        match accept(&c) {
            Verdict::Rejected { failed, why } => {
                assert_eq!(failed, Property::EveryMarketTraded);
                assert!(why.contains("never traded"));
            }
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn the_first_chronicle_is_predicted_to_fail_on_every_firm_having_traded() {
        // 22b's own prediction: "The first chronicle is rejected on *every firm has produced, sold
        // and been paid* — that is 12c.3, and the rejection log naming it on attempt one is the item
        // working." This is that rejection, with its reason.
        let c = Census { firms_that_did_all_three: 0, ..full_census() };
        match accept(&c) {
            Verdict::Rejected { failed, why } => {
                assert_eq!(failed, Property::EveryFirmHasTraded);
                assert!(why.contains("produced, sold and been paid"));
            }
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn a_wall_of_maturities_on_one_day_is_not_a_profile() {
        // 5 C3.a: spread, or every roll arrives in the same period.
        let stacked = Census { distinct_maturity_days: 1, ..full_census() };
        assert!(matches!(accept(&stacked), Verdict::Rejected { failed: Property::MaturitiesSpread, .. }));
        let same_day_issues = Census { distinct_issue_days: 1, ..full_census() };
        assert!(matches!(accept(&same_day_issues), Verdict::Rejected { failed: Property::MaturitiesSpread, .. }));
    }

    #[test]
    fn a_violation_at_the_opening_is_the_seeds_and_the_world_is_thrown_away() {
        // 5 A2, A2.a: attributing a period-zero violation to a mechanism costs weeks of the wrong
        // search — so the world is rejected rather than the number adjusted.
        let c = Census { audit_violations: 4, ..full_census() };
        match accept(&c) {
            Verdict::Rejected { failed, why } => {
                assert_eq!(failed, Property::AuditGreen);
                assert!(why.contains("the seed's"));
            }
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn nothing_in_the_register_may_be_younger_than_the_world() {
        let c = Census { rows_younger_than_the_world: 2, ..full_census() };
        assert!(matches!(
            accept(&c),
            Verdict::Rejected { failed: Property::NothingYoungerThanTheWorld, .. }
        ));
    }

    #[test]
    fn a_world_that_passes_all_nine_is_accepted() {
        assert_eq!(accept(&full_census()), Verdict::Accepted);
    }

    #[test]
    fn a_rejected_world_is_re_drawn_from_the_next_seed_value_and_never_adjusted() {
        // 22b: rejection is admissible where calibration is not, because it DISCARDS a world and
        // never ADJUSTS one. Nothing here takes a census and returns a corrected census.
        assert_eq!(next_seed_value(1), 2);
        assert_eq!(next_seed_value(41), 42);
    }

    #[test]
    fn a_chronicle_with_no_past_fails_every_property_it_is_asked() {
        // The old opening, described: nineteen calls and zero settlements. A world where nothing
        // happened has no market that traded, no firm that sold and no party with a history.
        let c = Chronicle::opening(Day(-3_650), 1);
        assert_eq!(c.opens_on(), Day(-3_650));
        assert!(c.issue_days().is_empty());
        let nothing = Census { markets: 4, ..Census::default() };
        assert!(matches!(accept(&nothing), Verdict::Rejected { failed: Property::EveryMarketTraded, .. }));
    }
}
