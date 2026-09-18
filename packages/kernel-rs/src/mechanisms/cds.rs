//! CREDIT DEFAULT SWAPS: a running spread that CLEARS, a premium leg that is real money and stops on
//! the event, and a protection leg somebody must actually be able to pay.
//!
//! @spec 17 A1.a · 17 A1.b · 17 A1.c · 17 A1.d · 17 A2 · 17 A3 · 17 A4 · 17 A4.a · 17 A5 · 17 A5.a ·
//! @spec 17 A5.b · 17 B1 · 17 B2 · 17 B2.a · 17 B3 · 17 B5 · 17 C1 · 17 C2 · 17 C3 · 17 C3.a ·
//! @spec 17 C3.b · 17 D1 · 17 D2 · 17 D2.a · 17 D2.b · 17 D3 · 17 D4 · 17 D5 · 17 E2 · 17 E3 ·
//! @spec 17 E4 · XI-13 · XI-1 · Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! **The implied probability is derived FROM the spread, never into it** (C2, XI-13). A default
//! probability computed from the firm's accounts and fed to every seller's reservation means the
//! market cannot disagree with the accounting model — Law 3 inverted in the one instrument whose
//! entire purpose is to hold a second opinion about a credit. So `implied` reads a cleared spread and
//! there is no function here that takes accounts.
//!
//! **No fixed recovery rate** (D2.a): a constant recovery makes the payoff a constant and turns a
//! credit derivative into an interest-rate instrument. `Recovery` carries what the defaulted
//! obligations actually fetched, with the auction or workout it came from.
//!
//! **A term structure of credit, not one number** (A1.d): a `Curve` is several tenors, and a single
//! tenor means the model has no term structure of credit anywhere.
//!
//! **No protection that pays without a payer** (E4): the seller's ability to pay is part of the
//! instrument, and `pays_out` answers with what the seller could actually deliver and what it could
//! not — which is E2's wrong-way risk when the seller is correlated with the reference.
//!
//! **Not every participant is a hedger** (B5, XI-13): `can_clear` refuses a book whose only buyer is
//! above an exposure limit and whose only seller is closing a regulatory gap. That book's spread is a
//! function of regulatory gaps and never of a view, it does not open at all in a period where neither
//! gap binds, and its price cannot move because somebody thinks the credit is mispriced.

use crate::ids::PartyId;

/// A1.a: the underlying is **a named reference entity and its default event** — not a price. A4: it
/// must exist in this world and be capable of defaulting, and A4.a forbids protection on an entity
/// nobody can observe failing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reference {
    pub entity: PartyId,
    /// A4: whether this party can fail at all. A central bank, or a treasury in the money it issues,
    /// cannot (XI-3) — and protection on one is protection on nothing.
    pub can_fail: bool,
}

/// A1.c, A1.d: one line of the curve — a tenor and the spread that cleared at it, in the contract's
/// own currency and periodicity (Law 8).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Point {
    pub tenor_years: f64,
    /// C1: cleared from the two sides' schedules. Never posted.
    pub spread: f64,
}

/// A1.d: **a term structure of credit and not one number.**
#[derive(Clone, Debug)]
pub struct Curve {
    pub on: Reference,
    pub points: Vec<Point>,
}

impl Curve {
    pub fn new(on: Reference, points: Vec<Point>) -> Curve {
        assert!(on.can_fail, "17 A4.a: no protection on an entity nobody can observe failing");
        assert!(
            points.len() > 1,
            "17 A1.d: one tenor is not a term structure, and a model with one has none anywhere"
        );
        Curve { on, points }
    }

    pub fn at(&self, tenor_years: f64) -> Option<f64> {
        self.points.iter().find(|p| p.tenor_years == tenor_years).map(|p| p.spread)
    }
}

