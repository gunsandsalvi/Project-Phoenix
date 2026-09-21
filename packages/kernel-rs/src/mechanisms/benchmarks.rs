//! THE BENCHMARKS: the things everything else prices off must themselves be PRICES.
//!
//! @spec XI-7 · XI-12 · 22 A1 · 22 A1.a · 22 A2 · 22 A3 · 22 A4 · 22 B1 · 22 B2 · 22 B2.a · 22 B3 ·
//! @spec 22 B4 · 22 C1 · 22 C2 · 22 C2.a · 22 C3 · 22 C4 · 22 D1 · 22 D2 · 22 D3 · 22 D3.a · 22 D3.b ·
//! @spec 22 D4 · 22 D4.a · 22 D5 · 22 D5.a · 22 E1 · 22 E2 · 22 E3 · Law 3, Law 4, Law 8, Law 19 ·
//! @spec Appendix B

use crate::ids::InstrumentId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::prices::{Print, Provenance};

/// One member of an index, with the weight it carries.
#[derive(Clone, Copy, Debug)]
pub struct Constituent {
    pub what: InstrumentId,
    pub weight: f64,
}

#[derive(Clone, Debug)]
pub struct Index {
    pub of: Vec<Constituent>,
}

impl Index {
    /// Built from what the registry declared, so the basket has one writer.
    pub fn declared(constituents: &[(u32, f64)]) -> Index {
        Index {
            of: constituents
                .iter()
                .map(|(what, weight)| Constituent { what: InstrumentId::at(*what), weight: *weight })
                .collect(),
        }
    }

    /// The level, from the constituents' prints in that period.
    pub fn level_at(&self, period: u32, prints: &[Print]) -> Option<f64> {
        let mut total = 0.0;
        for c in &self.of {
            let found = prints
                .iter()
                .find(|p| p.instrument == c.what && p.period == period)?;
            total += found.price * c.weight;
        }
        Some(total)
    }

    /// The terms and the magnitude a reader is entitled to dust against, published with the level,
    /// because a tolerance derived from one side of a comparison is derived from the wrong thing.
    pub fn terms(&self) -> usize {
        self.of.len()
    }

    /// The index is never an input to its constituents.
    pub fn contains(&self, what: InstrumentId) -> bool {
        self.of.iter().any(|c| c.what == what)
    }
}

/// The levels an index actually had, period by period — its REAL history, which is the constituents'
/// own prints and nothing else.
pub fn history(index: &Index, periods: &[u32], prints: &[Print]) -> Vec<(u32, f64)> {
    let mut out = Vec::with_capacity(periods.len());
    for &p in periods {
        if let Some(level) = index.level_at(p, prints) {
            out.push((p, level));
        }
    }
    out
}

/// A covariance against a random-walk opening history is a covariance against noise, and a
/// covariance against noise is a discount rate wherever a beta is used.
pub fn covariance(a: &[(u32, f64)], b: &[(u32, f64)]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    for (x, y) in a.iter().zip(b.iter()) {
        assert!(x.0 == y.0, "XI-7: a covariance across different periods is not a covariance");
    }
    let left: Vec<f64> = a.iter().map(|x| x.1).collect();
    let right: Vec<f64> = b.iter().map(|y| y.1).collect();
    crate::num::covariance(&left, &right)
}

/// The floating benchmark is a transacted rate.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fixing {
    pub rate: f64,
    pub period: u32,
}

/// A posted policy rate is not a benchmark.
pub fn fix(overnight: &Print) -> Option<Fixing> {
    match overnight.provenance {
        Provenance::Cleared => Some(Fixing { rate: overnight.price, period: overnight.period }),
        Provenance::Carried | Provenance::Seeded => None,
    }
}

/// Weights come from something real — market capitalisation, amount outstanding, equal weight
/// — and the choice is stated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weighing {
    MarketCapitalisation,
    AmountOutstanding,
    Equal,
}

/// A unit and a base — a level is meaningless without them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Base {
    pub level: f64,
    pub period: u32,
}

/// The constituent set changes — firms enter and leave, bonds mature — and a change
/// must not create a jump in the level: the index is CHAINED across the rebalance, because the
/// level's continuity is the whole basis of a return series.
pub fn chain(old_basket_that_day: Option<f64>, new_basket_that_day: Option<f64>) -> Option<f64> {
    let old = old_basket_that_day?;
    let new = new_basket_that_day?;
    if new <= 0.0 {
        return None;
    }
    Some(old / new)
}

/// A corporate action is handled explicitly — a split changes shares and price together and
/// must not change the level.
pub fn on_split(c: &Constituent, split_factor: f64) -> Constituent {
    assert!(split_factor > 0.0, "22 B3: a split into no shares is not a split");
    Constituent { weight: c.weight * split_factor, ..*c }
}

