use libm::{expm1, log1p};
use phx_macros::clause;

/// What a hazard acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActsOn {
    /// Members in a role of a kind's parties, as persons of a household.
    Role {
        kind: &'static str,
        role: &'static str,
    },
    Party {
        kind: &'static str,
    },
    /// Every person of a kind's parties, in whichever role, each read at the same components.
    Persons {
        kind: &'static str,
    },
    Tile,
    Region,
    Country,
    /// Units of a held class: a plant failing, a vehicle's accident.
    Holding {
        class: &'static str,
    },
}

/// A date on which a rate's inputs can change though its row is not visited, which ends the validity of the
/// envelope its candidate days were drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateChange {
    /// The first day of each year, when tables read by age or by year roll on.
    YearStart,
    /// The first day of each month, which ends the windows of a rate that climbs through a year, as the chance of a
    /// birthday not yet passed does.
    MonthStart,
    /// A policy value's effective day.
    Policy(&'static str),
    /// The row's next review by the named schedule.
    Review(&'static str),
}

/// A hazard's rate: a table primitive read at declared axes of the thing it acts on, and the dates its inputs can
/// change without a visit; a change of profile values comes only with a visit, which draws afresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateFn {
    pub table: &'static str,
    pub axes: &'static [&'static str],
    pub changes: &'static [RateChange],
}

/// How a scheduled hazard bounds its rate over a row's profile values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeRule {
    /// The table's greatest rate over the values the row's profile holds.
    MaxOverProfile,
}

/// How candidate days are drawn: ahead at an envelope rate and thinned, or every day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawScheme {
    Scheduled { envelope: EnvelopeRule },
    Daily,
}

/// A hazard process: what it acts on, its rate, what an occurrence does, how it is drawn, from which stream, and
/// where its rate comes from.
#[clause("CHN.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HazardDecl {
    pub name: &'static str,
    pub acts_on: ActsOn,
    pub rate: RateFn,
    pub outcome: &'static str,
    pub scheme: DrawScheme,
    pub stream: &'static str,
    pub clause: &'static str,
    pub source: &'static str,
}

impl HazardDecl {
    /// # Errors
    /// When the hazard names no rate table, outcome, stream or source.
    pub fn validate(&self) -> Result<(), String> {
        let named = [self.rate.table, self.outcome, self.stream, self.source];
        if named.iter().any(|s| s.trim().is_empty()) {
            return Err(format!("hazard `{}` lacks a rate table, outcome, stream or source", self.name));
        }
        Ok(())
    }
}

/// The daily probability of an annual one over a year of `days` days, so the days' chances compound to it exactly:
/// p = 1 − (1 − q)^(1/days).
#[clause("CHN.2")]
#[must_use]
pub fn annual_to_daily(q: f64, days: u32) -> f64 {
    -expm1(log1p(-q) / f64::from(days))
}

#[cfg(test)]
mod tests {
    use super::annual_to_daily;

    #[test]
    fn rate_fn_annual_to_daily_exact() {
        for (q, days) in [(0.01, 365), (0.01, 366), (0.3, 365), (1e-9, 365)] {
            let p = annual_to_daily(q, days);
            // The chance of at least one hit in the year's days, computed without cancellation.
            let yearly = -libm::expm1(f64::from(days) * libm::log1p(-p));
            assert!((yearly - q).abs() <= 1e-13 * q, "q = {q}: {yearly}");
        }
        // Drawn day by day, 365 days at 1% a year compound to 1%.
        let p = annual_to_daily(0.01, 365);
        let survived = (0..365).fold(1.0_f64, |s, _| s * (1.0 - p));
        assert!((1.0 - survived - 0.01).abs() < 1e-13);
    }
}
