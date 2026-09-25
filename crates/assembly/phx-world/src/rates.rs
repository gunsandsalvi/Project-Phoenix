//! The realised rates, measured live: over a fixed sample of agents, each process's expected hits a day — each person's
//! chance, once for every twin — beside the hits it drew there, by the person's age, so a run's realised rate can be
//! held to the declared one within its sampling error.

use std::collections::BTreeMap;

use phx_core::{AgentView, Household};
use phx_id::{Date, Day, PartyId};
use phx_macros::clause;
use phx_pop::population::Population;
use phx_store::SystemBacking;

use crate::consts::RATE_SAMPLE;
use crate::world::World;

/// One process's measure at one class of values: the hits expected, their variance, and the hits drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, phx_macros::Saved)]
pub struct RateTally {
    pub expected: f64,
    pub variance: f64,
    pub realised: u64,
}

/// The measures by process, calendar year and the age of the person hit.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Rates(pub BTreeMap<(u32, u32, u32), RateTally>);

/// Whether an agent is among those the rates are measured over: its identity's mix.
#[must_use]
pub fn sampled(party: PartyId) -> bool {
    phx_exec::mix64(party.get()).is_multiple_of(RATE_SAMPLE)
}

/// A person's whole years on a date, as the tallies key them.
fn age_class(p: &phx_core::Person, date: Date) -> u32 {
    let Ok(a) = u32::try_from(p.age_on(date)) else {
        phx_num::violation!(clause = "REP.25", "a person read before it was born");
    };
    a
}

/// A date's year as the tallies key it.
#[must_use]
pub fn year_of(date: phx_id::Date) -> u32 {
    let Ok(y) = u32::try_from(date.year()) else { phx_num::violation!(clause = "TIME.2", "a year before the era") };
    y
}

fn process_id(p: usize) -> u32 {
    let Ok(p) = u32::try_from(p) else { phx_num::capacity_exceeded!("processes", u32::MAX, p) };
    p
}

impl Rates {
    /// A sampled agent's persons a process reached on a day, once for every twin.
    pub fn realised(
        &mut self,
        (process, year): (usize, u32),
        h: &Household,
        reached: &[usize],
        twins: u64,
        date: Date,
    ) {
        for p in reached.iter().filter_map(|i| h.persons.get(*i)) {
            self.0.entry((process_id(process), year, age_class(p, date))).or_default().realised += twins;
        }
    }

    fn expect(&mut self, (process, year, class): (usize, u32, u32), persons: u64, rate: f64) {
        let t = self.0.entry((process_id(process), year, class)).or_default();
        let n = phx_rand::float::from_u64(persons);
        t.expected += n * rate;
        t.variance += n * rate * (1.0 - rate);
    }
}

impl World {
    /// Each sampled agent's expected hits today by every process on its kind, at its persons as 3b drew them.
    #[clause("CHN.7")]
    pub(crate) fn measure_rates(&mut self, day: Day) {
        let geo = crate::world::geo_in(&self.own);
        let country_of = |r: u32| geo.map.regions.get(usize::try_from(r).ok()?).map(|x| x.country);
        let date = self.calendar.date(day);
        let year = year_of(date);
        let Population { kinds, .. } = &self.population;
        let cells = self.books.parties.cells();
        let rates = &mut self.metrics.rates;
        for (k, kd) in kinds.iter().enumerate().filter(|(_, kd)| kd.processes > 0) {
            let table = Population::table::<SystemBacking>(cells, k);
            for slot in table.slots().filter(|s| sampled(table.party(*s))) {
                let h = phx_pop::explicit::household(&kd.decl, table, slot);
                let twins = u64::from(table.multiplicity(slot).get());
                let attr = |name: &str| h.attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
                let view = AgentView {
                    kind: kd.decl.kind,
                    party: table.party(slot),
                    attr: &attr,
                    country_of: &country_of,
                    date,
                };
                for (p, b) in self.processes.iter().enumerate().filter(|(_, b)| b.kind == k) {
                    for (_, person) in h.present() {
                        let rate = b.process.rate(&self.register, &view, person);
                        rates.expect((p, year, age_class(person, date)), twins, rate);
                    }
                }
            }
        }
    }
}
