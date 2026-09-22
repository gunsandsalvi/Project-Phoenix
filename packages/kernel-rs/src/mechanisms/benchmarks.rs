//! THE BENCHMARKS: the things everything else prices off must themselves be PRICES.
//!
//! @spec XI-7 · XI-12 · 22 A1 · 22 A1.a · 22 A2 · 22 A3 · 22 A4 · 22 B1 · 22 B2 · 22 B2.a · 22 B3 ·
//! @spec 22 B4 · 22 C1 · 22 C2 · 22 C2.a · 22 C3 · 22 C4 · 22 D1 · 22 D2 · 22 D3 · 22 D3.a · 22 D3.b ·
//! @spec 22 D4 · 22 D4.a · 22 D5 · 22 D5.a · 22 E1 · 22 E2 · 22 E3 · Law 3, Law 4, Law 8, Law 19 ·
//! @spec Appendix B

use crate::calendar::{Convention, Week};
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Class;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::prices::{Print, Provenance};
use crate::registry::{
    Capitalisation, CreditQuality, IndexId, IndexScope, IndexSubject, Weighting,
};

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

/// 22 B1, Appendix A: shelter enters at what the households paying it actually pay, weighted by how
/// many of them there are. An unweighted mean of rents is one household's rent standing for a
/// sector's, which is a decision taken at an average.
fn shelter(let_at: &[(f64, u32)]) -> Option<f64> {
    let households: f64 = let_at.iter().map(|(_, weight)| f64::from(*weight)).sum();
    if households <= 0.0 {
        return None;
    }
    Some(
        let_at
            .iter()
            .map(|(rent, weight)| rent * f64::from(*weight))
            .sum::<f64>()
            / households,
    )
}

/// 22 A1: WHAT THIS SUBJECT IS AN INDEX OF. A share is equity, a bond still to mature is fixed
/// income, a good is a price basket — the line's own nature, not a list of ids.
fn admits(
    ctx: &MechanismContext<'_>,
    subject: IndexSubject,
    line: InstrumentId,
    week: u32,
) -> bool {
    let class = ctx.instruments().class_of(line);
    let alive = |line: InstrumentId| match ctx.instruments().matures_on(line) {
        Some(back) => back.0 > i64::from(week),
        None => false,
    };
    match subject {
        IndexSubject::Equity(_) => class == Class::Share,
        IndexSubject::FixedBond(quality) => {
            class == Class::Claim
                && alive(line)
                && ctx.instruments().negotiated_amount_of(line).is_none()
                && of_quality(ctx, line, quality)
        }
        // A tradable term loan is a claim struck bilaterally and then traded, which is what having
        // negotiated terms says about it.
        IndexSubject::TradableTermLoan(quality) => {
            class == Class::Claim
                && alive(line)
                && ctx.instruments().negotiated_amount_of(line).is_some()
                && of_quality(ctx, line, quality)
        }
        // A CDS is a contract and not a line, so nothing in the instrument store can be in a CDS
        // index until one is. The rule is here and the set is empty, which is not the same as a
        // list somebody left blank.
        IndexSubject::Cds(_) => false,
        IndexSubject::ConsumerPrices | IndexSubject::ProducerPrices => class == Class::Good,
    }
}

/// 22 B1: what one member carries, from the basis the index declared.
fn weighs(
    ctx: &MechanismContext<'_>,
    weights: Weighting,
    line: InstrumentId,
    week: u32,
) -> Option<f64> {
    let outstanding = || match ctx.instruments().issued_of(line) {
        units if units > 0.0 => Some(units),
        _ => None,
    };
    match weights {
        Weighting::Equal => Some(1.0),
        Weighting::AmountOutstanding => outstanding(),
        Weighting::Capitalisation => Some(outstanding()? * ctx.prints().of_line(line, week)?.price),
    }
}

/// The larger half of what qualifies is large and the rest is small, by capitalisation at the week
/// asked about. A member nobody has priced is in neither band, because its size is missing.
fn banded(
    ctx: &MechanismContext<'_>,
    of: Vec<Constituent>,
    band: Capitalisation,
    week: u32,
) -> Vec<Constituent> {
    if band == Capitalisation::All {
        return of;
    }
    let mut sized: Vec<(Constituent, f64)> = of
        .into_iter()
        .filter_map(|c| {
            let price = ctx.prints().of_line(c.what, week)?.price;
            Some((c, ctx.instruments().issued_of(c.what) * price))
        })
        .collect();
    sized.sort_by(|a, b| a.1.total_cmp(&b.1));
    let half = sized.len() / 2;
    let (small, large) = sized.split_at(half);
    let keep = match band {
        Capitalisation::Small => small,
        Capitalisation::Large | Capitalisation::All => large,
    };
    keep.iter().map(|(c, _)| *c).collect()
}

