//! THE INSTRUMENTS: every priced thing in the world, with the party that issued it.
//!
//! @spec Money A1 · Money A2.b · Money D2 · Register A1 · 5 A4 · 5 C3 · 5 C3.a · 5 C4.b · Law 2,
//! @spec Law 4, Law 8, Law 9 · Appendix B

use crate::calendar::{Convention, Day};
use crate::ids::{CurrencyCode, HoldingId, InstrumentId, PartyId, UnitId};
use crate::register::Register;
use crate::stores::Claims;

/// WHAT A PRICED THING RETURNS, derived FROM its price and only this way round: what a holder gets
/// back over what it pays, spread over the days it waits, on the convention its market quotes in.
///
/// It lives with the thing that repays rather than with the price, so the file that WRITES prices
/// has no way to compute one and nothing can wire the derivation backwards.
pub fn yield_to(price: f64, repays: f64, from: Day, to: Day, c: Convention) -> Option<f64> {
    let days = to.0 - from.0;
    match price > 0.0 && days > 0 {
        true => Some((repays / price - 1.0) * c.year() / days as f64),
        false => None,
    }
}

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

    /// Price 1 for money is the only hard-coded price there is, and it is a fact about the class.
    pub fn hard_coded_price(&self) -> Option<f64> {
        match self {
            Class::Money => Some(1.0),
            _ => None,
        }
    }
}

/// Named as a market names it — issuer, coupon and maturity for a bond, the issuer alone for a
/// share. An internal id is never a display name.
pub fn named_as(class: Class, coupon: Option<f64>, matures: Option<Day>, issuer_name: &str) -> String {
    match (class, coupon, matures) {
        (Class::Claim, Some(c), Some(m)) => format!("{issuer_name} {c} {}", m.0),
        (Class::Claim, None, Some(m)) => format!("{issuer_name} {}", m.0),
        (Class::Money, _, _) => format!("{issuer_name} deposit"),
        _ => issuer_name.to_string(),
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
        self.class_of(i).hard_coded_price()
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
        named_as(self.class_of(i), self.coupon_of(i), self.matures_on(i), issuer_name)
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

// `issue` is the only door and it takes an issuer, so an instrument without one cannot be written
// by any route the type allows; what it refuses at the site is the NONE sentinel being passed, and
// a coupon on something that is not a claim. The rest of this store answers what `issue` was told.
//
// `equity` is a read over three stores, so what it answers is a question about a world: an issuer
// that does not get richer by issuing, a share that is a residual and not a liability, an estate
// worth what it holds less what is claimed on it. Those are positioned at 0n.5, where the Accounts
// family asks them of every party every period.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_is_the_only_class_with_a_price_and_the_only_one_without_lots() {
        assert_eq!(Class::Money.hard_coded_price(), Some(1.0));
        assert_eq!(Class::Claim.hard_coded_price(), None);
        assert_eq!(Class::Share.hard_coded_price(), None);
        assert_eq!(Class::Good.hard_coded_price(), None);
        assert_eq!(Class::Plant.hard_coded_price(), None);

        assert!(!Class::Money.carries_lots());
        assert!(Class::Claim.carries_lots());
        assert!(Class::Good.carries_lots());
    }

    #[test]
    fn an_instrument_is_named_as_a_market_would_name_it() {
        let bond = named_as(Class::Claim, Some(4.5), Some(Day(2_031)), "firm.4");
        assert_eq!(bond, "firm.4 4.5 2031");
        // A bill has no coupon, so it is the issuer and the date.
        assert_eq!(named_as(Class::Claim, None, Some(Day(900)), "us"), "us 900");
        // A share is the issuer, and nothing else.
        assert_eq!(named_as(Class::Share, None, None, "firm.4"), "firm.4");
        assert_eq!(named_as(Class::Money, None, None, "bank.2"), "bank.2 deposit");
    }
}
