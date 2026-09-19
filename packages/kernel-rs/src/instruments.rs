//! THE INSTRUMENTS: every priced thing in the world, with the party that issued it.
//!
//! @spec Money A1 · Money A2.b · Money D2 · Register A1 · 5 A4 · 5 C3 · 5 C3.a · 5 C4.b · Law 2,
//! @spec Law 4, Law 8, Law 9 · Appendix B
//!
//! No money without an issuer, and no holding without an issuer. The register knows
//! who HOLDS what; this knows who OWES it. Without both, a seed can credit a household with a deposit
//! that is nobody's liability — 5 A4's free money — and nothing in the world could say so.
//!
//! Price 1 for money is the only hard-coded price, and it is a fact about the CLASS
//! rather than a price anybody printed.
//!
//! A term is not an opening condition (5 C4.b). A coupon is fixed for the instrument's life and is
//! permanent structure; a price is an opening guess the next period re-clears. They are different
//! fields here for that reason: `coupon` is a term, and a price lives in `Prints` where it can be
//! re-cleared.
//!
//! Instruments are named as a market names them: issuer plus coupon plus maturity for a
//! bond, the issuer alone for a share. The row carries the facts; `Names` turns them into the display,
//! and an internal id is never shown.

use crate::calendar::Day;
use crate::ids::{CurrencyCode, HoldingId, InstrumentId, PartyId, UnitId};
use crate::register::Register;
use crate::stores::Claims;

/// What kind of thing this is. Not a branch a mechanism takes — the kernel uses
/// it to know what may be held in fractions, what carries lots, and what money is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// An account is a holding of money ISSUED BY a bank or the central bank. It carries no
    /// lots, because one unit of it is every other unit.
    Money,
    /// A promise to pay: a bond, a bill, a loan row.
    Claim,
    /// A share in an issuer's residual.
    Share,
    /// A physical good, in its own unit.
    Good,
    /// A productive asset with its own life.
    Plant,
}

impl Class {
    /// Money accounts hold no lots — there is no basis to carry, because every unit is the
    /// same unit. The register's `total_only` column is set from this.
    pub fn carries_lots(&self) -> bool {
        *self != Class::Money
    }
}

/// The issued amount moves only by a NAMED event, and these are the ones this
/// kernel has.
///
/// B1 names five — an issuance, a re-opening, a buyback, an amortisation, a maturity. Two of them
/// exist here and both are legs on the wire: `Leg::Create` and `Leg::Mint` make units,
/// `Leg::Destroy` unmakes them. The other three are not built, and a line that matures keeps every
/// unit it had — which is a fact about this world rather than a
/// vocabulary this enum should pretend to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Issuance {
    /// Register B1, Goods B: units come into existence. A line brought with its first units, a
    /// batch off a production line, an issuer minting its own money.
    Made,
    /// And units ceasing to exist — consumed, perished, scrapped.
    Gone,
}