/// 22 B2: WHETHER THIS LINE IS IN THIS INDEX, asked of the line itself rather than of a list. A
/// bond that matured is not fixed income any more, and a line brought this week is in as soon as
/// it is brought.
fn in_scope(ctx: &MechanismContext<'_>, scope: IndexScope, line: InstrumentId) -> bool {
    match scope {
        IndexScope::Global => true,
        IndexScope::Currency(ccy) => ctx.instruments().ccy_of(line) == ccy,
    }
}

/// The lowest grade any agency has published on this line's issuer — the conventional rule, and
/// missing where nobody has rated it, which is not a band.
fn graded(ctx: &MechanismContext<'_>, line: InstrumentId) -> Option<crate::stores::Grade> {
    let issuer = ctx.instruments().issuer_of(line);
    let mut worst: Option<crate::stores::Grade> = None;
    for row in ctx.standing().of_party(issuer) {
        let row = crate::stores::StandingId(*row);
        if !ctx.standing().live(row)
            || ctx.standing().kind_of(row) != crate::stores::standing::GRADE
            || ctx.standing().about(row) != issuer
        {
            continue;
        }
        let Some(notch) = ctx.standing().terms(row).first().copied() else {
            continue;
        };
        let Some(grade) = crate::stores::Grade::at_rank(notch) else {
            continue;
        };
        if worst.is_none_or(|had| grade > had) {
            worst = Some(grade);
        }
    }
    worst
}

fn of_quality(ctx: &MechanismContext<'_>, line: InstrumentId, want: CreditQuality) -> bool {
    match graded(ctx, line) {
        Some(grade) => grade.investment_grade() == (want == CreditQuality::InvestmentGrade),
        None => false,
    }
}

impl Index {
    /// 22 B2, E1: WHAT IS IN THIS INDEX NOW — every line that qualifies for its subject and its
    /// scope, weighted the way it declared, read at one week and never kept.
    pub fn qualifying(ctx: &MechanismContext<'_>, id: IndexId, week: u32) -> Index {
        let subject = ctx.registry().index_subject(id);
        let scope = ctx.registry().index_scope(id);
        let weights = ctx.registry().index_weights(id);
        let mut of: Vec<Constituent> = Vec::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            if !in_scope(ctx, scope, line) || !admits(ctx, subject, line, week) {
                continue;
            }
            let Some(weight) = weighs(ctx, weights, line, week) else {
                continue;
            };
            of.push(Constituent { what: line, weight });
        }
        // A capitalisation band is a RANK and not a threshold somebody declared: the larger half of
        // what qualifies is large, and the rest is small.
        if let IndexSubject::Equity(band) = subject {
            of = banded(ctx, of, band, week);
        }
        Index { of }
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

    /// 22 B2.a: WHAT CARRIED ACROSS A REBALANCE — the members in both sets, at the weights they
    /// carried in the earlier one. The link is measured over these and nothing else, so a member
    /// entering or leaving moves the level by itself not at all.
    pub fn carried_across(&self, now: &Index) -> Index {
        Index {
            of: self
                .of
                .iter()
                .filter(|c| now.contains(c.what))
                .copied()
                .collect(),
        }
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
    /// 22 A4: what the level is measured against, published beside it.
    pub at_base: u32,
    /// The consumer basket's other half: shelter is a contract price in the journal rather than a
    /// line that prints, so the basket that includes it reads the tenancies struck that week.
    pub rent_kind: u32,
    pub rent_key: u32,
    pub rent_weight: f64,
}

impl PublishedIndices {
    /// What a basket cost in a week, or nothing where one of its constituents did not price —
    /// a sum missing a constituent is a different basket wearing the same name.
    fn basket(
        &self,
        ctx: &MechanismContext<'_>,
        definition: &Index,
        subject: IndexSubject,
        week: u32,
    ) -> Option<f64> {
        let mut sum = 0.0;
        for member in &definition.of {
            let print = ctx.prints().of_line(member.what, week)?;
            if print.week != week {
                return None;
            }
            sum += print.price * member.weight;
        }
        match subject {
            IndexSubject::ConsumerPrices => {
                Some(sum + shelter(&self.let_at(ctx, week))? * self.rent_weight)
            }
            IndexSubject::Equity(_)
            | IndexSubject::FixedBond(_)
            | IndexSubject::Cds(_)
            | IndexSubject::TradableTermLoan(_)
            | IndexSubject::ProducerPrices => Some(sum),
        }
    }

