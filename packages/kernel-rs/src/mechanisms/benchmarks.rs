//! THE BENCHMARKS: the things everything else prices off must themselves be PRICES.
//!
//! @spec XI-7 · XI-12 · 22 A1 · 22 A1.a · 22 A2 · 22 A3 · 22 A4 · 22 B1 · 22 B2 · 22 B2.a · 22 B3 ·
//! @spec 22 B4 · 22 C1 · 22 C2 · 22 C2.a · 22 C3 · 22 C4 · 22 D1 · 22 D2 · 22 D3 · 22 D3.a · 22 D3.b ·
//! @spec 22 D4 · 22 D4.a · 22 D5 · 22 D5.a · 22 E1 · 22 E2 · 22 E3 · Law 3, Law 4, Law 8, Law 19 ·
//! @spec Appendix B

use crate::calendar::{Convention, Week};
use crate::ids::InstrumentId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::prices::{Print, Provenance};
use crate::registry::IndexId;

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

/// The declared consumer basket includes both traded goods and shelter actually let this week.
/// Rent is an observed contract price in the journal rather than an invented instrument print.
#[derive(Clone, Debug)]
pub struct ConsumerBasket {
    pub goods: Index,
    pub rent_kind: u32,
    pub rent_key: u32,
    pub rent_weight: f64,
}

fn consumer_level(goods: f64, observed_rents: &[f64], rent_weight: f64) -> Option<f64> {
    Some(goods + crate::num::mean(observed_rents)? * rent_weight)
}

impl ConsumerBasket {
    pub fn level_at(
        &self,
        week: u32,
        prints: &crate::prices::Prints,
        journal: &crate::journal::Journal,
    ) -> Option<f64> {
        let mut goods = 0.0;
        for constituent in &self.goods.of {
            let print = prints.of_line(constituent.what, week)?;
            if print.week != week {
                return None;
            }
            goods += print.price * constituent.weight;
        }
        let rents: Vec<f64> = journal
            .of_kind(self.rent_kind)
            .iter()
            .filter(|row| journal.period_of(**row) == week)
            .filter_map(|row| match journal.says(*row, self.rent_key) {
                Some(Value::Num(rent)) => Some(rent),
                _ => None,
            })
            .collect();
        consumer_level(goods, &rents, self.rent_weight)
    }
}

/// Publish the current consumer basket only when all declared goods and an observed rent exist.
pub struct ConsumerPrices {
    pub basket: ConsumerBasket,
    pub says: u32,
}

impl Mechanism for ConsumerPrices {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        if let Some(level) = self
            .basket
            .level_at(ctx.week(), ctx.prints(), ctx.journal())
        {
            ctx.say(self.says, &[], &[(0, Value::Num(level))], true);
        }
    }
}

impl Index {
    /// Built from what the registry declared, so the basket has one writer.
    pub fn declared(constituents: &[(u32, f64)]) -> Index {
        Index {
            of: constituents
                .iter()
                .map(|(what, weight)| Constituent {
                    what: InstrumentId::at(*what),
                    weight: *weight,
                })
                .collect(),
        }
    }