/// One instrument. Columnar, like every store here: a row per instrument and the id IS the row.
#[derive(Default)]
pub struct Instruments {
    /// No money without an issuer. Every row has one, and there is no constructor
    /// that omits it.
    issuer: Vec<u32>,
    ccy: Vec<u32>,
    class: Vec<Class>,
    unit: Vec<u32>,
    /// 5 C4.b: a TERM — fixed for the instrument's life, permanent structure, justified one at a
    /// time. `None` where the instrument pays no coupon.
    coupon: Vec<Option<f64>>,
    /// 5 C3: instruments outstanding at period zero have terms AND A REMAINING LIFE — a bond seeded
    /// at issue is a world with no maturity wall for its whole tenor.
    matures: Vec<Option<Day>>,
    /// CARRIED AT COST IS A DECLARED PROPERTY OF THE ASSET, and this is where it is
    /// declared.
    ///
    /// *"An asset genuinely not traded is carried at cost, and carried at cost is a declared
    /// property of the asset, not an accident of nobody having written it a market."* So it is
    /// not the `None` arm of a price lookup and it is not the absence of a book: a line nobody
    /// declared and nobody priced is a MISSING market, which is a different state from a line that
    /// genuinely does not trade, and only one of the two is an answer. `worth` throws on the other.
    at_cost: Vec<bool>,
    /// HOW MUCH OF THIS LINE EXISTS — set when it was issued and changed only by a
    /// named event.
    ///
    /// There was no such column, and that is why B2 — *holdings sum to the issued amount, per
    /// instrument, always* — was not merely unchecked but UNWRITABLE. The only other number in the
    /// world is `Register::held_total`, which IS the sum of the holdings, so a check against it
    /// would compare the answer with itself (Audit A1.a's tautology). Two independent sides is the
    /// whole of what makes B2.a readable: *a shortfall means somebody's claim vanished; a surplus
    /// means somebody's was invented.*
    ///
    /// It is not the issuer's liability. What an issuer owes is read off the holdings of
    /// what it issued, in `equity`, and stays there — this is the other side of that read, not a
    /// second copy of it.
    issued: Vec<f64>,
    /// The money line each issuer issues, by issuer row. A bank issues one deposit
    /// money and a central bank one reserve money, and the payment system has to be able to ask
    /// which — an index over this store rather than a second table somebody keeps beside it.
    money_of: Vec<u32>,
    /// The lines one issuer brought. Register A3's both-directions rule. A company asking
    /// *are my shares listed and held by outsiders* had to walk every instrument in the
    /// world — 16,750 of them, per company, per period — so the read that decides whether it reports
    /// was one nobody could afford to take.
    by_issuer: std::collections::HashMap<u32, Vec<u32>>,
}