/// The index return over a period equals the weighted return of its constituents, to
/// arithmetic dust — and a divergence is a defect in the READ, not a market event.
pub fn divergence(index_return: f64, constituent_returns: &[(f64, f64)], terms: usize) -> Option<f64> {
    let weighted: f64 = constituent_returns.iter().map(|(w, r)| w * r).sum();
    let off = index_return - weighted;
    if off.abs() <= crate::num::dust(terms, &[index_return, weighted]) {
        return None;
    }
    Some(off)
}

/// A fund tracks the index, so a change in it is a REAL FORCED TRADE by every tracker,
/// at the same time — and inclusion or exclusion is therefore visible in the constituent's price as
/// a CONSEQUENCE, never as an applied bump.
pub fn trackers_must_trade(weight: f64, tracking_assets: &[f64]) -> Vec<f64> {
    tracking_assets.iter().map(|a| a * weight).collect()
}

/// XI-7, 22 D4, D4.a: producer prices and consumer prices are two indices, and this is what having
/// two buys.
pub fn squeeze(producer_now: f64, producer_before: f64, consumer_now: f64, consumer_before: f64) -> f64 {
    assert!(
        producer_before > 0.0 && consumer_before > 0.0,
        "XI-7: a change with no level behind it is not a change (Law 8)"
    );
    (producer_now / producer_before) - (consumer_now / consumer_before)
}


/// THE FLOATING BENCHMARK IS A TRANSACTED RATE, OR IT IS NOTHING.
pub struct Fixes {
    /// The overnight book.
    pub on: Option<crate::ids::MarketId>,
    pub says: u32,
}

impl Mechanism for Fixes {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let Some(book) = self.on else { return };
        let Some(line) = ctx.subject_of(book) else { return };
        let Some(print) = ctx.prints().latest(line, ctx.period()) else { return };
        // Only a CLEARED print is a fixing.
        let Some(fixing) = fix(&print) else { return };
        ctx.say(
            self.says,
            &[],
            &[(0, Value::Num(fixing.rate)), (1, Value::Num(f64::from(fixing.period)))],
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CurrencyCode, MarketId};
    use crate::prices::QuotedAs;