/// The contract. A2: the premium leg is a **real periodic payment**, in cash, in the contract's
/// currency, **and it stops on the event**.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Contract {
    pub buyer: PartyId,
    pub seller: PartyId,
    pub on: Reference,
    pub notional: f64,
    pub spread: f64,
    pub tenor_years: f64,
    /// D2.b: a triggered contract **pays no premium**, marks at its expected payoff, and **holds past
    /// its own maturity** until the workout closes.
    pub triggered: bool,
}

impl Contract {
    /// A2: real money, per period, from the buyer to the seller — and nothing once the event has
    /// fired (Law 5: two named sides).
    pub fn premium(&self, periods_per_year: f64) -> Option<(PartyId, PartyId, f64)> {
        if self.triggered {
            return None;
        }
        Some((self.buyer, self.seller, self.notional * self.spread / periods_per_year))
    }
}

/// D2: **a recovery determined by what the defaulted obligations are actually worth** — an auction or
/// a realised workout, not an assumption. It carries where it came from, so a constant cannot be
/// passed where a recovery is wanted.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Recovery {
    pub of: PartyId,
    /// What the obligations fetched, per unit of par.
    pub fetched: f64,
    /// D2.b: false while the workout is open — the contract then marks at its EXPECTED payoff and
    /// the settlement is a true-up when this closes.
    pub workout_closed: bool,
}

/// A1.b: **on the event the protection seller pays par minus recovery on the notional; otherwise
/// nothing.** D3: real money from the seller to the buyer, and it can be large enough to fail the
/// seller.
pub fn owed_on_event(c: &Contract, r: &Recovery) -> f64 {
    assert!(r.of == c.on.entity, "17 A1.b: a recovery on one name does not settle another's contract");
    c.notional * (1.0 - r.fetched)
}

/// What the seller actually delivered, and what it did not. **No protection that pays without a
/// payer** (E4): the seller's ability to pay is part of the instrument, and E2's wrong-way risk is
/// exactly the case where this shortfall arrives when the protection was most needed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paid {
    pub from: PartyId,
    pub to: PartyId,
    pub paid: f64,
    /// What the buyer expected and did not receive. A loss with a holder (Law 5, XI-1).
    pub short: f64,
}

pub fn pays_out(c: &Contract, r: &Recovery, seller_can_find: f64) -> Paid {
    let owed = owed_on_event(c, r);
    let paid = if seller_can_find < owed { seller_can_find } else { owed };
    Paid { from: c.seller, to: c.buyer, paid, short: owed - paid }
}

/// C2, A3: **the implied default probability is a READ from the cleared spread** and the recovery —
/// never an input to either. `None` where the obligations paid in full: there is no loss to divide
/// by, and inventing one is the numeric default Appendix A refuses.
pub fn implied(spread: f64, r: &Recovery) -> Option<f64> {
    let loss_given_default = 1.0 - r.fetched;
    if loss_given_default <= 0.0 {
        return None;
    }
    Some(spread / loss_given_default)
}

/// Why a party is in this book. XI-13, B5: **a speculative participant with a view is required on
/// both sides.**
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// B1: it holds the issuer's debt, or lends to it and cannot sell the loan (B1.a).
    Hedging,
    /// B2: it wants credit exposure without funding a bond, or B1/B2's view — it thinks the credit
    /// will deteriorate, or that the spread is too wide for the risk.
    AView,
    /// B4: a dealer intermediates, and its book is rarely flat.
    Dealer,
}

#[derive(Clone, Copy, Debug)]
pub struct Participant {
    pub who: PartyId,
    pub reason: Reason,
    pub buying: bool,
    /// B3: **naked positions are possible on both sides** and they are how the market gets liquid —
    /// but a naked seller is an unfunded credit exposure and must be capitalised as one.
    pub naked: bool,
}

/// B5, XI-13: a book of hedgers on both sides clears at a function of regulatory gaps and never of a
/// view. **A speculative participant with a view is required on both sides.**
pub fn can_clear(book: &[Participant]) -> bool {
    let view_buying = book.iter().any(|p| p.reason == Reason::AView && p.buying);
    let view_selling = book.iter().any(|p| p.reason == Reason::AView && !p.buying);
    view_buying && view_selling
}