impl Instruments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.issuer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.issuer.is_empty()
    }

    /// The only way to make one. The issuer is not optional, so Appendix B's money without an issuer
    /// cannot be written rather than being checked for.
    pub fn issue(
        &mut self,
        issuer: PartyId,
        ccy: CurrencyCode,
        class: Class,
        unit: UnitId,
        coupon: Option<f64>,
        matures: Option<Day>,
    ) -> InstrumentId {
        assert!(issuer.some(), "Money A1: no instrument without an issuer");
        if let Some(c) = coupon {
            assert!(
                class == Class::Claim,
                "5 C4.b: a coupon is a term of a CLAIM; {class:?} paying {c} is a price wearing a term's clothes"
            );
        }
        let row = self.issuer.len() as u32;
        if class == Class::Money {
            while self.money_of.len() <= issuer.row() {
                self.money_of.push(PartyId::NONE.0);
            }
            assert!(
                self.money_of[issuer.row()] == PartyId::NONE.0,
                "Money D2, Law 4: an issuer issues ONE money — two would be two answers to \"what do \
                 I owe my depositors\", and nothing could say which account a payment lands in"
            );
            self.money_of[issuer.row()] = row;
        }
        self.issuer.push(issuer.0);
        self.by_issuer.entry(issuer.0).or_default().push(row);
        self.ccy.push(ccy.0);
        self.class.push(class);
        self.unit.push(unit.0);
        self.coupon.push(coupon);
        self.matures.push(matures);
        // A line exists before any of it does. The units arrive by a named event —
        // `Brings` settles a `Leg::Create` for them — so issuing the line and issuing the units are
        // two acts, and this is the first.
        self.issued.push(0.0);
        // And nothing is carried at cost until somebody SAYS so. A line issued and never
        // declared is a line whose market is missing, not one that does not trade.
        self.at_cost.push(false);
        InstrumentId(row)
    }

    /// This line is not traded, and what it is worth is what it cost. The kernel declares
    /// it where a `Brings` asks for no book, because that is the same decision seen from the other
    /// side and there is one writer of it. A seed declares its own.
    pub fn carried_at_cost(&mut self, i: InstrumentId) {
        self.at_cost[i.row()] = true;
    }

    #[inline]
    pub fn is_carried_at_cost(&self, i: InstrumentId) -> bool {
        self.at_cost[i.row()]
    }

    /// What exists of this line. The independent side of the identity, against
    /// which the holdings are summed.
    #[inline]
    pub fn issued_of(&self, i: InstrumentId) -> f64 {
        self.issued[i.row()]
    }

    /// The one writer of how much of a line there is, and it is SETTLEMENT that
    /// calls it — because settlement is where units come into and go out of existence, and a second
    /// caller anywhere else would be a second writer of the same fact.
    ///
    /// Nothing is clamped. Units ceasing beyond what was ever issued is not a smaller
    /// disappearance, it is a read of the wrong line, and the citation says which.
    /// It does not refuse a line that goes below zero, and that is deliberate. Units ceasing
    /// beyond what was ever issued means the register holds units this store never saw issued — and
    /// that is Register B2's finding, not a contract violation: *"a shortfall means somebody's claim
    /// vanished; a surplus means somebody's was invented."* CLAUDE.md's discipline puts the line
    /// exactly there — a contract violation throws at the site, an invariant violation is reported
    /// with an owner and a size and is never thrown and never repaired.
    ///
    /// It will go below zero today, everywhere, and the reason is worth reading. The seeding
    /// credits `Register::credit` and `money_delta` directly rather than settling a `Leg::Create`,
    /// so every unit this world opened with exists on the register and was never issued. The
    /// measure says so, which is the first time anything could.
    pub fn moves(&mut self, i: InstrumentId, event: Issuance, units: f64) {
        assert!(units > 0.0, "Register B1: an event over {units} units is not an event");
        match event {
            Issuance::Made => self.issued[i.row()] += units,
            Issuance::Gone => self.issued[i.row()] -= units,
        }
    }

    /// The money this party issues, if it issues one. `Missing` is missing: a party
    /// that issues no money has none, and that is not a zeroth instrument.
    pub fn money_issued_by(&self, p: PartyId) -> Option<InstrumentId> {
        match self.money_of.get(p.row()) {
            Some(row) if *row != PartyId::NONE.0 => Some(InstrumentId::at(*row)),
            _ => None,
        }
    }

    #[inline]
    pub fn issuer_of(&self, i: InstrumentId) -> PartyId {
        PartyId(self.issuer[i.row()])
    }

    /// The lines this party brought, oldest first. Register A3: both directions.
    pub fn of_issuer(&self, p: PartyId) -> &[u32] {
        match self.by_issuer.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    #[inline]
    pub fn ccy_of(&self, i: InstrumentId) -> CurrencyCode {
        CurrencyCode(self.ccy[i.row()])
    }

    #[inline]
    pub fn class_of(&self, i: InstrumentId) -> Class {
        self.class[i.row()]
    }

    #[inline]
    pub fn unit_of(&self, i: InstrumentId) -> UnitId {
        UnitId(self.unit[i.row()])
    }

    /// 5 C4.b: the TERM. Fixed for the life of the instrument.
    #[inline]
    pub fn coupon_of(&self, i: InstrumentId) -> Option<f64> {
        self.coupon[i.row()]
    }

    #[inline]
    pub fn matures_on(&self, i: InstrumentId) -> Option<Day> {
        self.matures[i.row()]
    }

    /// Price 1 for money is the only hard-coded price there is, and it is a fact
    /// about the class. Everything else has to have printed.
    pub fn hard_coded_price(&self, i: InstrumentId) -> Option<f64> {
        match self.class_of(i) {
            Class::Money => Some(1.0),
            _ => None,
        }
    }

    /// 5 C3.a: a maturity profile that is SPREAD, or every roll arrives in the same period. A
    /// read over the rows — how much matures on or before a day.
    pub fn maturing_by(&self, when: Day, held: impl Fn(InstrumentId) -> f64) -> f64 {
        (0..self.len())
            .map(|row| InstrumentId(row as u32))
            .filter(|i| matches!(self.matures_on(*i), Some(d) if d <= when))
            .map(held)
            .sum()
    }

    /// Named as a market names it — issuer, coupon and maturity for a bond; the issuer
    /// alone for a share. The id is never the name.
    pub fn display(&self, i: InstrumentId, issuer_name: &str) -> String {
        match (self.class_of(i), self.coupon_of(i), self.matures_on(i)) {
            (Class::Claim, Some(c), Some(m)) => format!("{issuer_name} {c} {}", m.0),
            (Class::Claim, None, Some(m)) => format!("{issuer_name} {}", m.0),
            (Class::Money, _, _) => format!("{issuer_name} deposit"),
            _ => issuer_name.to_string(),
        }
    }
}

