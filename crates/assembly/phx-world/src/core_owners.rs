//! Firms' owners on the core. The self-employed the opening draws — employed, but no one's employees — own and work in
//! the firms of their region, in an order drawn by lot: each occupation's dealt over the firms by the hours their
//! output takes of it that its product's self-employed work, so a product's owners are its share of its hours and a
//! firm with none is run by its founders' preferences alone. A firm's first working owner manages it, holding every office
//! its form declares with the preferences it was founded with; each working owner works its country's full-time week
//! in its occupation, counted with the firm's staff. What an owner holds is its household's: it passes to the
//! household's estate, and from an estate where its money goes; a firm's estate pays what its claims leave to its
//! owners, a share each.

use std::collections::BTreeMap;

use phx_core::opening_subject;
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::float::len_u64;

use crate::consts::WEEKS_A_YEAR;
use crate::consts::firm::{COMPENSATION, OWNERS_PURPOSE, PRODUCT, PURPOSES, SURPLUS};
use crate::core::{Core, kind_number};
use crate::core_jobs::{JobsOpening, deal, month_point};
use crate::opening::economy::table;

/// A self-employed person the opening drew: its household, identity, occupation, country and region.
#[derive(Clone, Copy, Debug, phx_macros::Saved)]
pub(crate) struct OpenOwner {
    pub household: PartyKey,
    pub person: u64,
    pub occupation: u32,
    pub country: u8,
    pub region: u32,
}

