//! STA, the statistics agencies: each country's agency publishes its series on a calendar, each from a sample of the
//! period's records or of households, a first release from the returns in by then and a revision from every return.

mod consts;

use if_state::stats::{NAMES, Release, SERIES, Schedule, StaKind, StaLaw};
use phx_id::Day;

use crate::consts::{DAYS_A_YEAR, SHARE_PARTS};
use phx_core::{Declarations, HandlerTable, OpeningCountry, Register, StreamDef, System, declare_prim, declare_stream};
use phx_macros::clause;

declare_stream! { pub SampleStream = "STA.sample" { purpose: Sample, keyed: true, clause: "STA.2" } }
declare_stream! { pub ReturnsStream = "STA.returns" { purpose: Sample, keyed: true, clause: "STA.2" } }

declare_prim! {
    /// The business day of the month each series is released on, in the series' order.
    pub RELEASE_DAY = "STA.release_day" {
        kind: Policy, decided_by: "statistics agency", value: Table1 { axis_exp: 0, exp: 0 }, clause: "STA.5",
        scope: PerCountry
    }
}
declare_prim! {
    /// The months after a period's end each series' first release comes in.
    pub RELEASE_LAG = "STA.release_lag" {
        kind: Policy, decided_by: "statistics agency", value: Table1 { axis_exp: 0, exp: 0 }, clause: "STA.5",
        scope: PerCountry
    }
}
declare_prim! {
    /// The share of its records or households each series samples.
    pub SAMPLE_SHARE = "STA.sample_share" {
        kind: Policy, decided_by: "statistics agency", value: Table1 { axis_exp: 0, exp: 4 }, clause: "STA.5",
        scope: PerCountry
    }
}
declare_prim! {
    /// The age classes the agency reports by, by their first ages.
    pub AGE_CLASSES = "STA.age_classes" {
        kind: Policy, decided_by: "statistics agency", value: Partition { exp: 0 }, clause: "STA.5", scope: Shared
    }
}
declare_prim! {
    /// The share of a sample's returns in by its first release.
    pub EARLY_RETURNS = "STA.early_returns" {
        kind: Policy, decided_by: "statistics agency", value: Fixed { exp: 2 }, clause: "STA.5", scope: PerCountry
    }
}
declare_prim! {
    /// The months after a first release its revision, from every return, comes.
    pub REVISION_MONTHS = "STA.revision_months" {
        kind: Policy, decided_by: "statistics agency", value: Count, clause: "STA.5", scope: PerCountry
    }
}

/// A country's statistics law.
///
/// # Errors
/// A primitive missing, of another shape, or not one value for each series.
#[clause("STA.5")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<StaLaw, String> {
    let column = |id: &str| -> Result<Vec<i64>, String> {
        let t = register.table1_in(id, c.id)?;
        if t.values().len() == SERIES {
            Ok(t.values().to_vec())
        } else {
            Err(format!("`{id}` holds {} values for {SERIES} series ({})", t.values().len(), NAMES.join(", ")))
        }
    };
    let whole = |v: i64| u32::try_from(v).map_err(|e| e.to_string());
    let (days, lags, shares) = (column(RELEASE_DAY.id)?, column(RELEASE_LAG.id)?, column(SAMPLE_SHARE.id)?);
    let series = days
        .iter()
        .zip(&lags)
        .zip(&shares)
        .map(|((d, l), s)| {
            Ok(Schedule { day: whole(*d)?, lag: whole(*l)?, sample: phx_rand::float::from_i64(*s) / SHARE_PARTS })
        })
        .collect::<Result<Vec<Schedule>, String>>()?;
    let revision = u32::try_from(register.count_in(REVISION_MONTHS.id, c.id)?).map_err(|e| e.to_string())?;
    Ok(StaLaw { series, early: register.fixed_in(EARLY_RETURNS.id, c.id)?, revision })
}

/// Events a year over the person-days exposed to them; none where no one was exposed.
#[clause("STA.1", "STA.2")]
#[must_use]
pub fn rate(events: f64, exposure: f64) -> Option<f64> {
    (exposure > 0.0).then(|| events / exposure * DAYS_A_YEAR)
}

/// The release of a series a reader may read on a day: of the latest period published by then, its latest vintage
/// published by then. None before the first is published.
#[clause("STA.4", "IDX.1")]
#[must_use]
pub fn latest(releases: &[Release], (series, country): (u8, u8), today: Day) -> Option<&Release> {
    releases.iter().filter(|r| r.series == series && r.country == country && r.published <= today).fold(
        None,
        |best: Option<&Release>, r| match best {
            Some(b) if (b.period, b.vintage) >= (r.period, r.vintage) => Some(b),
            _ => Some(r),
        },
    )
}

/// The statistics agency, which the kernel binds: its consumer index over retail, its producer index over the goods
/// markets at the factory gate.
pub const AGENCY: StaKind = StaKind {
    law,
    rate,
    latest,
    consumer: &["SRV.retail"],
    producer: &["GDS.commodities", "GDS.between_firms"],
    classes: AGE_CLASSES.id,
    sample: SampleStream::DECL,
    returns: ReturnsStream::DECL,
};

/// The statistics agencies.
#[derive(Debug)]
pub struct Sta;

impl System for Sta {
    const CODE: &'static str = "STA";

    fn declare(d: &mut Declarations) {
        for p in [&RELEASE_DAY, &RELEASE_LAG, &SAMPLE_SHARE] {
            let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(p);
        }
        let _: phx_core::Prim<phx_num::Fixed<2>> = d.prim(&EARLY_RETURNS);
        let _: phx_core::Prim<phx_num::Count> = d.prim(&REVISION_MONTHS);
        let _: phx_core::Prim<phx_core::register::values::Partition> = d.prim(&AGE_CLASSES);
        d.stream(SampleStream::DECL);
        d.stream(ReturnsStream::DECL);
        d.market(Box::new(AGENCY));
    }

    fn handlers(_: &mut HandlerTable) {}
}

#[cfg(test)]
mod tests {
    use if_state::stats::Release;
    use phx_id::Day;

    use super::{latest, rate};

    #[test]
    fn rates_are_events_a_year_over_exposure() {
        assert_eq!(rate(0.0, 0.0), None, "no one exposed, no rate");
        let r = rate(10.0, 365.25 * 1_000.0).unwrap();
        assert!((r - 0.01).abs() < 1e-15, "ten deaths among a thousand exposed a year: {r}");
    }
    fn release(period: u32, vintage: u8, published: u32, value: i64) -> Release {
        Release { series: 0, country: 1, period, vintage, published: Day::new(published), values: vec![value] }
    }

    #[test]
    fn revision_from_later_sample() {
        let first = release(7, 0, 40, 101);
        let revised = release(7, 1, 70, 103);
        let kept = vec![first.clone(), revised.clone()];
        assert_eq!(latest(&kept, (0, 1), Day::new(39)), None, "nothing before its publication day");
        assert_eq!(latest(&kept, (0, 1), Day::new(40)), Some(&first));
        assert_eq!(latest(&kept, (0, 1), Day::new(70)), Some(&revised), "the revision is the later estimate");
        assert_eq!(kept.len(), 2, "both vintages are kept with their dates");
        let next = release(8, 0, 71, 104);
        let all = vec![first, revised, next.clone()];
        assert_eq!(latest(&all, (0, 1), Day::new(71)), Some(&next), "a later period's first release reads first");
        assert_eq!(latest(&all, (0, 2), Day::new(71)), None, "another country's is its own");
    }
}
