//! CREDIT DEFAULT SWAPS: a running spread that CLEARS, a premium leg that is real money and stops on
//! the event, and a protection leg somebody must actually be able to pay.
//!
//! @spec 17 A1.a · 17 A1.b · 17 A1.c · 17 A1.d · 17 A2 · 17 A3 · 17 A4 · 17 A4.a · 17 A5 · 17 A5.a ·
//! @spec 17 A5.b · 17 B1 · 17 B2 · 17 B2.a · 17 B3 · 17 B5 · 17 C1 · 17 C2 · 17 C3 · 17 C3.a ·
//! @spec 17 C3.b · 17 D1 · 17 D2 · 17 D2.a · 17 D2.b · 17 D3 · 17 D4 · 17 D5 · 17 E2 · 17 E3 ·
//! @spec 17 E4 · XI-13 · XI-1 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::ids::PartyId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{agreed, standing};

/// The underlying is a named reference entity and its default event — not a price. A4: it must exist
/// in this world and be capable of defaulting, and A4.a forbids protection on an entity nobody can
/// observe failing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reference {
    pub entity: PartyId,
    /// Whether this party can fail at all. A central bank, or a treasury in the money it issues,
    /// cannot — and protection on one is protection on nothing.
    pub can_fail: bool,
}

/// One line of the curve — a tenor and the spread that cleared at it, in the contract's own currency
/// and periodicity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Point {
    pub tenor_years: f64,
    /// Cleared from the two sides' schedules. Never posted.
    pub spread: f64,
}

/// A term structure of credit and not one number.
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

/// The contract. A2: the premium leg is a real periodic payment, in cash, in the contract's
/// currency, and it stops on the event.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Contract {
    pub buyer: PartyId,
    pub seller: PartyId,
    pub on: Reference,
    pub notional: f64,
    pub spread: f64,
    pub tenor_years: f64,
    /// A triggered contract pays no premium, marks at its expected payoff, and holds past its own
    /// maturity until the workout closes.
    pub triggered: bool,
}

impl Contract {
    /// Real money, per period, from the buyer to the seller — and nothing once the event has fired
    /// (Law 5: two named sides).
    pub fn premium(&self, periods_per_year: f64) -> Option<(PartyId, PartyId, f64)> {
        if self.triggered {
            return None;
        }
        Some((self.buyer, self.seller, self.notional * self.spread / periods_per_year))
    }
}

/// A recovery determined by what the defaulted obligations are actually worth — an auction or a
/// realised workout, not an assumption. It carries where it came from, so a constant cannot be
/// passed where a recovery is wanted.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Recovery {
    pub of: PartyId,
    /// What the obligations fetched, per unit of par.
    pub fetched: f64,
    /// False while the workout is open — the contract then marks at its EXPECTED payoff and the
    /// settlement is a true-up when this closes.
    pub workout_closed: bool,
}

/// On the event the protection seller pays par minus recovery on the notional; otherwise nothing.
/// Real money from the seller to the buyer, and it can be large enough to fail the seller.
pub fn owed_on_event(c: &Contract, r: &Recovery) -> f64 {
    assert!(r.of == c.on.entity, "17 A1.b: a recovery on one name does not settle another's contract");
    c.notional * (1.0 - r.fetched)
}

/// What the seller actually delivered, and what it did not. No protection that pays without a payer:
/// the seller's ability to pay is part of the instrument, and E2's wrong-way risk is exactly the
/// case where this shortfall arrives when the protection was most needed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paid {
    pub from: PartyId,
    pub to: PartyId,
    pub paid: f64,
    /// What the buyer expected and did not receive. A loss with a holder.
    pub short: f64,
}

pub fn pays_out(c: &Contract, r: &Recovery, seller_can_find: f64) -> Paid {
    let owed = owed_on_event(c, r);
    let paid = if seller_can_find < owed { seller_can_find } else { owed };
    Paid { from: c.seller, to: c.buyer, paid, short: owed - paid }
}

/// The implied default probability is a READ from the cleared spread and the recovery — never an
/// input to either. `None` where the obligations paid in full: there is no loss to divide by, and
/// inventing one is the numeric default Appendix A refuses.
pub fn implied(spread: f64, r: &Recovery) -> Option<f64> {
    let loss_given_default = 1.0 - r.fetched;
    if loss_given_default <= 0.0 {
        return None;
    }
    Some(spread / loss_given_default)
}

/// Why a party is in this book. XI-13, B5: a speculative participant with a view is required on both
/// sides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// It holds the issuer's debt, or lends to it and cannot sell the loan.
    Hedging,
    /// It wants credit exposure without funding a bond, or B1/B2's view — it thinks the credit will
    /// deteriorate, or that the spread is too wide for the risk.
    AView,
    /// A dealer intermediates, and its book is rarely flat.
    Dealer,
}