/// VALUE IS `units × price(asset)`, COMPUTED AT READ — and this is the one place that
/// computes it.
///
/// @spec XI-6 · Register D3 · Money A2.b · Appendix A
///
/// It was three places, character for character: a fund's NAV, a prime broker's client assets and a
/// levered fund's book each carried `match prints.latest(…) { Some(print) => units * print.price,
/// None => …lots…basis_per_unit…sum() }`. One fact, three writers, and XI-6's whole point
/// is that value is a function.
///
/// Money is the single degenerate case: its price is one by definition, and that is the only
/// place a hard-coded one is allowed.
///
/// An unpriced read is not a cost and not a zero, and it is `Missing`. Appendix A: *an unpriced
/// instrument is NOT PRICED, and whoever asked must handle that; a price of zero is a price, and
/// it propagates.* So a line with no print falls to its declaration, and a line that declared
/// nothing answers `None` — because the alternative is the silent cost arm XI-6 names, and with one
/// book in 1,546 printing per period that arm is not the exception, it is the valuation.
///
/// `None` is a real state of this world and not a corner. A line with a book that has never
/// crossed is unpriced: Clearing C4 says a book that ran and nothing crossed carries its last level
/// *and says so*, and a book with no last level has nothing to carry. 1,541 of this world's lines
/// are in exactly that state, so a caller that demanded a number would be demanding one that does
/// not exist.
pub fn worth(
    row: HoldingId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    period: u32,
) -> Option<f64> {
    let line = register.instrument_of(row);
    let units = register.quantity(row);
    // Zero multiplies. None of this line is held, so what it is worth is knowable
    // without knowing what one of it costs — and an emptied row is not an unpriced holding. A row
    // stays on the register after its last unit leaves, so without this a single sold-out line
    // would make its holder's whole book unvaluable for ever.
    if units == 0.0 {
        return Some(0.0);
    }
    if let Some(one) = instruments.hard_coded_price(line) {
        return Some(units * one);
    }
    match prints.latest(line, period) {
        // Read the way its book quotes it. A line quoted as a RATE has no
        // money value per unit, and `money` refuses it rather than multiplying by it.
        Some(print) => {
            Some(units * crate::prices::Prints::money(&print, "XI-6: what a holding is worth"))
        }
        None if instruments.is_carried_at_cost(line) => {
            Some(register.lots(row).iter().map(|l| l.qty * l.basis_per_unit).sum())
        }
        None => None,
    }
}

/// What a party's holdings are worth, or `Missing` where ANY of them cannot be valued.
///
/// A book with one line nobody has priced has no value. A sum that quietly left that line out would
/// be a number that looks like the whole book and is not — which is the same silence as the cost
/// arm, one level up. A caller that gets `None` here knows it cannot value this party, and what it
/// does about that is its own decision to state.
pub fn book_value(
    who: PartyId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    period: u32,
) -> Option<f64> {
    let mut total = 0.0;
    for &row in register.of_holder(who) {
        total += worth(HoldingId(row), register, instruments, prints, period)?;
    }
    Some(total)
}

