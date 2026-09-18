//! THE INSTRUMENTS: every priced thing in the world, with **the party that issued it**.
//!
//! @spec Money A1 · Money A2.b · Money D2 · Register A1 · 5 A4 · 5 C3 · 5 C3.a · 5 C4.b · Law 2,
//! @spec Law 4, Law 8, Law 9 · Appendix B
//!
//! **No money without an issuer, and no holding without an issuer** (Appendix B). The register knows
//! who HOLDS what; this knows who OWES it. Without both, a seed can credit a household with a deposit
//! that is nobody's liability — 5 A4's free money — and nothing in the world could say so.
//!
//! **Price 1 for money is the only hard-coded price** (Money A2.b), and it is a fact about the CLASS
//! rather than a price anybody printed.
//!
//! **A term is not an opening condition** (5 C4.b). A coupon is fixed for the instrument's life and is
//! permanent structure; a price is an opening guess the next period re-clears. They are different
//! fields here for that reason: `coupon` is a term, and a price lives in `Prints` where it can be
//! re-cleared.
//!
//! **Instruments are named as a market names them** (Law 9): issuer plus coupon plus maturity for a
//! bond, the issuer alone for a share. The row carries the facts; `Names` turns them into the display,
//! and an internal id is never shown.

use crate::calendar::Day;
use crate::ids::{CurrencyCode, InstrumentId, PartyId, UnitId};

/// Register A1: what kind of thing this is. Not a branch a mechanism takes (Law 15) — the kernel uses
/// it to know what may be held in fractions, what carries lots, and what money is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Money D2: an account is a holding of money ISSUED BY a bank or the central bank. It carries no
    /// lots, because one unit of it is every other unit.
    Money,
    /// A promise to pay: a bond, a bill, a loan row.
    Claim,
    /// A share in an issuer's residual.
    Share,
    /// A physical good, in its own unit (Law 8).
    Good,
    /// A productive asset with its own life.
    Plant,
}

impl Class {
    /// Money D2: money accounts hold no lots — there is no basis to carry, because every unit is the
    /// same unit. The register's `total_only` column is set from this.
    pub fn carries_lots(&self) -> bool {
        *self != Class::Money
    }
}

/// One instrument. Columnar, like every store here: a row per instrument and the id IS the row.
#[derive(Default)]
pub struct Instruments {
    /// Appendix B: **no money without an issuer.** Every row has one, and there is no constructor
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
        self.issuer.push(issuer.0);
        self.ccy.push(ccy.0);
        self.class.push(class);
        self.unit.push(unit.0);
        self.coupon.push(coupon);
        self.matures.push(matures);
        InstrumentId(row)
    }

    #[inline]
    pub fn issuer_of(&self, i: InstrumentId) -> PartyId {
        PartyId(self.issuer[i.row()])
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

    /// Money A2.b: **price 1 for money is the only hard-coded price there is**, and it is a fact
    /// about the class. Everything else has to have printed.
    pub fn hard_coded_price(&self, i: InstrumentId) -> Option<f64> {
        match self.class_of(i) {
            Class::Money => Some(1.0),
            _ => None,
        }
    }

    /// 5 C3.a: **a maturity profile that is SPREAD**, or every roll arrives in the same period. A
    /// read over the rows — how much matures on or before a day.
    pub fn maturing_by(&self, when: Day, held: impl Fn(InstrumentId) -> f64) -> f64 {
        (0..self.len())
            .map(|row| InstrumentId(row as u32))
            .filter(|i| matches!(self.matures_on(*i), Some(d) if d <= when))
            .map(held)
            .sum()
    }

    /// Law 9: **named as a market names it** — issuer, coupon and maturity for a bond; the issuer
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

/// 5 A4, C2: **every asset is somebody's liability, party by party.** What an issuer owes, read from
/// the holdings of what it issued — never a second tally kept beside them (Law 19).
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
        // Appendix B: no money without an issuer, and no holding without one. The register knows who
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
        // Money A2.b: everything else has to have printed.
        let mut i = Instruments::new();
        let deposit = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        let bond = i.issue(party(9), ccy(), Class::Claim, unit(), Some(0.04), Some(Day(900)));
        let share = i.issue(party(9), ccy(), Class::Share, unit(), None, None);
        assert_eq!(i.hard_coded_price(deposit), Some(1.0));
        assert!(i.hard_coded_price(bond).is_none());
        assert!(i.hard_coded_price(share).is_none());
    }

    #[test]
    fn a_money_account_carries_no_lots_because_every_unit_is_the_same_unit() {
        // Money D2, and the reason `LotsAgainstQuantity` found ten thousand violations before the
        // register had a `total_only` column.
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
        // Law 9: the id is never the name.
        let mut i = Instruments::new();
        let bond = i.issue(party(9), ccy(), Class::Claim, unit(), Some(4.5), Some(Day(2_031)));
        let share = i.issue(party(9), ccy(), Class::Share, unit(), None, None);
        let deposit = i.issue(party(5), ccy(), Class::Money, unit(), None, None);
        assert_eq!(i.display(bond, "firm.4"), "firm.4 4.5 2031");
        assert_eq!(i.display(share, "firm.4"), "firm.4");
        assert_eq!(i.display(deposit, "bank.2"), "bank.2 deposit");
    }
}