/// An owner working in its firm: its household, identity, occupation, weekly hours and age class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Worker {
    pub household: PartyKey,
    pub person: u64,
    pub occupation: u32,
    pub hours: u32,
    /// Its age class at the last year's close, or at the opening.
    pub window: Missing<u16>,
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
    /// What a firm's working owners' hours earn a month: each one's last wage point; none where an owner holds none.
    pub(crate) fn owners_pay(&self, firm: PartyKey, country: usize) -> Option<f64> {
        let law = self.labour.laws.get(country)?;
        let decl = self.declared.household.as_ref()?;
        let mut pay = 0.0;
        for w in self.owners.working.get(&firm).into_iter().flatten() {
            let ps = self.persons.get(usize::from(w.household.kind()))?.as_ref()?;
            let at = ps.place_of(w.household.slot(), w.person)?;
            let word = ps.of(w.household.slot()).nth(at)?.word;
            let point = phx_pop::person::unpack(decl, word)
                .attr(sys_lab::LAST_POINT.name)
                .filter(|p| *p != if_labour::class::NO_POINT)?;
            pay += sys_lab::wages::wage_at(law, i64::from(point));
        }
        Some(pay)
    }

    /// The self-employed dealt to the firms of their regions: in each region, in an order drawn by lot, each
    /// occupation's over its firms by the hours their output takes of it times their product's self-employed's share,
    /// evenly where none takes it.
    ///
    /// # Errors
    /// A primitive the dealing reads that the register does not hold.
    #[clause("FRM.1", "FRM.23", "PTY.16", "GEN.2")]
    pub fn open_owners(&mut self, o: &JobsOpening<'_>, types: &phx_val::types::Types) -> Result<(), String> {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Ok(()) };
        let firms = self.firm_hours(o, firm)?;
        let mut owned: BTreeMap<u8, Vec<f64>> = BTreeMap::new();
        for c in o.countries {
            owned.insert(
                c.id.get(),
                table(o.register, "LAB.self_employed_shares", c.id)?.0.into_iter().next().unwrap_or_default(),
            );
        }
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
            let date = o.calendar.date(o.today);
            let subject = opening_subject(u32::from(country) * PURPOSES + OWNERS_PURPOSE, region);
            let mut lot = o.streams.open(o.stream, subject, Day::new(0), 0);
            // An order drawn by lot, so no household's place in the books decides the firm it owns.
            for i in (1..members.len()).rev() {
                let j = phx_rand::float::index(phx_rand::below_u64(&mut lot, len_u64(i + 1)));
                members.swap(i, j);
            }
            let here = firms.get(&region).map_or(&[][..], Vec::as_slice);
            let mut share = Vec::with_capacity(here.len());
            for (slot, _) in here {
                let product = self.record_word(firm, *slot, PRODUCT).and_then(|p| usize::try_from(p).ok());
                let Some(s) = product.and_then(|p| owned.get(&country)?.get(p).copied()) else {
                    return Err(format!("country {country}: a firm's product with no self-employed share"));
                };
                share.push(s);
            }
            let mut of: BTreeMap<u32, Vec<OpenOwner>> = BTreeMap::new();
            for m in members {
                of.entry(m.occupation).or_default().push(m);
            }
            for (occupation, ms) in of {
                let at = usize::try_from(occupation).unwrap_or(usize::MAX);
                let mut weights: Vec<f64> =
                    here.iter().zip(&share).map(|((_, h), s)| h.get(at).copied().unwrap_or(0.0) * s).collect();
                if !weights.iter().any(|w| *w > 0.0) {
                    weights = vec![1.0; here.len()];
                }
                let mut left = ms.into_iter();
                for ((slot, _), n) in here.iter().zip(deal(len_u64(left.len()), &weights)) {
                    for m in left.by_ref().take(usize::try_from(n).unwrap_or(usize::MAX)) {
                        self.own(PartyKey::new(kind_number(firm), *slot), &m, (hours, date), types);
                    }
                }
            }
        }
        let working = &self.owners.working;
        let unowned = firms
            .values()
            .flatten()
            .filter(|(slot, _)| !working.contains_key(&PartyKey::new(kind_number(firm), *slot)));
        self.owners.unowned = len_u64(unowned.count());
        Ok(())
    }

    /// A firm's working owners' hours a year in an occupation.
    pub(crate) fn owner_hours(&self, firm: PartyKey, occupation: u32) -> f64 {
        self.owners.hours_of(firm).filter(|(o, _)| *o == occupation).map(|(_, h)| f64::from(h) * WEEKS_A_YEAR).sum()
    }

    /// What each working owner's hours earn: the self-employed's labour income in its country's accounts — its
    /// labour share, which counts it, less its employees' compensation — shared over the owners as their hours would
    /// be paid at their activity's and occupation's employee wage, and held as the owner's last wage point. An
    /// activity whose owners' labour income passes its operating surplus and mixed income, which holds it, is a
    /// finding: its tables disagree.
    ///
    /// # Errors
    /// A primitive the income reads that the register does not hold, or a country whose labour share is no more than
    /// its compensation, so its self-employed would earn nothing.
    #[clause("GEN.4", "GEN.15", "FRM.14")]
    pub fn price_owners(&mut self, o: &JobsOpening<'_>) -> Result<(), String> {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Ok(()) };
        let working: Vec<(PartyKey, Vec<Worker>)> = self.owners.working.iter().map(|(k, w)| (*k, w.clone())).collect();
        for c in o.countries {
            let id = c.id.get();
            let added = crate::opening::economy::accounts(o.register, c.id)?.0.added;
            let part = |col: usize| -> Vec<f64> {
                added.iter().map(|r| r.get(col).copied().unwrap_or(f64::NAN) * c.gdp).collect()
            };
            let (compensation, surplus) = (part(COMPENSATION), part(SURPLUS));
            let Some(share) = c.derived("GEN.labour_share").map(|v| v / phx_core::consts::PERCENT_F64) else {
                return Err(format!("country {id}: no labour share"));
            };
            let income = share * c.gdp - compensation.iter().sum::<f64>();
            if income <= 0.0 {
                return Err(format!("country {id}: a labour share of {share} leaves the self-employed nothing"));
            }
            let law = sys_lab::law::law(o.register, c)?;
            // Each owner with the employee wage an hour of its activity and occupation, where its activity has one.
            let mut owners: Vec<(Worker, usize, f64)> = Vec::new();
            for (key, workers) in &working {
                let (Some(product), Some(region)) = (
                    self.record_word(firm, key.slot(), crate::consts::firm::PRODUCT),
                    self.record_word(firm, key.slot(), crate::consts::firm::REGION),
                ) else {
                    continue;
                };
                if !c.regions.iter().any(|(r, _)| i64::from(*r) == region) {
                    continue;
                }
                let activity = usize::try_from(product).unwrap_or(usize::MAX);
                for w in workers {
                    if let Some(wage) = self.drawn.wage_in((id, activity), w.occupation) {
                        owners.push((*w, activity, wage));
                    }
                }
            }
            let at_wages: f64 = owners.iter().map(|(w, _, wage)| wage * f64::from(w.hours) * WEEKS_A_YEAR).sum();
            if at_wages <= 0.0 {
                continue;
            }
            let earned = income / at_wages;
            let mut by_activity: BTreeMap<usize, f64> = BTreeMap::new();
            for (w, activity, wage) in owners {
                let Some(point) = month_point(&law, earned * wage, w.hours) else {
                    phx_num::violation!(
                        clause = "REP.34",
                        "an owner's income beyond the wage points",
                        activity = activity
                    );
                };
                self.put_last_point((w.household, w.person), point);
                *by_activity.entry(activity).or_insert(0.0) += earned * wage * f64::from(w.hours) * WEEKS_A_YEAR;
            }
            for (activity, owed) in by_activity {
                let held = surplus.get(activity).copied().unwrap_or(f64::NAN);
                if owed > held {
                    self.found.push(phx_core::findings::Finding {
                        family: "opening",
                        clause: "GEN.4",
                        owner: phx_core::findings::FindingOwner::Run,
                        size: i128::from(phx_ledger::opening::whole(owed - held)),
                        unit: phx_core::findings::Unit::Count,
                        day: o.today,
                        detail: format!(
                            "country {id}, activity {activity}: its owners' labour income {owed:.0} a year passes its \
                             operating surplus and mixed income {held:.0}"
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// A self-employed person owning a share of a firm and working in it, managing it where it is the first.
    fn own(
        &mut self,
        firm: PartyKey,
        m: &OpenOwner,
        (hours, date): (u32, phx_id::Date),
        types: &phx_val::types::Types,
    ) {
        let window = match self.age_of((m.household, m.person), date) {
            Some(age) => types.window_of(age),
            None => Missing::Absent,
        };
        let o = &mut self.owners;
        let workers = o.working.entry(firm).or_default();
        let first = workers.is_empty();
        workers.push(Worker { household: m.household, person: m.person, occupation: m.occupation, hours, window });
        o.works_at.insert((m.household, m.person), firm);
        o.of.entry(firm).or_default().push(m.household);
        o.holds.entry(m.household).or_default().push(firm);
        o.dealt += 1;
        if first {
            let prefs = phx_core::Prefs { window, ..self.decisions.founding_of(firm) };
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
        let next = workers.first().map(|w| (w.person, w.window));
        if workers.is_empty() {
            self.owners.working.remove(&firm);
        }
        if managed {
            self.decisions.vacate(firm, Some(person));
            if let Some((p, window)) = next {
                let prefs = phx_core::Prefs { window, ..self.decisions.founding_of(firm) };
                self.decisions.appoint(firm, p, prefs);
            }
        }
    }

    /// What a holder holds passed to another, share for share.
    #[clause("PTY.9", "PTY.7", "GEO.15")]
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

    /// The day's ended estates no longer owned: their holders' shares of them let go, each holder's in one pass.
    pub(crate) fn estates_ended(&mut self, estates: &std::collections::BTreeSet<PartyKey>) {
        let mut by_holder: BTreeMap<PartyKey, std::collections::BTreeSet<PartyKey>> = BTreeMap::new();
        for estate in estates {
            for h in self.owners.of.remove(estate).unwrap_or_default() {
                by_holder.entry(h).or_default().insert(*estate);
            }
        }
        for (h, gone) in by_holder {
            if let Some(owned) = self.owners.holds.get_mut(&h) {
                owned.retain(|p| !gone.contains(p));
                if owned.is_empty() {
                    self.owners.holds.remove(&h);
                }
            }
        }
    }
}