/// B3, B2.a: what a naked seller is carrying — **short a jump**: small regular income, large sudden
/// loss, which is why its capital and margin matter more than its mark. An unfunded credit exposure,
/// and it must be capitalised as one.
pub fn unfunded_exposure(book: &[(Participant, Contract)]) -> f64 {
    book.iter()
        .filter(|(p, _)| !p.buying && p.naked)
        .map(|(_, c)| c.notional)
        .sum()
}

/// E3: **the net notional per reference entity is a real number and a real concentration, knowable
/// only by adding up the contracts.** Law 19: a walk over the rows, never a stored total.
pub fn net_notional(on: PartyId, contracts: &[Contract]) -> f64 {
    contracts
        .iter()
        .filter(|c| c.on.entity == on)
        .map(|c| c.notional)
        .sum()
}

/// A5, A5.a: **the index** — a fixed basket of names traded as one line, which is how broad credit
/// risk is actually bought and sold. The basket is a SERIES: names fixed at the roll, and **a name's
/// event settles its weight once for every contract on the line, the line running on with the
/// survivors.**
#[derive(Clone, Debug)]
pub struct Series {
    pub roll: u32,
    /// Fixed at the roll. Each is a name and the weight it carries.
    pub names: Vec<(PartyId, f64)>,
    /// The names whose event has already settled. They are gone from the line, not from the series.
    pub settled: Vec<PartyId>,
}

impl Series {
    /// A5.a: the line runs on with the survivors, carrying the weight that is left.
    pub fn surviving_weight(&self) -> f64 {
        self.names
            .iter()
            .filter(|(n, _)| !self.settled.contains(n))
            .map(|(_, w)| w)
            .sum()
    }

    /// What one name's event settles, once, for every contract on the line.
    pub fn settles(&mut self, name: PartyId, notional: f64, r: &Recovery) -> Option<f64> {
        let weight = self.names.iter().find(|(n, _)| *n == name).map(|(_, w)| *w)?;
        if self.settled.contains(&name) {
            // Once. A second settlement of the same name would pay the same loss twice.
            return None;
        }
        self.settled.push(name);
        Some(notional * weight * (1.0 - r.fetched))
    }
}

/// C3, C3.a: **the basis** between the swap spread and the cash bond's spread over the risk-free
/// curve — a CONSEQUENCE of funding cost, deliverability and who can trade which, measured at every
/// tenor both books print, and never set. C3.b: a persistently large basis is a finding about one of
/// the two markets, and it must be visible.
pub fn basis(swap_spread: f64, bond_spread: f64) -> f64 {
    swap_spread - bond_spread
}

