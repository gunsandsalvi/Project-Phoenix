//! Firms' owners on the core. The self-employed the opening draws — employed, but no one's employees — own and work in
//! the firms of their region: each firm its first owner in an order drawn by lot, the rest dealt over the firms by the
//! hours their output takes of each one's occupation. A firm's first working owner manages it, holding every office
//! its form declares with the preferences it was founded with; each working owner works its country's full-time week
//! in its occupation, counted with the firm's staff. What an owner holds is its household's: it passes to the
//! household's estate, and from an estate where its money goes; a firm's estate pays what its claims leave to its
//! owners, a share each.

use std::collections::BTreeMap;

use phx_core::opening_subject;
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_rand::float::len_u64;

use crate::consts::firm::{OWNERS_PURPOSE, PURPOSES};
use crate::core::{Core, kind_number};
use crate::core_jobs::{JobsOpening, deal};

/// A self-employed person the opening drew: its household, identity, occupation, country and region.
#[derive(Clone, Copy, Debug, phx_macros::Saved)]
pub(crate) struct OpenOwner {
    pub household: PartyKey,
    pub person: u64,
    pub occupation: u32,
    pub country: u8,
    pub region: u32,
}

/// An owner working in its firm: its household, identity, occupation and weekly hours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Worker {
    pub household: PartyKey,
    pub person: u64,
    pub occupation: u32,
    pub hours: u32,
}

/// Who owns what: each owned party's holders, a share each, and each holder's owned parties; each firm's working
/// owners, its manager first, and the firm each works in; the self-employed the opening drew and dealt, and the firms
/// it left with no owner.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Owners {
    pub of: BTreeMap<PartyKey, Vec<PartyKey>>,
    pub holds: BTreeMap<PartyKey, Vec<PartyKey>>,
    pub working: BTreeMap<PartyKey, Vec<Worker>>,
    pub works_at: BTreeMap<(PartyKey, u64), PartyKey>,
    pub drawn: u64,
    pub dealt: u64,
    pub unowned: u64,
}

impl Owners {
    /// The occupations and weekly hours of a firm's working owners.
    pub(crate) fn hours_of(&self, firm: PartyKey) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.working.get(&firm).into_iter().flatten().map(|w| (w.occupation, w.hours))
    }

    /// A party's holders, a share each.
    pub(crate) fn holders_of(&self, owned: PartyKey) -> &[PartyKey] {
        self.of.get(&owned).map_or(&[][..], Vec::as_slice)
    }
}

impl Core {
    /// The self-employed dealt to the firms of their regions: in each region, in an order drawn by lot, one to each
    /// firm, and the rest of each occupation over its firms by the hours their output takes of it, evenly where none
    /// takes it.
    ///
    /// # Errors
    /// A primitive the dealing reads that the register does not hold.
    #[clause("FRM.1", "FRM.23", "PTY.16", "GEN.2")]
    pub fn open_owners(&mut self, o: &JobsOpening<'_>) -> Result<(), String> {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Ok(()) };
        let firms = self.firm_hours(o, firm)?;
        let mut by_region: BTreeMap<u32, Vec<OpenOwner>> = BTreeMap::new();
        for d in std::mem::take(&mut self.drawn.owners) {
            self.owners.drawn += 1;
            by_region.entry(d.region).or_default().push(d);
        }
        for (region, mut members) in by_region {
            let Some(country) = members.first().map(|m| m.country) else { continue };
            let Some(c) = o.countries.iter().find(|c| c.id.get() == country) else {
                return Err(format!("self-employed drawn in country {country}, which the world does not hold"));
            };
            let hours = u32::try_from(o.register.count_in("LAB.full_time_hours", c.id)?).map_err(|e| e.to_string())?;
            let subject = opening_subject(u32::from(country) * PURPOSES + OWNERS_PURPOSE, region);
            let mut lot = o.streams.open(o.stream, subject, Day::new(0), 0);
            // An order drawn by lot, so no household's place in the books decides the firm it owns.
            for i in (1..members.len()).rev() {
                let j = phx_rand::float::index(phx_rand::below_u64(&mut lot, len_u64(i + 1)));
                members.swap(i, j);
            }
            let here = firms.get(&region).map_or(&[][..], Vec::as_slice);
            let mut next = members.into_iter();
            for (slot, _) in here {
                match next.next() {
                    Some(m) => self.own(PartyKey::new(kind_number(firm), *slot), &m, hours),
                    None => self.owners.unowned += 1,
                }
            }
            let mut rest: BTreeMap<u32, Vec<OpenOwner>> = BTreeMap::new();
            for m in next {
                rest.entry(m.occupation).or_default().push(m);
            }
            for (occupation, ms) in rest {
                let at = usize::try_from(occupation).unwrap_or(usize::MAX);
                let mut weights: Vec<f64> = here.iter().map(|(_, h)| h.get(at).copied().unwrap_or(0.0)).collect();
                if !weights.iter().any(|w| *w > 0.0) {
                    weights = vec![1.0; here.len()];
                }
                let mut left = ms.into_iter();
                for ((slot, _), n) in here.iter().zip(deal(len_u64(left.len()), &weights)) {
                    for m in left.by_ref().take(usize::try_from(n).unwrap_or(usize::MAX)) {
                        self.own(PartyKey::new(kind_number(firm), *slot), &m, hours);
                    }
                }
            }
        }
        Ok(())
    }

