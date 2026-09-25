//! The realised rates, measured live: over a fixed sample of cells, each process's expected hits a day — each value's
//! persons times its rate — beside the hits it drew there, by the value's first component, so a run's realised rate
//! can be held to the declared one within its sampling error.

use std::collections::BTreeMap;

use phx_core::CellView;
use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_pop::kind::PopKindDecl;
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

/// The measures by process, calendar year and the first component of the value hit.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Rates(pub BTreeMap<(u32, u32, u32), RateTally>);

/// Whether a cell is among those the rates are measured over: its identity's mix, so the sample holds whatever the
/// cell becomes.
#[must_use]
pub fn sampled(party: PartyId) -> bool {
    phx_exec::mix64(party.get()).is_multiple_of(RATE_SAMPLE)
}

fn first_component(kind: &PopKindDecl, group: usize, value: u32) -> u32 {
    let Some(g) = kind.groups.get(group) else {
        phx_num::violation!(clause = "REP.32", "a value of a group the kind does not hold", group = group);
    };
    phx_core::component(&g.components, value, 0)
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
    /// A sampled cell's hits by a process in a year, by group and value.
    pub fn realised(&mut self, (process, year): (usize, u32), kind: &PopKindDecl, by_value: &[(usize, u32, u64)]) {
        for (g, v, n) in by_value {
            self.0.entry((process_id(process), year, first_component(kind, *g, *v))).or_default().realised += n;
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
    /// Each sampled cell's expected hits today by every process on its kind, at the values it holds as 3b drew them.
    #[clause("CHN.7")]
    pub(crate) fn measure_rates(&mut self, day: Day) {
        let geo = crate::world::geo_in(&self.own);
        let country_of = |r: u32| geo.map.regions.get(usize::try_from(r).ok()?).map(|x| x.country);
        let date = self.calendar.date(day);
        let year = year_of(date);
        let Population { kinds, .. } = &self.population;
        let cells = self.books.parties.cells_mut().0;
        let rates = &mut self.metrics.rates;
        for (k, kd) in kinds.iter().enumerate() {
            let table = Population::table::<SystemBacking>(cells, k);
            for slot in table.slots().filter(|s| sampled(table.party(*s))) {
                let record = kd.keys.record(table.hot(slot).key_id);
                let key = |name: &str| {
                    kd.decl.key_attrs.iter().position(|a| a.item.name == name).map(|i| kd.decl.key.get(&record, i))
                };
                let view =
                    CellView { kind: kd.decl.kind, party: table.party(slot), key: &key, country_of: &country_of, date };
                for (p, b) in self.processes.iter().enumerate().filter(|(_, b)| b.kind == k) {
                    for g in &b.groups {
                        let name = crate::cells::group_name(&kd.decl, *g);
                        for (v, n) in table.profile_group(slot, *g) {
                            let rate = b.process.rate(&self.register, &view, name, v);
                            rates.expect((p, year, first_component(&kd.decl, *g, v)), u64::from(n), rate);
                        }
                    }
                }
            }
        }
    }
}