/// D5: **protection paid equals protection received, and the net effect across the whole world of a
/// default is a TRANSFER, never a change in total loss.**
///
/// What is worth measuring is the only part that can fail. `Paid` carries one amount with a payer and
/// a payee on it, so paid-equals-received holds by construction and comparing it with itself would be
/// a VERIFY that cannot fail. **The shortfall is what breaks D5**: protection the seller could not
/// find is not a transfer, it is a second loss, and it has a holder (XI-1, E4).
pub fn shortfall(paid: &[Paid], terms: usize) -> Option<f64> {
    let short: f64 = paid.iter().map(|p| p.short).sum();
    let moved: f64 = paid.iter().map(|p| p.paid).sum();
    if short <= crate::num::dust(terms, &[moved]) {
        return None;
    }
    Some(short)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn reference() -> Reference {
        Reference { entity: party(9), can_fail: true }
    }

    fn contract(buyer: u32, seller: u32, notional: f64) -> Contract {
        Contract {
            buyer: party(buyer),
            seller: party(seller),
            on: reference(),
            notional,
            spread: 0.02,
            tenor_years: 5.0,
            triggered: false,
        }
    }

    fn recovery(fetched: f64) -> Recovery {
        Recovery { of: party(9), fetched, workout_closed: true }
    }

    #[test]
    fn the_premium_is_real_money_and_it_stops_on_the_event() {
        // A2: a periodic payment between two named parties (Law 5), and nothing once it has fired.
        let c = contract(1, 2, 10_000.0);
        assert_eq!(c.premium(4.0), Some((party(1), party(2), 50.0)));
        let fired = Contract { triggered: true, ..c };
        assert!(fired.premium(4.0).is_none());
    }

    #[test]
    fn the_payoff_is_par_minus_what_the_obligations_actually_fetched() {
        // A1.b, D2, D2.a: no fixed recovery rate — a constant recovery makes the payoff a constant
        // and turns a credit derivative into an interest-rate instrument. Two workouts, two payoffs.
        let c = contract(1, 2, 10_000.0);
        assert_eq!(owed_on_event(&c, &recovery(0.4)), 6_000.0);
        // Law 7: 1 - 0.7 is not 0.3 in binary, so this one is asserted against its DUST — derived
        // from the magnitudes that went through the subtraction, never a band anybody chose.
        let dear = owed_on_event(&c, &recovery(0.7));
        assert!((dear - 3_000.0).abs() <= crate::num::dust(2, &[10_000.0, 3_000.0]));
    }

    #[test]
    fn protection_that_pays_without_a_payer_is_not_protection() {
        // E4, E2: the seller's ability to pay is part of the instrument, and wrong-way risk is this
        // shortfall arriving exactly when the protection was most needed.
        let c = contract(1, 2, 10_000.0);
        let good = pays_out(&c, &recovery(0.4), 50_000.0);
        assert_eq!(good.paid, 6_000.0);
        assert_eq!(good.short, 0.0);
        let broke = pays_out(&c, &recovery(0.4), 1_500.0);
        assert_eq!(broke.paid, 1_500.0);
        assert_eq!(broke.short, 4_500.0);
        // D5: with a shortfall it is no longer a transfer — it is a second loss, with a holder.
        assert!(shortfall(&[good], 2).is_none());
        assert_eq!(shortfall(&[broke], 2), Some(4_500.0));
    }

    #[test]
    fn the_implied_probability_is_read_from_the_spread_and_never_fed_to_it() {
        // C2, XI-13: a probability computed from the accounts and fed to every seller means the
        // market cannot disagree with the accounting model. There is no such function in this file.
        let tight = implied(0.01, &recovery(0.4)).unwrap();
        let wide = implied(0.05, &recovery(0.4)).unwrap();
        assert!(wide > tight);
        // Obligations that paid in full imply nothing about default.
        assert!(implied(0.01, &recovery(1.0)).is_none());
    }

    #[test]
    fn one_tenor_is_not_a_term_structure() {
        // A1.d: a single tenor means the model has no term structure of credit anywhere.
        let curve = Curve::new(
            reference(),
            vec![
                Point { tenor_years: 1.0, spread: 0.012 },
                Point { tenor_years: 5.0, spread: 0.020 },
                Point { tenor_years: 10.0, spread: 0.024 },
            ],
        );
        assert_eq!(curve.at(5.0), Some(0.020));
        assert!(curve.at(7.0).is_none());
    }

    #[test]
    #[should_panic(expected = "is not a term structure")]
    fn a_curve_of_one_point_is_refused() {
        Curve::new(reference(), vec![Point { tenor_years: 5.0, spread: 0.02 }]);
    }

    #[test]
    #[should_panic(expected = "nobody can observe failing")]
    fn there_is_no_protection_on_an_entity_that_cannot_fail() {
        // A4.a, XI-3: a central bank, or a treasury in the money it issues, cannot fail — and
        // protection on one is protection on nothing.
        Curve::new(
            Reference { entity: party(9), can_fail: false },
            vec![
                Point { tenor_years: 1.0, spread: 0.01 },
                Point { tenor_years: 5.0, spread: 0.02 },
            ],
        );
    }

    #[test]
    fn a_book_of_hedgers_on_both_sides_does_not_clear() {
        // B5, XI-13: its spread would be a function of regulatory gaps and never of a view, and a
        // period in which neither gap binds would not open the book at all.
        let hedgers = [
            Participant { who: party(1), reason: Reason::Hedging, buying: true, naked: false },
            Participant { who: party(2), reason: Reason::Hedging, buying: false, naked: false },
        ];
        assert!(!can_clear(&hedgers));
        // A view on one side is not enough either — the other side is still only closing a gap.
        let half = [
            Participant { who: party(3), reason: Reason::AView, buying: true, naked: true },
            Participant { who: party(2), reason: Reason::Hedging, buying: false, naked: false },
        ];
        assert!(!can_clear(&half));
        let whole = [
            Participant { who: party(3), reason: Reason::AView, buying: true, naked: true },
            Participant { who: party(4), reason: Reason::AView, buying: false, naked: true },
            Participant { who: party(1), reason: Reason::Hedging, buying: true, naked: false },
        ];
        assert!(can_clear(&whole));
    }

    #[test]
    fn a_naked_seller_is_an_unfunded_credit_exposure_and_is_countable_as_one() {
        // B3, B2.a: it is short a jump — small regular income, large sudden loss — which is why its
        // capital and margin matter more than its mark.
        let naked = Participant { who: party(4), reason: Reason::AView, buying: false, naked: true };
        let covered = Participant { who: party(5), reason: Reason::Hedging, buying: false, naked: false };
        let book = [(naked, contract(1, 4, 10_000.0)), (covered, contract(1, 5, 7_000.0))];
        assert_eq!(unfunded_exposure(&book), 10_000.0);
    }

    #[test]
    fn the_net_notional_on_a_name_is_knowable_only_by_adding_up_the_contracts() {
        // E3: a real number and a real concentration, and Law 19 says it is a walk over the rows.
        let other = Reference { entity: party(8), can_fail: true };
        let contracts = [
            contract(1, 2, 10_000.0),
            contract(3, 4, 5_000.0),
            Contract { on: other, ..contract(1, 2, 90_000.0) },
        ];
        assert_eq!(net_notional(party(9), &contracts), 15_000.0);
        assert_eq!(net_notional(party(8), &contracts), 90_000.0);
        assert_eq!(net_notional(party(7), &contracts), 0.0);
    }

    #[test]
    fn a_names_event_settles_its_weight_once_and_the_line_runs_on_with_the_survivors() {
        // A5.a: names fixed at the roll; the event settles the weight once for every contract on
        // the line, and a second settlement of the same name would pay the same loss twice.
        let mut s = Series {
            roll: 21,
            names: vec![(party(9), 0.008), (party(8), 0.008), (party(7), 0.008)],
            settled: Vec::new(),
        };
        let before = s.surviving_weight();
        let paid = s.settles(party(9), 1_000_000.0, &recovery(0.4)).unwrap();
        assert_eq!(paid, 1_000_000.0 * 0.008 * 0.6);
        assert!(s.surviving_weight() < before);
        assert!(s.settles(party(9), 1_000_000.0, &recovery(0.4)).is_none());
        // A name that was never in the series settles nothing.
        assert!(s.settles(party(6), 1_000_000.0, &recovery(0.4)).is_none());
    }

    #[test]
    fn the_basis_is_measured_at_every_tenor_both_books_print() {
        // C3, C3.a, C3.b: a consequence of funding cost, deliverability and who can trade which —
        // never set, and a persistently large one is a finding about one of the two markets.
        assert!(basis(0.020, 0.018).abs() > 0.0);
        assert_eq!(basis(0.020, 0.020), 0.0);
    }

    #[test]
    #[should_panic(expected = "does not settle another's contract")]
    fn a_recovery_on_one_name_does_not_settle_anothers_contract() {
        let other = Recovery { of: party(8), fetched: 0.4, workout_closed: true };
        owed_on_event(&contract(1, 2, 10_000.0), &other);
    }
}