#[derive(Clone, Copy, Debug)]
pub struct Participant {
    pub who: PartyId,
    pub reason: Reason,
    pub buying: bool,
    /// Naked positions are possible on both sides and they are how the market gets liquid — but a
    /// naked seller is an unfunded credit exposure and must be capitalised as one.
    pub naked: bool,
}

/// A book of hedgers on both sides clears at a function of regulatory gaps and never of a view. A
/// speculative participant with a view is required on both sides.
pub fn can_clear(book: &[Participant]) -> bool {
    let view_buying = book.iter().any(|p| p.reason == Reason::AView && p.buying);
    let view_selling = book.iter().any(|p| p.reason == Reason::AView && !p.buying);
    view_buying && view_selling
}

/// What a naked seller is carrying — short a jump: small regular income, large sudden loss, which is
/// why its capital and margin matter more than its mark. An unfunded credit exposure, and it must be
/// capitalised as one.
pub fn unfunded_exposure(book: &[(Participant, Contract)]) -> f64 {
    book.iter()
        .filter(|(p, _)| !p.buying && p.naked)
        .map(|(_, c)| c.notional)
        .sum()
}

/// The net notional per reference entity is a real number and a real concentration, knowable only by
/// adding up the contracts. Law 19: a walk over the rows, never a stored total.
pub fn net_notional(on: PartyId, contracts: &[Contract]) -> f64 {
    contracts
        .iter()
        .filter(|c| c.on.entity == on)
        .map(|c| c.notional)
        .sum()
}

/// The index — a fixed basket of names traded as one line, which is how broad credit risk is
/// actually bought and sold. The basket is a SERIES: names fixed at the roll, and a name's event
/// settles its weight once for every contract on the line, the line running on with the survivors.
#[derive(Clone, Debug)]
pub struct Series {
    pub roll: u32,
    /// Fixed at the roll. Each is a name and the weight it carries.
    pub names: Vec<(PartyId, f64)>,
    /// The names whose event has already settled. They are gone from the line, not from the series.
    pub settled: Vec<PartyId>,
}

impl Series {
    /// The line runs on with the survivors, carrying the weight that is left.
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

/// The basis between the swap spread and the cash bond's spread over the risk-free curve — a
/// CONSEQUENCE of funding cost, deliverability and who can trade which, measured at every tenor both
/// books print, and never set. C3.b: a persistently large basis is a finding about one of the two
pub fn basis(swap_spread: f64, bond_spread: f64) -> f64 {
    swap_spread - bond_spread
}

/// Protection paid equals protection received, and the net effect across the whole world of a
/// default is a TRANSFER, never a change in total loss.
pub fn shortfall(paid: &[Paid], terms: usize) -> Option<f64> {
    let short: f64 = paid.iter().map(|p| p.short).sum();
    let moved: f64 = paid.iter().map(|p| p.paid).sum();
    if short <= crate::num::dust(terms, &[moved]) {
        return None;
    }
    Some(short)
}

// §19 RUNS HERE. `Protection` was `running.rs:2663`, a hundred lines away from the arithmetic it is
// about: `owed_on_event`, `pays_out`, `can_clear` and `basis` are in this file and it called none

/// PROTECTION CLEARS BETWEEN TWO PARTIES WHO DISAGREE.
pub struct Protection {
    pub kind: u32,
    /// The premium runs for a tenor. A market CONVENTION.
    pub tenor: &'static str,
}

impl Mechanism for Protection {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let tenor = ctx.params().years(self.tenor);

        // What the estates of this world actually fetched, per unit of par. One read over the
        // claims, and a world where nothing has died has none.
        let mut fetched = 0.0;
        let mut owed = 0.0;
        for row in 0..ctx.claims().len() as u32 {
            let c = crate::stores::ClaimId(row);
            owed += ctx.claims().owed(c);
            fetched += ctx.claims().paid(c);
        }
        if owed <= 0.0 {
            return;
        }
        let recovery = fetched / owed;
        if recovery >= 1.0 {
            // The obligations paid in full. There is no loss to divide by, and inventing one is the
            // numeric default Appendix A refuses.
            return;
        }

        // Every view held on every name, by whom.
        let mut views: std::collections::HashMap<u32, Vec<(PartyId, f64)>> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let st = crate::stores::StandingId(row);
            if !ctx.standing().live(st) || ctx.standing().kind_of(st) != standing::OWN_VIEW {
                continue;
            }
            views
                .entry(ctx.standing().about(st).0)
                .or_default()
                .push((ctx.standing().held_by(st), ctx.standing().terms(st)[0]));
        }