/// WHAT A PARTY IS WORTH: WHAT IT HOLDS, LESS WHAT IT OWES. A read, every time.
///
/// @spec Audit B5 · 5 A4 · 5 C2 · Law 4, Law 12, Law 19 · Appendix B
///
/// It used to be a POT the register kept and settlement bumped, and that was wrong twice over.
/// Appendix B forbids a stored aggregate and says capital is never a pot; and the pot counted the
/// ASSET SIDE ONLY, so an issuer got richer by issuing — the treasury's equity rose when it sold a
/// bill, a firm's when it borrowed, because the money arrived and the obligation went nowhere. The
/// snapshot had to write the number down because nothing derived it, which is what made it
/// impossible to miss.
///
/// Holdings are at what they COST, which is what the lots carry and what a money total is worth
/// at the one hard-coded price. Nothing here consults a market: an equity read that
/// re-priced a book would be re-deriving what a market printed, and what a party is worth
/// AT MARKET is a different question with a different name.
/// A claim is neither held nor issued, and it is still owed. An estate's liabilities
/// stand in `Claims` and nothing about them is an instrument — nobody holds a row, nobody
/// issued a line — so a read that only walked holdings and issues gave an estate the assets it took
/// on and none of the debts that came with them: `probate.us.1.bank.b` closed a year with an account
/// equal to its assets exactly and 15.7bn of liabilities nowhere in it. Both sides are read here,
/// from the store's two indexes, because a claim subtracted from the estate and not added to its
/// holder would be a one-sided flow.
pub fn equity(party: PartyId, register: &Register, instruments: &Instruments, claims: &Claims) -> f64 {
    let holds: f64 = register
        .of_holder(party)
        .iter()
        .map(|row| at_cost(register, HoldingId(*row)))
        .sum::<f64>()
        + claims.owed_to(party);
    // 5 A4: and what it owes is what OTHERS hold of what it issued. Its own line on its own book is
    // not a debt to itself — netting it off here is the whole of what "issued and outstanding" means.
    //
    // Only money and claims are debts. Law 8: units are part of the number, and a share is not a
    // sum of money at all — it IS the residual this function computes, so counting it as a liability
    // would net a firm to nothing by construction. A good or a plant on an issuer's book is a thing
    // it made, not a promise it owes. Money and a claim are carried at par, which is what a unit of
    // each is worth to the party that must hand it over.
    let owes = owed_by(party, instruments, |i| {
        match instruments.class_of(i) {
            Class::Money | Class::Claim => {
                let (held, _) = register.held_total(i);
                held - register.quantity(register.row(party, i))
            }
            Class::Share | Class::Good | Class::Plant => 0.0,
        }
    });
    holds - owes - claims.owed_by_estate(party)
}

/// What one holding cost. A money account is a total at price 1; anything else
/// carries the basis its units arrived with.
fn at_cost(register: &Register, row: HoldingId) -> f64 {
    if register.is_total(row) {
        return register.quantity(row);
    }
    register.lots(row).iter().map(|l| l.qty * l.basis_per_unit).sum()
}

