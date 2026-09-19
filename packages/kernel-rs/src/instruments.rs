//! THE INSTRUMENTS: every priced thing in the world, with the party that issued it.
//!
//! @spec Money A1 · Money A2.b · Money D2 · Register A1 · 5 A4 · 5 C3 · 5 C3.a · 5 C4.b · Law 2,
//! @spec Law 4, Law 8, Law 9 · Appendix B

use crate::calendar::Day;
use crate::ids::{CurrencyCode, HoldingId, InstrumentId, PartyId, UnitId};
use crate::register::Register;
use crate::stores::Claims;

/// What kind of thing this is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// An account is a holding of money ISSUED BY a bank or the central bank.
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
    /// Money accounts hold no lots — there is no basis to carry, because every unit is the same
    /// unit.
    pub fn carries_lots(&self) -> bool {
        *self != Class::Money
    }
}

/// The issued amount moves only by a NAMED event, and these are the ones this kernel has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Issuance {
    /// Register B1, Goods B: units come into existence.
    Made,
    /// And units ceasing to exist — consumed, perished, scrapped.
    Gone,
}

/// One instrument.
#[derive(Default)]
pub struct Instruments {
    /// No money without an issuer.
    issuer: Vec<u32>,
    ccy: Vec<u32>,
    class: Vec<Class>,
    unit: Vec<u32>,
    /// A TERM — fixed for the instrument's life, permanent structure, justified one at a
    /// time.
    coupon: Vec<Option<f64>>,
    /// Instruments outstanding at period zero have terms AND A REMAINING LIFE — a bond seeded
    /// at issue is a world with no maturity wall for its whole tenor.
    matures: Vec<Option<Day>>,
    /// CARRIED AT COST IS A DECLARED PROPERTY OF THE ASSET, and this is where it is declared.
    at_cost: Vec<bool>,
    /// HOW MUCH OF THIS LINE EXISTS — set when it was issued and changed only by a named event.
    issued: Vec<f64>,
    /// The money line each issuer issues, by issuer row.
    money_of: Vec<u32>,
    /// The lines one issuer brought.
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

    /// The only way to make one.
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
        // A line exists before any of it does.
        self.issued.push(0.0);
        // And nothing is carried at cost until somebody SAYS so.
        self.at_cost.push(false);
        InstrumentId(row)
    }

    /// This line is not traded, and what it is worth is what it cost.
    pub fn carried_at_cost(&mut self, i: InstrumentId) {
        self.at_cost[i.row()] = true;
    }

    #[inline]
    pub fn is_carried_at_cost(&self, i: InstrumentId) -> bool {
        self.at_cost[i.row()]
    }

    /// What exists of this line.
    #[inline]
    pub fn issued_of(&self, i: InstrumentId) -> f64 {
        self.issued[i.row()]
    }

    /// The one writer of how much of a line there is, and it is SETTLEMENT that calls it — because
    /// settlement is where units come into and go out of existence, and a second caller anywhere
    /// else would be a second writer of the same fact.
    pub fn moves(&mut self, i: InstrumentId, event: Issuance, units: f64) {
        assert!(units > 0.0, "Register B1: an event over {units} units is not an event");
        match event {
            Issuance::Made => self.issued[i.row()] += units,
            Issuance::Gone => self.issued[i.row()] -= units,
        }
    }

    /// The money this party issues, if it issues one.
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

    /// The lines this party brought, oldest first.
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

    /// The TERM.
    #[inline]
    pub fn coupon_of(&self, i: InstrumentId) -> Option<f64> {
        self.coupon[i.row()]
    }

    #[inline]
    pub fn matures_on(&self, i: InstrumentId) -> Option<Day> {
        self.matures[i.row()]
    }

    /// Price 1 for money is the only hard-coded price there is, and it is a fact about the class.
    pub fn hard_coded_price(&self, i: InstrumentId) -> Option<f64> {
        match self.class_of(i) {
            Class::Money => Some(1.0),
            _ => None,
        }
    }

    /// A maturity profile that is SPREAD, or every roll arrives in the same period.
    pub fn maturing_by(&self, when: Day, held: impl Fn(InstrumentId) -> f64) -> f64 {
        (0..self.len())
            .map(|row| InstrumentId(row as u32))
            .filter(|i| matches!(self.matures_on(*i), Some(d) if d <= when))
            .map(held)
            .sum()
    }

    /// Named as a market names it — issuer, coupon and maturity for a bond; the issuer alone for a
    /// share.
    pub fn display(&self, i: InstrumentId, issuer_name: &str) -> String {
        match (self.class_of(i), self.coupon_of(i), self.matures_on(i)) {
            (Class::Claim, Some(c), Some(m)) => format!("{issuer_name} {c} {}", m.0),
            (Class::Claim, None, Some(m)) => format!("{issuer_name} {}", m.0),
            (Class::Money, _, _) => format!("{issuer_name} deposit"),
            _ => issuer_name.to_string(),
        }
    }
}

/// VALUE IS `units × price(asset)`, COMPUTED AT READ — and this is the one place that computes it.
///
/// @spec XI-6 · Register D3 · Money A2.b · Appendix A
pub fn worth(
    row: HoldingId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    period: u32,
) -> Option<f64> {
    let line = register.instrument_of(row);
    let units = register.quantity(row);
    // Zero multiplies.
    if units == 0.0 {
        return Some(0.0);
    }
    if let Some(one) = instruments.hard_coded_price(line) {
        return Some(units * one);
    }
    match prints.latest(line, period) {
        // Read the way its book quotes it.
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

/// WHAT A PARTY IS WORTH: WHAT IT HOLDS, LESS WHAT IT OWES.
///
/// @spec Audit B5 · 5 A4 · 5 C2 · Law 4, Law 12, Law 19 · Appendix B
pub fn equity(party: PartyId, register: &Register, instruments: &Instruments, claims: &Claims) -> f64 {
    let holds: f64 = register
        .of_holder(party)
        .iter()
        .map(|row| at_cost(register, HoldingId(*row)))
        .sum::<f64>()
        + claims.owed_to(party);
    // And what it owes is what OTHERS hold of what it issued.
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

fn at_cost(register: &Register, row: HoldingId) -> f64 {
    if register.is_total(row) {
        return register.quantity(row);
    }
    register.lots(row).iter().map(|l| l.qty * l.basis_per_unit).sum()
}

/// Every asset is somebody's liability, party by party.
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
        // No money without an issuer, and no holding without one.
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

    // `worth` HAS NO TEST, and the reason is the testing rule.

    #[test]
    fn a_money_account_carries_no_lots_because_every_unit_is_the_same_unit() {
        assert!(!Class::Money.carries_lots());
        assert!(Class::Claim.carries_lots());
        assert!(Class::Good.carries_lots());
    }

    #[test]
    fn a_coupon_is_a_term_of_a_claim_and_not_a_price_on_something_else() {
        // A seeded term is permanent structure and must be justified individually; an
        // opening price is a guess the next period re-clears.
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
        // A bond seeded at issue is a world with no maturity wall for its whole tenor, and 5
        // C3.a wants the profile SPREAD rather than stacked on one day.
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
        // 22b.7a, and the whole reason the pot had to go.
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
        // Units are part of the number, and shares are not a sum of money.
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

        claims.pays(c, 400.0);
        reg.money_delta(estate, cash, -400.0);
        reg.money_delta(claimant, cash, 400.0);
        assert_eq!(equity(estate, &reg, &i, &claims), 600.0);
        assert_eq!(equity(claimant, &reg, &i, &claims), 400.0);
    }
}