        let mut struck: Vec<(PartyId, PartyId, PartyId, f64, f64)> = Vec::new();
        for (&on, holders) in &views {
            if holders.len() < 2 {
                // One opinion is not a market. A book that cleared on one view would be a
                // restatement of that view rather than a price.
                continue;
            }
            // What each party's own view says protection is worth to it — the probability it holds
            // times the loss given default it can actually observe. A buyer will pay up to its own
            let loss_given_default = 1.0 - recovery;
            let mut posted: Vec<(PartyId, f64)> = holders
                .iter()
                .map(|&(who, p)| (who, p * loss_given_default))
                .collect();
            posted.sort_by(|a, b| a.1.total_cmp(&b.1));
            let (seller, takes) = posted[0];
            let (buyer, pays) = posted[posted.len() - 1];
            if seller == buyer || pays <= takes {
                // No overlap: the most worried holder will not pay what the least worried will
                // take. That is a real outcome and nothing is invented to close it.
                continue;
            }
            // Clearing: the seller's level, because the sellers compete for the buyer's premium.
            struck.push((buyer, seller, PartyId(on), takes, tenor));
        }

        for (buyer, seller, on, spread, tenor) in struck {
            // It is a RELATION between two named parties — terms `[the name it is on, the spread,
            // the tenor]` — and neither side holds an instrument for it (§19 A1: a contract, not a
            ctx.agrees(crate::module::Agrees {
                kind: agreed::DERIVATIVE,
                one: buyer,
                other: seller,
                terms: vec![f64::from(on.0), spread, tenor],
                until: None,
            });
            ctx.say(self.kind, &[buyer.0, seller.0, on.0], &[(0, Value::Num(spread))], true);
        }
    }
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
        // A periodic payment between two named parties, and nothing once it has fired.
        let c = contract(1, 2, 10_000.0);
        assert_eq!(c.premium(4.0), Some((party(1), party(2), 50.0)));
        let fired = Contract { triggered: true, ..c };
        assert!(fired.premium(4.0).is_none());
    }

    #[test]
    fn the_payoff_is_par_minus_what_the_obligations_actually_fetched() {
        // No fixed recovery rate — a constant recovery makes the payoff a constant and turns a
        // credit derivative into an interest-rate instrument. Two workouts, two payoffs.
        let c = contract(1, 2, 10_000.0);
        assert_eq!(owed_on_event(&c, &recovery(0.4)), 6_000.0);
        // 1 - 0.7 is not 0.3 in binary, so this one is asserted against its DUST — derived from the
        // magnitudes that went through the subtraction, never a band anybody chose.
        let dear = owed_on_event(&c, &recovery(0.7));
        assert!((dear - 3_000.0).abs() <= crate::num::dust(2, &[10_000.0, 3_000.0]));
    }

    #[test]
    fn protection_that_pays_without_a_payer_is_not_protection() {
        // The seller's ability to pay is part of the instrument, and wrong-way risk is this
        // shortfall arriving exactly when the protection was most needed.
        let c = contract(1, 2, 10_000.0);
        let good = pays_out(&c, &recovery(0.4), 50_000.0);
        assert_eq!(good.paid, 6_000.0);
        assert_eq!(good.short, 0.0);
        let broke = pays_out(&c, &recovery(0.4), 1_500.0);
        assert_eq!(broke.paid, 1_500.0);
        assert_eq!(broke.short, 4_500.0);
        // With a shortfall it is no longer a transfer — it is a second loss, with a holder.
        assert!(shortfall(&[good], 2).is_none());
        assert_eq!(shortfall(&[broke], 2), Some(4_500.0));
    }

    #[test]
    fn the_implied_probability_is_read_from_the_spread_and_never_fed_to_it() {
        // A probability computed from the accounts and fed to every seller means the market cannot
        // disagree with the accounting model. There is no such function in this file.
        let tight = implied(0.01, &recovery(0.4)).unwrap();
        let wide = implied(0.05, &recovery(0.4)).unwrap();
        assert!(wide > tight);
        // Obligations that paid in full imply nothing about default.
        assert!(implied(0.01, &recovery(1.0)).is_none());
    }

    #[test]
    fn one_tenor_is_not_a_term_structure() {
        // A single tenor means the model has no term structure of credit anywhere.
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
        // A central bank, or a treasury in the money it issues, cannot fail — and protection on one
        // is protection on nothing.
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
        // Its spread would be a function of regulatory gaps and never of a view, and a period in
        // which neither gap binds would not open the book at all.
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
        // It is short a jump — small regular income, large sudden loss — which is why its capital
        // and margin matter more than its mark.
        let naked = Participant { who: party(4), reason: Reason::AView, buying: false, naked: true };
        let covered = Participant { who: party(5), reason: Reason::Hedging, buying: false, naked: false };
        let book = [(naked, contract(1, 4, 10_000.0)), (covered, contract(1, 5, 7_000.0))];
        assert_eq!(unfunded_exposure(&book), 10_000.0);
    }

    #[test]
    fn the_net_notional_on_a_name_is_knowable_only_by_adding_up_the_contracts() {
        // A real number and a real concentration, and Law 19 says it is a walk over the rows.
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
        // Names fixed at the roll; the event settles the weight once for every contract on the
        // line, and a second settlement of the same name would pay the same loss twice.
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
        // A consequence of funding cost, deliverability and who can trade which — never set, and a
        // persistently large one is a finding about one of the two markets.
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
