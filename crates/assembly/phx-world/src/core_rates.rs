//! The realised rates, measured live on the core: over a fixed sample of households, each process's expected hits a
//! day — each person's chance — beside the hits it drew there, by the person's age, so a run's realised rate can be
//! held to the declared one within its sampling error.

use std::collections::BTreeMap;

use phx_core::pop_process::{AgentView, Household, Person};
use phx_id::{Date, Day, PartyId, Slot};
use phx_macros::clause;

use crate::consts::RATE_SAMPLE;
use crate::core::Core;
use crate::core_pop::Ctx;

/// One process's measure at one class of values: the hits expected, their variance, and the hits drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, phx_macros::Saved)]
pub struct RateTally {
    pub expected: f64,
    pub variance: f64,
    pub realised: u64,
}

/// The measures by process, calendar year and the age of the person hit, and the households measured.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Rates {
    pub tallies: BTreeMap<(u32, u32, u32), RateTally>,
    sample: Option<Vec<(Slot, PartyId)>>,
}

/// Whether a household is among those the rates are measured over: its identity's mix.
#[must_use]
pub fn sampled(party: PartyId) -> bool {
    phx_exec::mix64(party.get()).is_multiple_of(RATE_SAMPLE)
}

/// A person's whole years on a date, as the tallies key them.
fn age_class(p: &Person, date: Date) -> u32 {
    u32::try_from(p.age_on(date))
        .unwrap_or_else(|_| phx_num::violation!(clause = "REP.25", "a person read before it was born"))
}

/// A date's year as the tallies key it.
fn year_of(date: Date) -> u32 {
    u32::try_from(date.year()).unwrap_or_else(|_| phx_num::violation!(clause = "TIME.2", "a year before the era"))
}

fn process_id(p: usize) -> u32 {
    u32::try_from(p).unwrap_or_else(|_| phx_num::capacity_exceeded!("processes", u32::MAX, p))
}

impl Rates {
    /// A sampled household's persons a process reached on a day.
    pub(crate) fn realised(&mut self, process: usize, h: &Household, reached: &[usize], date: Date) {
        for p in reached.iter().filter_map(|i| h.persons.get(*i)) {
            let key = (process_id(process), year_of(date), age_class(p, date));
            self.tallies.entry(key).or_default().realised += 1;
        }
    }

    fn expect(&mut self, key: (u32, u32, u32), rate: f64) {
        let t = self.tallies.entry(key).or_default();
        t.expected += rate;
        t.variance += rate * (1.0 - rate);
    }
}

impl Core {
    /// Each sampled household's expected hits today by every process on its persons, as the day finds them.
    #[clause("CHN.7")]
    pub(crate) fn measure_rates(&mut self, ctx: &Ctx<'_>, day: Day) {
        let Some(place) = self.bound.kinds.household else { return };
        let Some(decl) = self.declared.household.clone() else { return };
        let Some(store) = self.kinds.get(place) else { return };
        // The sample is found in one sweep the first day, then read alone; a household that ended leaves it.
        let mut sample = self.rates.sample.take().unwrap_or_else(|| {
            store
                .parties
                .live_slots()
                .filter_map(|s| store.parties.id(s).map(|p| (s, p)))
                .filter(|(_, p)| sampled(*p))
                .collect()
        });
        sample.retain(|(slot, party)| store.parties.id(*slot) == Some(*party));
        let date = ctx.calendar.date(day);
        let year = year_of(date);
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let mut h = Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() };
        for &(slot, party) in &sample {
            self.read_household((place, &decl), slot, &mut h);
            let attr = |name: &str| h.attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
            let decider = |_: &str| phx_num::violation!(clause = "MND.20", "a decision taken while a chance is read");
            let view =
                AgentView { kind: decl.kind, party, attr: &attr, country_of: &country_of, date, decider: &decider };
            for hz in &self.hazards {
                let Some(b) = ctx.processes.get(hz.process) else { continue };
                for (_, person) in h.present() {
                    let rate = b.process.rate(ctx.register, &view, person);
                    self.rates.expect((process_id(hz.process), year, age_class(person, date)), rate);
                }
            }
        }
        self.rates.sample = Some(sample);
    }
}