    fn instrument(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn print(n: u32, period: u32, price: f64, provenance: Provenance) -> Print {
        Print {
            instrument: instrument(n),
            market: MarketId::at(0),
            period,
            price,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance,
        }
    }

    fn basket() -> Index {
        Index {
            of: vec![
                Constituent { what: instrument(1), weight: 0.6 },
                Constituent { what: instrument(2), weight: 0.4 },
            ],
        }
    }

    #[test]
    fn the_level_is_computed_from_the_constituents_and_stored_nowhere() {
        // No stored index level.
        let prints = [
            print(1, 3, 100.0, Provenance::Cleared),
            print(2, 3, 50.0, Provenance::Cleared),
        ];
        let level = basket().level_at(3, &prints).unwrap();
        let dust = 2.0 * f64::EPSILON * (60.0 + 20.0);
        assert!((level - 80.0).abs() <= dust);
    }

    #[test]
    fn an_index_whose_constituent_did_not_print_has_no_level_that_period() {
        // Carrying the last level under this period's date is a number claiming a period it does not
        // belong to.
        let prints = [print(1, 3, 100.0, Provenance::Cleared)];
        assert!(basket().level_at(3, &prints).is_none());
    }

    #[test]
    fn the_index_knows_its_members_and_a_member_knows_nothing_of_the_index() {
        // No index that inputs to its constituents.
        let b = basket();
        assert!(b.contains(instrument(1)));
        assert!(!b.contains(instrument(9)));
    }

    #[test]
    fn a_covariance_needs_a_history_the_index_actually_had() {
        // A covariance measured against a made-up opening history is a covariance against noise, and
        // a covariance against noise is a discount rate wherever a beta is used.
        let one = [(1u32, 100.0)];
        assert!(covariance(&one, &one).is_none());
        let a = [(1u32, 100.0), (2, 110.0), (3, 90.0)];
        let with_it = [(1u32, 50.0), (2, 56.0), (3, 44.0)];
        let against_it = [(1u32, 50.0), (2, 44.0), (3, 56.0)];
        assert!(covariance(&a, &with_it).unwrap() > 0.0);
        assert!(covariance(&a, &against_it).unwrap() < 0.0);
    }

    #[test]
    fn the_history_is_the_constituents_own_prints_and_the_gaps_are_gaps() {
        // Read the source.
        let prints = [
            print(1, 1, 100.0, Provenance::Cleared),
            print(2, 1, 50.0, Provenance::Cleared),
            print(1, 2, 105.0, Provenance::Cleared),
            // Nothing printed for constituent 2 in period 2.
            print(1, 3, 110.0, Provenance::Cleared),
            print(2, 3, 55.0, Provenance::Cleared),
        ];
        let h = history(&basket(), &[1, 2, 3], &prints);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].0, 1);
        assert_eq!(h[1].0, 3);
    }

    #[test]
    fn a_floating_coupon_fixes_on_a_transacted_rate_and_not_on_a_posted_one() {
        // A posted policy rate is not a benchmark.
        let transacted = print(7, 4, 0.031, Provenance::Cleared);
        assert_eq!(fix(&transacted), Some(Fixing { rate: 0.031, period: 4 }));
        // A book that ran and had nothing cross in it did not transact this period.
        assert!(fix(&print(7, 4, 0.031, Provenance::Carried)).is_none());
        // And the world's opening level is a primitive that dies at the seed, not a fixing.
        assert!(fix(&print(7, 0, 0.031, Provenance::Seeded)).is_none());
    }

    #[test]
    fn two_indices_can_show_a_margin_squeeze_and_one_wearing_both_names_cannot() {
        // Input prices rising faster than output prices is most of what a cost shock does to a firm,
        // and it is a DIFFERENCE between two indices — invisible to a single one.
        let squeezed = squeeze(112.0, 100.0, 103.0, 100.0);
        assert!(squeezed > 0.0);
        let relieved = squeeze(101.0, 100.0, 108.0, 100.0);
        assert!(relieved < 0.0);
        // One index wearing both names is this case, and it reads zero in every world.
        let both_names = squeeze(112.0, 100.0, 112.0, 100.0);
        assert!(both_names.abs() <= 4.0 * f64::EPSILON * 112.0 / 100.0);
    }

    #[test]
    fn a_rebalance_is_chained_so_the_level_does_not_jump() {
        // The level's continuity is the whole basis of a return series, and a constituent
        // joining or leaving is not a return.
        let factor = chain(Some(80.0), Some(100.0)).unwrap();
        assert_eq!(factor, 0.8);
        assert_eq!(100.0 * factor, 80.0);
        // A rebalance cannot be chained across a day the index did not exist.
        assert!(chain(None, Some(100.0)).is_none());
        assert!(chain(Some(80.0), None).is_none());
        assert!(chain(Some(80.0), Some(0.0)).is_none());
    }

    #[test]
    fn a_split_changes_shares_and_price_together_and_does_not_change_the_level() {
        // Handled explicitly.
        let c = Constituent { what: instrument(1), weight: 0.6 };
        let after = on_split(&c, 2.0);
        assert_eq!(after.weight, 1.2);
        // Price halves, weight doubles: the contribution is the same number.
        let before_level = 100.0 * c.weight;
        let after_level = 50.0 * after.weight;
        assert!((before_level - after_level).abs() <= crate::num::dust(2, &[before_level, after_level]));
    }

    #[test]
    fn a_divergence_between_the_index_and_its_constituents_is_a_defect_in_the_read() {
        // The index return equals the weighted return of its constituents, to arithmetic
        // dust — and a divergence is NOT a market event.
        let constituents = [(0.6, 0.05), (0.4, -0.02)];
        assert!(divergence(0.022, &constituents, 3).is_none());
        let off = divergence(0.040, &constituents, 3).unwrap();
        // Asserted against its dust, not written out to its last binary digit.
        assert!((off - 0.018).abs() <= crate::num::dust(3, &[0.040, 0.022]));
    }

    #[test]
    fn a_change_in_the_index_is_a_real_forced_trade_by_every_tracker_at_the_same_time() {
        // Inclusion is visible in the constituent's price as a CONSEQUENCE of those
        // orders clearing — never as an applied bump.
        let trades = trackers_must_trade(0.03, &[1_000_000.0, 250_000.0]);
        assert_eq!(trades, vec![30_000.0, 7_500.0]);
    }

    #[test]
    fn the_weighting_choice_and_the_base_are_stated_because_an_index_nobody_can_reproduce_is_not_one() {
        // All three of rule, set and weights are public, and a level is meaningless
        // without its unit and base.
        assert_ne!(Weighing::MarketCapitalisation, Weighing::Equal);
        let b = Base { level: 100.0, period: 0 };
        assert_eq!(b.level, 100.0);
    }

    #[test]
    #[should_panic(expected = "is not a split")]
    fn a_split_into_no_shares_is_not_a_split() {
        on_split(&Constituent { what: instrument(1), weight: 0.6 }, 0.0);
    }

    #[test]
    #[should_panic(expected = "is not a change")]
    fn a_displayed_change_with_no_level_behind_it_is_a_lie() {
        squeeze(112.0, 0.0, 103.0, 100.0);
    }
}
