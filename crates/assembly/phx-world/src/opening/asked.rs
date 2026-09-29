//! What a country's output asks of its people at the opening: each occupation's hours a year by the activities whose
//! staff the world holds — the products' firms and public administration's agency — at the accounts' output and the
//! ways' hours a unit. The persons' occupations and the self-employed among them, the firms of each product and the
//! agency's share of each occupation's jobs are all read from it, so the people drawn are the people the output needs.

use phx_core::OpeningCountry;
use phx_core::Register;
use phx_macros::clause;

use crate::consts::firm::PUBLIC_ADMINISTRATION;
use crate::opening::economy::{accounts, table};

/// An activity the world staffs: its place among the accounts' activities, the share of its hours its self-employed
/// work, and its hours a year of each occupation.
#[derive(Clone, Debug, PartialEq)]
pub struct Staffed {
    pub activity: usize,
    pub owned: f64,
    pub hours: Vec<f64>,
}

/// What a country's output asks of each occupation, by the activities the world staffs.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    pub staffed: Vec<Staffed>,
}

impl Asked {
    /// A country's asked hours: each product's units in the accounts and public administration's output, its unit a
    /// currency unit as its way counts it, times the way's hours of each occupation a unit; a product's self-employed
    /// at its share, public administration's staff all the agency's employees.
    ///
    /// # Errors
    /// A primitive the hours read that the register does not hold, or an activity asking hours no number counts.
    #[clause("GEN.2", "TEC.1", "SOC.2")]
    pub fn of(register: &Register, c: &OpeningCountry) -> Result<Asked, String> {
        let snap = crate::core_firms::snapshot(register, c)?;
        let (ways, _) = table(register, "TEC.labour", c.id)?;
        let owned = table(register, "LAB.self_employed_shares", c.id)?.0.into_iter().next().unwrap_or_default();
        let (flows, _) = accounts(register, c.id)?;
        let Some(public) = flows.output.get(PUBLIC_ADMINISTRATION).map(|o| o * c.gdp) else {
            return Err(format!("country {}: no public administration in the accounts", c.id.get()));
        };
        let units = snap.units.iter().copied().enumerate().map(|(p, u)| (p, u, owned.get(p).copied()));
        let mut staffed = Vec::new();
        for (activity, units, owned) in units.chain([(PUBLIC_ADMINISTRATION, public, Some(0.0))]) {
            let hours: Vec<f64> =
                ways.iter().map(|occ| occ.get(activity).copied().unwrap_or(f64::NAN) * units).collect();
            let Some(owned) = owned.filter(|o| o.is_finite()) else {
                return Err(format!("country {}: activity {activity} with no self-employed share", c.id.get()));
            };
            if hours.iter().any(|h| !h.is_finite()) {
                return Err(format!("country {}: activity {activity} asks hours no way counts", c.id.get()));
            }
            staffed.push(Staffed { activity, owned, hours });
        }
        Ok(Asked { staffed })
    }

    /// Each occupation's hours a year asked as employees' and as the self-employed's.
    #[must_use]
    pub fn by_occupation(&self) -> Vec<[f64; 2]> {
        let occupations = self.staffed.first().map_or(0, |s| s.hours.len());
        let mut out = vec![[0.0; 2]; occupations];
        for s in &self.staffed {
            for (o, h) in out.iter_mut().zip(&s.hours) {
                for (way, part) in [(sys_lab::AS_EMPLOYEE, 1.0 - s.owned), (sys_lab::AS_OWNER, s.owned)] {
                    if let Some(v) = o.get_mut(way) {
                        *v += h * part;
                    }
                }
            }
        }
        out
    }

    /// Each occupation's employees' hours that are an activity's, as a share of all its employees' hours; none where
    /// no employee's hour of it is asked.
    #[must_use]
    pub fn employees_share(&self, activity: usize) -> Vec<Option<f64>> {
        let mine: Vec<f64> = self
            .staffed
            .iter()
            .find(|s| s.activity == activity)
            .map(|s| s.hours.iter().map(|h| h * (1.0 - s.owned)).collect())
            .unwrap_or_default();
        self.by_occupation()
            .iter()
            .enumerate()
            .map(|(o, w)| {
                let all = w.get(sys_lab::AS_EMPLOYEE).copied().filter(|a| *a > 0.0)?;
                Some(mine.get(o).copied().unwrap_or(0.0) / all)
            })
            .collect()
    }

    /// Each activity's persons employed: the country's employed shared over the staffed activities by the hours
    /// each asks.
    #[must_use]
    pub fn persons(&self, employed: f64) -> Vec<(usize, f64)> {
        let total = |s: &Staffed| s.hours.iter().sum::<f64>();
        let whole: f64 = self.staffed.iter().map(total).sum();
        self.staffed.iter().map(|s| (s.activity, if whole > 0.0 { employed * total(s) / whole } else { 0.0 })).collect()
    }
}

#[path = "asked_tests.rs"]
mod tests;