    /// A self-employed person owning a share of a firm and working in it, managing it where it is the first.
    fn own(&mut self, firm: PartyKey, m: &OpenOwner, hours: u32) {
        let o = &mut self.owners;
        let workers = o.working.entry(firm).or_default();
        let first = workers.is_empty();
        workers.push(Worker { household: m.household, person: m.person, occupation: m.occupation, hours });
        o.works_at.insert((m.household, m.person), firm);
        o.of.entry(firm).or_default().push(m.household);
        o.holds.entry(m.household).or_default().push(firm);
        o.dealt += 1;
        if first {
            let prefs = self.decisions.founding_of(firm);
            self.decisions.appoint(firm, m.person, prefs);
        }
    }

    /// A person who no longer works stops working in its firm, keeping its household's share; where it managed the
    /// firm its offices pass to the next owner working there, or stand empty.
    #[clause("PTY.16", "FRM.1")]
    pub(crate) fn stop_working(&mut self, household: PartyKey, person: u64) {
        let Some(firm) = self.owners.works_at.remove(&(household, person)) else { return };
        let Some(workers) = self.owners.working.get_mut(&firm) else { return };
        let managed = workers.first().is_some_and(|w| w.household == household && w.person == person);
        workers.retain(|w| !(w.household == household && w.person == person));
        let next = workers.first().map(|w| w.person);
        if workers.is_empty() {
            self.owners.working.remove(&firm);
        }
        if managed {
            self.decisions.vacate(firm, Some(person));
            if let Some(p) = next {
                let prefs = self.decisions.founding_of(firm);
                self.decisions.appoint(firm, p, prefs);
            }
        }
    }

    /// What a holder holds passed to another, share for share.
    #[clause("PTY.9", "PTY.7")]
    pub(crate) fn pass_holdings(&mut self, from: PartyKey, to: PartyKey) {
        let Some(owned) = self.owners.holds.remove(&from) else { return };
        for party in &owned {
            for h in self.owners.of.get_mut(party).into_iter().flatten().filter(|h| **h == from) {
                *h = to;
            }
        }
        self.owners.holds.entry(to).or_default().extend(owned);
    }

    /// A firm ended into its estate: its owners own the estate in its place, and those who worked in it no longer
    /// manage or work there; the owners who worked in it are returned.
    #[clause("PTY.9", "FRM.1")]
    pub(crate) fn owners_to_estate(&mut self, firm: PartyKey, estate: PartyKey) -> Vec<Worker> {
        if let Some(holders) = self.owners.of.remove(&firm) {
            for h in &holders {
                for owned in self.owners.holds.get_mut(h).into_iter().flatten().filter(|p| **p == firm) {
                    *owned = estate;
                }
            }
            self.owners.of.insert(estate, holders);
        }
        let workers = self.owners.working.remove(&firm).unwrap_or_default();
        for w in &workers {
            self.owners.works_at.remove(&(w.household, w.person));
        }
        self.decisions.vacate(firm, None);
        workers
    }

    /// An ended estate no longer owned: its holders' shares of it let go.
    pub(crate) fn estate_ended(&mut self, estate: PartyKey) {
        let Some(holders) = self.owners.of.remove(&estate) else { return };
        for h in holders {
            if let Some(owned) = self.owners.holds.get_mut(&h) {
                owned.retain(|p| *p != estate);
                if owned.is_empty() {
                    self.owners.holds.remove(&h);
                }
            }
        }
    }
}