    /// 22 B2.a: THE LEVEL, CHAINED from the base. Each week's link is the weighted return of the
    /// members that were in the index in BOTH weeks, at the weights they carried in the earlier
    /// one — so a bond maturing out or a line entering moves nothing by itself, and the level's
    /// continuity survives the rebalance. Nothing is stored: the chain is walked from the base
    /// every time it is asked for.
    fn chained(
        &self,
        ctx: &MechanismContext<'_>,
        id: IndexId,
        subject: IndexSubject,
        from: u32,
        to: u32,
    ) -> Option<f64> {
        let mut level = 1.0;
        for week in (from + 1)..=to {
            let before = Index::qualifying(ctx, id, week - 1);
            let now = Index::qualifying(ctx, id, week);
            let common = before.carried_across(&now);
            if common.of.is_empty() {
                // Nothing carried across, so there is no link and no level — not a jump to one.
                return None;
            }
            let was = self.basket(ctx, &common, subject, week - 1)?;
            let is = self.basket(ctx, &common, subject, week)?;
            if was <= 0.0 {
                return None;
            }
            level *= is / was;
        }
        Some(level)
    }

    /// The rents struck in a week, and how many households each of them stands for.
    fn let_at(&self, ctx: &MechanismContext<'_>, week: u32) -> Vec<(f64, u32)> {
        ctx.journal()
            .of_kind(self.rent_kind)
            .iter()
            .filter(|row| ctx.journal().period_of(**row) == week)
            .filter_map(|row| match ctx.journal().says(*row, self.rent_key) {
                // The tenant is the event's second subject, and its weight is how many households
                // that one tenancy stands for.
                Some(Value::Num(rent)) => ctx
                    .journal()
                    .subjects_of(*row)
                    .get(1)
                    .map(|tenant| (rent, ctx.parties().weight(PartyId::at(*tenant)))),
                _ => None,
            })
            .collect()
    }
}

impl Mechanism for PublishedIndices {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let observed = ctx.week();
        let mut levels = Vec::new();
        for row in 0..ctx.registry().indices() as u32 {
            let id = IndexId::at(row);
            let subject = ctx.registry().index_subject(id);
            let base = ctx.registry().index_base(id);
            // 22 A4: a level is against its base, and the base week is the level 1 it starts from.
            let Some(level) = self.chained(ctx, id, subject, base.0 as u32, observed) else {
                continue;
            };
            levels.push((id, subject, level, base));
        }
        for (id, subject, level, base) in levels {
            ctx.say(
                self.kind,
                &[],
                &[
                    (self.at_index, Value::Num(f64::from(id.0))),
                    (self.at_subject, Value::Num(f64::from(subject.code()))),
                    (self.at_level, Value::Num(level)),
                    (self.at_observed, Value::Num(f64::from(observed))),
                    (self.at_base, Value::Num(base.0 as f64)),
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
            // 22 D3: the curve is over paper whose issuer can fail as a SOVEREIGN — a declared
            // capability the registry answers, not a kind this mechanism compares against.
            let issuer = ctx.instruments().issuer_of(line);
            let sovereign = ctx
                .registry()
                .profile(ctx.parties().kind_of(issuer))
                .is_some_and(|it| it.failure == crate::registry::FailureMode::Sovereign);
            if !sovereign {
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
    fn a_change_in_the_constituents_does_not_move_the_level_by_itself() {
        // 22 B2.a: the link is the weighted return of what was in both weeks, so a member leaving
        // and another arriving at a different price is worth nothing on its own.
        let before = basket();
        let after = Index {
            of: vec![
                Constituent {
                    what: instrument(1),
                    weight: 0.6,
                },
                Constituent {
                    what: instrument(3),
                    weight: 0.4,
                },
            ],
        };
        let common = before.carried_across(&after);
        assert_eq!(common.of.len(), 1);
        assert_eq!(common.of[0].what, instrument(1));
        // And it keeps the weight it carried BEFORE, not the one the new set gives it.
        assert_eq!(common.of[0].weight, 0.6);
        let steady = [
            print(1, 1, 100.0, Provenance::Cleared),
            print(1, 2, 100.0, Provenance::Cleared),
        ];
        let was = common.level_at(1, &steady).unwrap();
        let is = common.level_at(2, &steady).unwrap();
        assert_eq!(is / was, 1.0, "the rebalance alone moved the level");
    }

    #[test]
    fn shelter_is_what_the_households_paying_it_pay_and_not_one_tenancy_averaged() {
        // Two tenancies standing for one household each: the basket's shelter is their mean.
        assert_eq!(shelter(&[(30.0, 1), (50.0, 1)]), Some(40.0));
        // The same two rents, one standing for nine households, is a different number entirely.
        assert_eq!(shelter(&[(30.0, 9), (50.0, 1)]), Some(32.0));
        assert_eq!(shelter(&[]), None);
        // A tenancy that stands for nobody cannot weigh the basket.
        assert_eq!(shelter(&[(30.0, 0)]), None);
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