/// 5 A4, C2: every asset is somebody's liability, party by party. What an issuer owes, read from
/// the holdings of what it issued — never a second tally kept beside them.
pub fn owed_by(
    issuer: PartyId,
    instruments: &Instruments,
    held_of: impl Fn(InstrumentId) -> f64,
) -> f64 {
    (0..instruments.len())
        .map(|row| InstrumentId(row as u32))
        .filter(|i| instruments.issuer_of(*i) == issuer)
        .map(held_of)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn ccy() -> CurrencyCode {
        CurrencyCode::at(0)
    }

    fn unit() -> UnitId {
        UnitId::at(0)
    }

    #[test]
    fn every_instrument_has_an_issuer_and_there_is_no_door_that_omits_one() {
        // No money without an issuer, and no holding without one. The register knows who
        // HOLDS; this knows who OWES.
        let mut i = Instruments::new();
        let deposit = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        assert_eq!(i.issuer_of(deposit), party(5));
    }

    #[test]
    #[should_panic(expected = "no instrument without an issuer")]
    fn an_instrument_issued_by_nobody_is_refused() {
        let mut i = Instruments::new();
        i.issue(PartyId::NONE, ccy(), Class::Money, unit(), None, None);
    }

    #[test]
    fn price_one_for_money_is_the_only_hard_coded_price() {
        // Everything else has to have printed.
        let mut i = Instruments::new();
        let deposit = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        let bond = i.issue(party(9), ccy(), Class::Claim, unit(), Some(0.04), Some(Day(900)));
        let share = i.issue(party(9), ccy(), Class::Share, unit(), None, None);
        assert_eq!(i.hard_coded_price(deposit), Some(1.0));
        assert!(i.hard_coded_price(bond).is_none());
        assert!(i.hard_coded_price(share).is_none());
    }

    // `worth` HAS NO TEST, and the reason is the testing rule. What it does is a READ over three
    // stores, so the only way to assert on it is to build a register, an instrument table
    // and a print — parties, holdings and prices, arranged by the same hand that wrote the match.
    // That is a second world, and it would pass for exactly as long as the arrangement held.
    //
    // What guards it instead:
    //  - the TYPE. `Option<f64>` is what makes the unpriced arm unmissable: no caller can take a
    //  number that is not there, and each of the three states what it does about `None`.
    //  - `Prints::money`, which refuses a line quoted as a rate rather than multiplying by it.
    //  - the MEASUREMENT, against the real world: the Accounts family reports every holding
    //  whose line is neither printed nor declared carried at cost, with its owner. That is the
    //  same question asked of a world nobody arranged — 15,204 lines of it rather than four.

    #[test]
    fn a_money_account_carries_no_lots_because_every_unit_is_the_same_unit() {
        // Money D2, and the reason the audit reported ten thousand violations in its first
        // end-to-end period, before the register had a `total_only` column: every money account in
        // the world looked like a holding whose lots had gone missing.
        assert!(!Class::Money.carries_lots());
        assert!(Class::Claim.carries_lots());
        assert!(Class::Good.carries_lots());
    }

    #[test]
    fn a_coupon_is_a_term_of_a_claim_and_not_a_price_on_something_else() {
        // 5 C4.b: a seeded term is permanent structure and must be justified individually; an
        // opening price is a guess the next period re-clears. A share with a coupon would be the
        // second wearing the first's clothes.
        let mut i = Instruments::new();
        let bond = i.issue(party(9), ccy(), Class::Claim, unit(), Some(0.04), Some(Day(900)));
        assert_eq!(i.coupon_of(bond), Some(0.04));
        assert_eq!(i.matures_on(bond), Some(Day(900)));
    }

    #[test]
    #[should_panic(expected = "wearing a term's clothes")]
    fn a_share_with_a_coupon_is_refused() {
        let mut i = Instruments::new();
        i.issue(party(9), ccy(), Class::Share, unit(), Some(0.04), None);
    }

    #[test]
    fn a_bond_outstanding_at_period_zero_has_a_remaining_life() {
        // 5 C3: a bond seeded at issue is a world with no maturity wall for its whole tenor, and
        // 5 C3.a wants the profile SPREAD rather than stacked on one day.
        let mut i = Instruments::new();
        let soon = i.issue(party(9), ccy(), Class::Claim, unit(), Some(0.04), Some(Day(100)));
        let later = i.issue(party(9), ccy(), Class::Claim, unit(), Some(0.05), Some(Day(2_000)));
        let held = |x: InstrumentId| if x == soon { 500.0 } else { 900.0 };
        assert_eq!(i.maturing_by(Day(200), held), 500.0);
        assert_eq!(i.maturing_by(Day(3_000), held), 1_400.0);
        assert_eq!(i.maturing_by(Day(10), held), 0.0);
        let _ = later;
    }

    #[test]
    fn what_an_issuer_owes_is_read_from_what_was_issued_and_never_kept_beside_it() {
        // 5 A4, C2, Law 19: every asset is somebody's liability, party by party.
        let mut i = Instruments::new();
        let a = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        let b = i.issue(party(5), ccy(), Class::Claim, unit(), Some(0.03), Some(Day(900)));
        let theirs = i.issue(party(6), ccy(), Class::Money, unit(), None, None);
        let held = |x: InstrumentId| {
            if x == a {
                1_000.0
            } else if x == b {
                400.0
            } else {
                9_999.0
            }
        };
        assert_eq!(owed_by(party(5), &i, held), 1_400.0);
        assert_eq!(owed_by(party(6), &i, held), 9_999.0);
        assert_eq!(owed_by(party(7), &i, held), 0.0);
        let _ = theirs;
    }

    #[test]
    fn an_instrument_is_displayed_as_a_market_would_name_it() {
        // The id is never the name.
        let mut i = Instruments::new();
        let bond = i.issue(party(9), ccy(), Class::Claim, unit(), Some(4.5), Some(Day(2_031)));
        let share = i.issue(party(9), ccy(), Class::Share, unit(), None, None);
        let deposit = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        assert_eq!(i.display(bond, "firm.4"), "firm.4 4.5 2031");
        assert_eq!(i.display(share, "firm.4"), "firm.4");
        assert_eq!(i.display(deposit, "bank.2"), "bank.2 deposit");
    }

    #[test]
    fn an_issuer_does_not_get_richer_by_issuing() {
        // 22b.7a, and the whole reason the pot had to go. Settlement used to BUMP an equity
        // account, and it bumped the asset side only: money arrived and the obligation went nowhere,
        // so the treasury got richer for selling a bill and a firm for borrowing. Read as holdings
        // against liabilities, issuing is what it is — neutral at the moment it happens.
        let mut i = Instruments::new();
        let mut reg = Register::new();
        let treasury = party(5);
        let buyer = party(6);
        let cash = i.issue(party(7), ccy(), Class::Money, unit(), None, None);
        let bill = i.issue(treasury, ccy(), Class::Claim, unit(), None, Some(Day(900)));

        reg.money_delta(buyer, cash, 1_000.0);
        let before = equity(treasury, &reg, &i, &Claims::new());

        // It sold the bill: money in, and a promise out at the same instant.
        reg.money_delta(buyer, cash, -900.0);
        reg.money_delta(treasury, cash, 900.0);
        reg.credit(buyer, bill, 900.0, 1.0, 0);

        assert_eq!(equity(treasury, &reg, &i, &Claims::new()), before, "selling a promise is not income");
        assert_eq!(equity(buyer, &reg, &i, &Claims::new()), 1_000.0, "and the buyer swapped money for a claim");
    }

    #[test]
    fn a_share_is_the_residual_and_never_a_liability() {
        // Units are part of the number, and shares are not a sum of money. Counting a firm's
        // own shares as a debt would net every firm in the world to nothing by construction.
        let mut i = Instruments::new();
        let mut reg = Register::new();
        let firm = party(1);
        let holder = party(2);
        let cash = i.issue(party(7), ccy(), Class::Money, unit(), None, None);
        let share = i.issue(firm, ccy(), Class::Share, unit(), None, None);
        let plant = i.issue(firm, ccy(), Class::Plant, unit(), None, None);

        reg.money_delta(firm, cash, 400.0);
        reg.credit(firm, plant, 10.0, 60.0, 0);
        reg.credit(holder, share, 100.0, 1.0, 0);

        // 400 of money and 600 of plant, and the shares its owners hold are not a debt against it.
        assert_eq!(equity(firm, &reg, &i, &Claims::new()), 1_000.0);
    }

    #[test]
    fn an_estate_is_worth_what_it_holds_less_what_is_claimed_on_it() {
        // An estate that took on what the dead party owed had a liability with no entry
        // against it — `probate.us.1.bank.b` closed a year with an account equal to its assets
        // exactly and 15.7bn of liabilities nowhere in it. A claim is neither held nor issued, so a
        // read that walked only holdings and issues could not see it.
        let mut i = Instruments::new();
        let mut reg = Register::new();
        let estate = party(3);
        let claimant = party(4);
        let cash = i.issue(party(7), ccy(), Class::Money, unit(), None, None);
        reg.money_delta(estate, cash, 1_000.0);

        let mut claims = Claims::new();
        assert_eq!(equity(estate, &reg, &i, &claims), 1_000.0, "nothing claimed on it yet");

        let c = claims.against(estate, claimant, 400.0, 0);
        assert_eq!(equity(estate, &reg, &i, &claims), 600.0, "what is claimed on it is owed");
        // The same row is an asset to whoever holds it, or the world's equity fell by 400.
        assert_eq!(equity(claimant, &reg, &i, &claims), 400.0);

        // And paying it discharges both sides at once, which is what makes a payment neutral: the
        // money leg and the mark are the same event (`running::Ranked`), so neither party's worth
        // moves. Marking it paid WITHOUT the leg would make an estate richer by paying, which is why
        // the two are proposed together.
        claims.pays(c, 400.0);
        reg.money_delta(estate, cash, -400.0);
        reg.money_delta(claimant, cash, 400.0);
        assert_eq!(equity(estate, &reg, &i, &claims), 600.0);
        assert_eq!(equity(claimant, &reg, &i, &claims), 400.0);
    }
}