    /// The level, from the constituents' prints in that week.
    pub fn level_at(&self, week: u32, prints: &[Print]) -> Option<f64> {
        let mut total = 0.0;
        for c in &self.of {
            let found = prints
                .iter()
                .find(|p| p.instrument == c.what && p.week == week)?;
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

/// Index definitions live in the registry; their levels do not. This reader freezes the
/// observation week before touching a constituent and publishes only a complete current basket.
pub struct PublishedIndices {
    pub kind: u32,
    pub at_index: u32,
    pub at_subject: u32,
    pub at_level: u32,
    pub at_observed: u32,
}

impl Mechanism for PublishedIndices {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let observed = ctx.week();
        let mut levels = Vec::new();
        for row in 0..ctx.registry().indices() as u32 {
            let id = IndexId::at(row);
            let definition = Index::declared(ctx.registry().index_constituents(id));
            let mut prints = Vec::with_capacity(definition.of.len());
            for member in &definition.of {
                let Some(print) = ctx.prints().of_line(member.what, observed) else {
                    prints.clear();
                    break;
                };
                if print.week != observed {
                    prints.clear();
                    break;
                }
                prints.push(print);
            }
            if let Some(level) = definition.level_at(observed, &prints) {
                levels.push((id, ctx.registry().index_subject(id), level));
            }
        }
        for (id, subject, level) in levels {
            ctx.say(
                self.kind,
                &[],
                &[
                    (self.at_index, Value::Num(f64::from(id.0))),
                    (self.at_subject, Value::Num(f64::from(subject.code()))),
                    (self.at_level, Value::Num(level)),
                    (self.at_observed, Value::Num(f64::from(observed))),
                ],
                true,
            );
        }
    }
}

/// The levels an index actually had, week by week — its REAL history, which is the constituents'
/// own prints and nothing else.
pub fn history(index: &Index, weeks: &[u32], prints: &[Print]) -> Vec<(u32, f64)> {
    let mut out = Vec::with_capacity(weeks.len());
    for &p in weeks {
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
        assert!(
            x.0 == y.0,
            "XI-7: a covariance across different weeks is not a covariance"
        );
    }
    let left: Vec<f64> = a.iter().map(|x| x.1).collect();
    let right: Vec<f64> = b.iter().map(|y| y.1).collect();
    crate::num::covariance(&left, &right)
}

/// The floating benchmark is a transacted rate.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fixing {
    pub rate: f64,
    pub week: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CurveProvenance {
    Observed,
    Interpolated,
    Extrapolated,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CurvePoint {
    pub tenor_days: i64,
    pub rate: f64,
    pub provenance: CurveProvenance,
}

pub const SOVEREIGN_CURVE_TENORS_DAYS: [i64; 8] = [7, 30, 90, 180, 365, 730, 1_825, 3_650];

pub fn curve_at(observed: &[CurvePoint], tenor_days: i64) -> Option<CurvePoint> {
    if tenor_days <= 0 {
        return None;
    }
    let mut points = observed
        .iter()
        .copied()
        .filter(|point| point.provenance == CurveProvenance::Observed)
        .collect::<Vec<_>>();
    points.sort_by_key(|point| point.tenor_days);
    points.dedup_by_key(|point| point.tenor_days);
    if let Some(point) = points.iter().find(|point| point.tenor_days == tenor_days) {
        return Some(*point);
    }
    if points.len() < 2 {
        return None;
    }
    let (left, right, provenance) =
        match points.binary_search_by_key(&tenor_days, |point| point.tenor_days) {
            Ok(at) => return Some(points[at]),
            Err(0) => (points[0], points[1], CurveProvenance::Extrapolated),
            Err(at) if at == points.len() => (
                points[at - 2],
                points[at - 1],
                CurveProvenance::Extrapolated,
            ),
            Err(at) => (points[at - 1], points[at], CurveProvenance::Interpolated),
        };
    let weight =
        (tenor_days - left.tenor_days) as f64 / (right.tenor_days - left.tenor_days) as f64;
    Some(CurvePoint {
        tenor_days,
        rate: left.rate + weight * (right.rate - left.rate),
        provenance,
    })
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FundingComparison {
    pub tenor_days: i64,
    pub domestic_rate: f64,
    pub foreign_rate: f64,
    pub spot: f64,
    pub forward: f64,
    pub hedged_foreign_rate: f64,
}

pub fn compare_funding(
    domestic: CurvePoint,
    foreign: CurvePoint,
    spot: &Print,
    forward: &Print,
) -> Option<FundingComparison> {
    if domestic.tenor_days != foreign.tenor_days
        || spot.provenance != Provenance::Cleared
        || forward.provenance != Provenance::Cleared
        || spot.price <= 0.0
        || forward.price <= 0.0
    {
        return None;
    }
    Some(FundingComparison {
        tenor_days: domestic.tenor_days,
        domestic_rate: domestic.rate,
        foreign_rate: foreign.rate,
        spot: spot.price,
        forward: forward.price,
        hedged_foreign_rate: (1.0 + foreign.rate) * forward.price / spot.price - 1.0,
    })
}

fn observed_curve_point(
    print: &Print,
    on: Week,
    cashflows: &[(Week, f64)],
    convention: Convention,
) -> Option<CurvePoint> {
    if print.provenance != Provenance::Cleared || print.price <= 0.0 || cashflows.is_empty() {
        return None;
    }
    let mut tenor_days = None;
    for (due, _) in cashflows {
        let days = on.elapsed_days_until(*due);
        if tenor_days.is_none_or(|longest| days > longest) {
            tenor_days = Some(days);
        }
    }
    let tenor_days = tenor_days?;
    if tenor_days <= 0
        || cashflows
            .iter()
            .any(|(due, amount)| *due <= on || *amount < 0.0)
    {
        return None;
    }
    let present_value = |rate: f64| {
        cashflows
            .iter()
            .map(|(due, amount)| amount / (1.0 + rate).powf(convention.year_fraction(on, *due)))
            .sum::<f64>()
    };
    let (mut low, mut high) = (-0.999_999, 10.0);
    if present_value(low) < print.price || present_value(high) > print.price {
        return None;
    }
    for _ in 0..128 {
        let middle = (low + high) / 2.0;
        if present_value(middle) > print.price {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(CurvePoint {
        tenor_days,
        rate: (low + high) / 2.0,
        provenance: CurveProvenance::Observed,
    })
}

/// A posted policy rate is not a benchmark.
pub fn fix(weekly_funding: &Print) -> Option<Fixing> {
    match weekly_funding.provenance {
        Provenance::Cleared => Some(Fixing {
            rate: weekly_funding.price,
            week: weekly_funding.week,
        }),
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
    pub week: u32,
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
    assert!(
        split_factor > 0.0,
        "22 B3: a split into no shares is not a split"
    );
    Constituent {
        weight: c.weight * split_factor,
        ..*c
    }
}

/// The index return over a week equals the weighted return of its constituents, to
/// arithmetic dust — and a divergence is a defect in the READ, not a market event.
pub fn divergence(
    index_return: f64,
    constituent_returns: &[(f64, f64)],
    terms: usize,
) -> Option<f64> {
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
pub fn squeeze(
    producer_now: f64,
    producer_before: f64,
    consumer_now: f64,
    consumer_before: f64,
) -> f64 {
    assert!(
        producer_before > 0.0 && consumer_before > 0.0,
        "XI-7: a change with no level behind it is not a change (Law 8)"
    );
    (producer_now / producer_before) - (consumer_now / consumer_before)
}

/// THE FLOATING BENCHMARK IS A TRANSACTED RATE, OR IT IS NOTHING.
pub struct Fixes {
    /// The weekly_funding book.
    pub on: Option<crate::ids::MarketId>,
    pub says: u32,
    pub sovereign_says: u32,
}

impl Mechanism for Fixes {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        if let Some(fixing) = self
            .on
            .and_then(|book| ctx.subject_of(book))
            .and_then(|line| ctx.prints().of_line(line, ctx.week()))
            .and_then(|print| fix(&print))
        {
            ctx.say(
                self.says,
                &[],
                &[
                    (0, Value::Num(fixing.rate)),
                    (1, Value::Num(f64::from(fixing.week))),
                ],
                true,
            );
        }

        let today = ctx.today();
        let mut points = Vec::new();
        for row in 0..ctx.instruments().len() {
            let line = InstrumentId::at(row as u32);
            let issuer = ctx.instruments().issuer_of(line);
            if ctx.parties().kind_of(issuer) != crate::assembly::kinds::TREASURY {
                continue;
            }
            let Some(print) = ctx.prints().of_line(line, ctx.week()) else {
                continue;
            };
            let issued = ctx.instruments().issued_of(line);
            if issued <= 0.0 {
                continue;
            }
            let cashflows = ctx
                .schedules()
                .of_instrument(line)
                .iter()
                .map(|row| crate::stores::DueId(*row))
                .filter(|due| !ctx.schedules().paid(*due) && ctx.schedules().due(*due) > today)
                .map(|due| {
                    (
                        ctx.schedules().due(due),
                        ctx.schedules().amount(due) / issued,
                    )
                })
                .collect::<Vec<_>>();
            if let Some(point) =
                observed_curve_point(&print, today, &cashflows, Convention::Actual365)
            {
                points.push((issuer, line, point));
            }
        }
        for (issuer, line, point) in &points {
            ctx.say(
                self.sovereign_says,
                &[issuer.0, line.0],
                &[
                    (0, Value::Num(point.tenor_days as f64)),
                    (1, Value::Num(point.rate)),
                    (2, Value::Num(0.0)),
                    (3, Value::Num(0.0)),
                ],
                true,
            );
        }
        let issuers = points
            .iter()
            .map(|(issuer, _, _)| *issuer)
            .collect::<std::collections::BTreeSet<_>>();
        for issuer in issuers {
            let observed = points
                .iter()
                .filter(|(of, _, _)| *of == issuer)
                .map(|(_, _, point)| *point)
                .collect::<Vec<_>>();
            for tenor in SOVEREIGN_CURVE_TENORS_DAYS {
                if observed.iter().any(|point| point.tenor_days == tenor) {
                    continue;
                }
                let Some(point) = curve_at(&observed, tenor) else {
                    continue;
                };
                let source = match point.provenance {
                    CurveProvenance::Observed => 0.0,
                    CurveProvenance::Interpolated => 1.0,
                    CurveProvenance::Extrapolated => 2.0,
                };
                ctx.say(
                    self.sovereign_says,
                    &[issuer.0],
                    &[
                        (0, Value::Num(point.tenor_days as f64)),
                        (1, Value::Num(point.rate)),
                        (2, Value::Num(source)),
                        // Annual-effective is the sole published convention.
                        (3, Value::Num(0.0)),
                    ],
                    true,
                );
            }
        }
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

    fn print(n: u32, week: u32, price: f64, provenance: Provenance) -> Print {
        Print {
            instrument: instrument(n),
            market: MarketId::at(0),
            week,
            struck: week,
            price,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance,
        }
    }

    fn basket() -> Index {
        Index {
            of: vec![
                Constituent {
                    what: instrument(1),
                    weight: 0.6,
                },
                Constituent {
                    what: instrument(2),
                    weight: 0.4,
                },
            ],
        }
    }

    #[test]
    fn the_consumer_basket_uses_rent_observed_in_a_tenancy_crossing() {
        // Goods contribute 80 and the observed mean rent contributes 40 × 0.5.
        assert_eq!(consumer_level(80.0, &[30.0, 50.0], 0.5), Some(100.0));
        assert_eq!(consumer_level(80.0, &[], 0.5), None);
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
        // Carrying the last level under this week's date is a number claiming a week it does not
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
            // Nothing printed for constituent 2 in week 2.
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
        assert_eq!(
            fix(&transacted),
            Some(Fixing {
                rate: 0.031,
                week: 4
            })
        );
        // A book that ran and had nothing cross in it did not transact this week.
        assert!(fix(&print(7, 4, 0.031, Provenance::Carried)).is_none());
        // And the world's opening level is a primitive that dies at the seed, not a fixing.
        assert!(fix(&print(7, 0, 0.031, Provenance::Seeded)).is_none());
    }

    #[test]
    fn a_sovereign_curve_point_is_derived_from_a_cleared_price_and_dated_cashflows() {
        let on = crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
            year: 2026,
            month: 1,
            day: 1,
        });
        let maturity =
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2027,
                month: 1,
                day: 1,
            });
        let traded = print(7, 4, 0.95, Provenance::Cleared);
        let point =
            observed_curve_point(&traded, on, &[(maturity, 1.0)], Convention::Actual365).unwrap();
        assert_eq!(point.tenor_days, 364);
        assert_eq!(point.provenance, CurveProvenance::Observed);
        let repriced = 1.0 / (1.0 + point.rate).powf(364.0 / 365.0);
        assert!(
            (repriced - traded.price).abs() <= crate::num::dust(128, &[repriced, traded.price])
        );
        assert!(observed_curve_point(
            &print(7, 4, 0.95, Provenance::Carried),
            on,
            &[(maturity, 1.0)],
            Convention::Actual365,
        )
        .is_none());
    }

    #[test]
    fn sovereign_curve_operations_keep_their_provenance_and_leave_unsupported_tenors_absent() {
        let observed = [
            CurvePoint {
                tenor_days: 30,
                rate: 0.02,
                provenance: CurveProvenance::Observed,
            },
            CurvePoint {
                tenor_days: 90,
                rate: 0.04,
                provenance: CurveProvenance::Observed,
            },
        ];
        assert_eq!(curve_at(&observed, 30), Some(observed[0]));
        let interpolated = curve_at(&observed, 60).unwrap();
        assert_eq!(interpolated.provenance, CurveProvenance::Interpolated);
        assert!(
            (interpolated.rate - 0.03).abs() <= crate::num::dust(3, &[interpolated.rate, 0.03])
        );
        let extrapolated = curve_at(&observed, 180).unwrap();
        assert_eq!(extrapolated.provenance, CurveProvenance::Extrapolated);
        assert!(
            (extrapolated.rate - 0.07).abs() <= crate::num::dust(3, &[extrapolated.rate, 0.07])
        );
        assert!(curve_at(&observed[..1], 60).is_none());
        assert!(curve_at(&observed, 0).is_none());
    }

    #[test]
    fn foreign_funding_is_compared_at_one_tenor_through_cleared_spot_and_forward_fx() {
        let domestic = CurvePoint {
            tenor_days: 365,
            rate: 0.04,
            provenance: CurveProvenance::Observed,
        };
        let foreign = CurvePoint {
            tenor_days: 365,
            rate: 0.02,
            provenance: CurveProvenance::Observed,
        };
        let spot = print(10, 4, 1.20, Provenance::Cleared);
        let forward = print(11, 4, 1.23, Provenance::Cleared);
        let compared = compare_funding(domestic, foreign, &spot, &forward).unwrap();
        assert_eq!(compared.tenor_days, 365);
        assert_eq!(compared.spot, 1.20);
        assert_eq!(compared.forward, 1.23);
        let expected = (1.0 + 0.02) * 1.23 / 1.20 - 1.0;
        assert!(
            (compared.hedged_foreign_rate - expected).abs()
                <= crate::num::dust(4, &[compared.hedged_foreign_rate, expected])
        );
        assert!(compare_funding(
            domestic,
            CurvePoint {
                tenor_days: 180,
                ..foreign
            },
            &spot,
            &forward,
        )
        .is_none());
        assert!(compare_funding(
            domestic,
            foreign,
            &print(10, 4, 1.20, Provenance::Carried),
            &forward
        )
        .is_none());
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
        let c = Constituent {
            what: instrument(1),
            weight: 0.6,
        };
        let after = on_split(&c, 2.0);
        assert_eq!(after.weight, 1.2);
        // Price halves, weight doubles: the contribution is the same number.
        let before_level = 100.0 * c.weight;
        let after_level = 50.0 * after.weight;
        assert!(
            (before_level - after_level).abs() <= crate::num::dust(2, &[before_level, after_level])
        );
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
    fn the_weighting_choice_and_the_base_are_stated_because_an_index_nobody_can_reproduce_is_not_one(
    ) {
        // All three of rule, set and weights are public, and a level is meaningless
        // without its unit and base.
        assert_ne!(Weighing::MarketCapitalisation, Weighing::Equal);
        let b = Base {
            level: 100.0,
            week: 0,
        };
        assert_eq!(b.level, 100.0);
    }

    #[test]
    #[should_panic(expected = "is not a split")]
    fn a_split_into_no_shares_is_not_a_split() {
        on_split(
            &Constituent {
                what: instrument(1),
                weight: 0.6,
            },
            0.0,
        );
    }

    #[test]
    #[should_panic(expected = "is not a change")]
    fn a_displayed_change_with_no_level_behind_it_is_a_lie() {
        squeeze(112.0, 0.0, 103.0, 100.0);
    }
}
