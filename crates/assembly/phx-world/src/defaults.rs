//! Parties under an insolvency law in default of payment: a party whose contract has been in arrears for longer than
//! its law's grace ends into an estate at 2e, succeeding to every row and holding it had, which the estate liquidates
//! from the next business day. The days a grace ends are queued as the contract process records the fails, and read
//! again from the arrears when a save is loaded.

use std::collections::BTreeSet;

use phx_core::{Declarations, KindTableRef, Register, SubStep};
use phx_id::{Day, LineId, PartyId, Slot};
use phx_ledger::algebra::Side;
use phx_ledger::apply::ApplyAt;
use phx_ledger::transfer::{LineTransfer, MoveAt};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_store::SystemBacking;

use crate::world::World;

/// A kind's insolvency law as the world applies it: the kind's table among the books' holders, whether its parties
/// are individuals, and each country's grace in days.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Law {
    pub place: u16,
    pub individuals: bool,
    pub grace: Vec<u32>,
}

/// A contract's grace ending: its day, then the party, the line and the side in arrears.
pub(crate) type Due = (Day, PartyId, LineId, u8);

/// Every declared law bound to its kind's table and its grace; every refusal at once.
pub(crate) fn bind(d: &Declarations, register: &Register) -> Result<Vec<Law>, Vec<String>> {
    let individuals: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Individuals).map(|(_, k)| k.name).collect();
    let agents: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Cells).map(|(_, k)| k.name).collect();
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, law) in &d.insolvency {
        let place = individuals
            .iter()
            .position(|k| *k == law.kind)
            .map(|i| (i, true))
            .or_else(|| agents.iter().position(|k| *k == law.kind).map(|i| (individuals.len() + i, false)));
        let Some((place, is_individual)) = place else {
            errors
                .push(format!("{system} puts `{}`, a kind the world does not keep, under an insolvency law", law.kind));
            continue;
        };
        let grace = register
            .counts_per_country(law.grace_days)
            .map(|days| days.into_iter().map(u32::try_from).collect::<Result<Vec<u32>, _>>());
        match (u16::try_from(place), grace) {
            (Ok(place), Ok(Ok(grace))) => out.push(Law { place, individuals: is_individual, grace }),
            (_, Err(e)) => errors.push(format!("`{}`'s grace: {e}", law.kind)),
            _ => errors.push(format!("`{}`'s grace or table beyond their widths", law.kind)),
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

fn side_code(side: Side) -> u8 {
    match side {
        Side::Asset => 0,
        Side::Liability => 1,
    }
}

fn side_of(code: u8) -> Side {
    if code == 0 { Side::Asset } else { Side::Liability }
}

impl World {
    /// The law a party is under, if any, from its table, and its grace in the country it is sited in.
    fn law_of(&self, party: PartyId) -> Option<(&Law, u32)> {
        let (place, _) = self.books.parties.row(party);
        let law = self.laws.iter().find(|l| l.place == place)?;
        let Missing::Present(country) = self.country_of_party(party) else {
            violation!(clause = "REP.41", "a party under an insolvency law sited in no country", party = party.get());
        };
        let Some(grace) = law.grace.get(usize::from(country.get())).copied() else {
            violation!(clause = "NUM.3", "a country with no insolvency grace", party = party.get());
        };
        Some((law, grace))
    }

    /// Each contract in arrears of a party under a law, queued at the day its grace ends.
    fn queue_default(&mut self, (party, line, side): (PartyId, LineId, Side), since: Day) {
        let Some((_, grace)) = self.law_of(party) else { return };
        let Some(ends) = since.get().checked_add(grace) else {
            violation!(clause = "FRM.15", "a grace ending beyond the world's days", party = party.get());
        };
        self.defaults.insert((Day::new(ends), party, line, side_code(side)));
    }

    /// After the contract process: every fail it left in arrears on a contract of a live party under a law, queued.
    #[clause("FRM.15")]
    pub(crate) fn defaults_queue(&mut self, fails: &[phx_ledger::fails::Fail]) {
        if self.laws.is_empty() {
            return;
        }
        for f in fails {
            let Missing::Present(row) = f.row else { continue };
            let phx_core::Resolved::Live(party, _) = self.books.parties.directory().resolve(f.party) else { continue };
            if let Some(since) = self.books.ledger.arrears().of(row.line, row.side, party) {
                self.queue_default((party, row.line, row.side), since);
            }
        }
    }

    /// The queue read again from the arrears a loaded save holds.
    pub(crate) fn defaults_rebuild(&mut self) {
        let held: Vec<(phx_ledger::contract_process::ArrearsKey, Day)> = self.books.ledger.arrears().iter().collect();
        for (key, since) in held {
            if let phx_core::Resolved::Live(party, _) = self.books.parties.directory().resolve(key.party) {
                self.queue_default((party, key.line, side_of(key.side)), since);
            }
        }
    }

    /// 2e: every party whose grace has ended on a contract still in arrears defaults and ends into an estate, so no
    /// firm fails its payments for ever.
    #[clause("FRM.15", "FRM.21", "L3", "PTY.9")]
    pub(crate) fn defaults_end(&mut self, day: Day) {
        let mut due: Vec<Due> = Vec::new();
        while let Some(first) = self.defaults.first().copied() {
            if first.0 > day {
                break;
            }
            self.defaults.remove(&first);
            due.push(first);
        }
        let mut ended: BTreeSet<PartyId> = BTreeSet::new();
        for (ends, party, line, side) in due {
            if ended.contains(&party) {
                continue;
            }
            let phx_core::Resolved::Live(live, _) = self.books.parties.directory().resolve(party) else { continue };
            let Some((law, grace)) = self.law_of(live) else { continue };
            let individuals = law.individuals;
            let Some(since) = self.books.ledger.arrears().of(line, side_of(side), live) else { continue };
            if since.get().checked_add(grace).is_none_or(|e| e > ends.get()) {
                continue;
            }
            self.default(day, live, individuals);
            ended.insert(live);
        }
    }

    /// A party in default ended into an estate standing for as many real parties as it did, at its site, which
    /// succeeds to every row and holding it had.
    fn default(&mut self, day: Day, party: PartyId, individuals: bool) {
        let m = MoveAt {
            contracts: phx_ledger::opening::contract_unit(&self.register),
            rounding: Round::HalfEven,
            day,
            at: ApplyAt::Day(SubStep::S2e),
        };
        let (place, slot) = self.books.parties.row(party);
        if !individuals {
            let first = self.books.parties.first_cell_place();
            let Some(kind) = place.checked_sub(first).map(usize::from) else {
                violation!(clause = "REP.1", "an agent's table before the first", party = party.get());
            };
            let region = self.agent_region(kind, slot);
            self.end_agent((day, SubStep::S2e), (kind, slot, party), Some(region));
            self.agent_day.defaults += 1;
            return;
        }
        self.release_visits(place, slot);
        let site = self.books.parties.site(party);
        let estate = self.books.parties.begin_weighted(phx_core::ESTATE_KIND.name, site, day, 1);
        let succeeded = self.books.dues.succeeded;
        let rows: Vec<(LineId, Side, u32)> = phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .iter()
            .map(|r| (r.row.line, r.side(), r.row.count))
            .collect();
        for (line, side, count) in rows {
            let t = LineTransfer { line, side, from: party, to: estate, count, reason: succeeded };
            if let Err(f) = self.books.transfer(t, m, self.audit.stream()) {
                violation!(clause = "PTY.9", "an estate's succession did not settle", party = f.party.get());
            }
        }
        if let Err(f) = self.books.pass_holdings((party, estate), succeeded, m, self.audit.stream()) {
            violation!(clause = "PTY.9", "an estate's succession to holdings did not settle", party = f.party.get());
        }
        self.books.parties.end(party, day);
        self.accounts.close(party);
        self.agent_day.estates += 1;
        self.agent_day.defaults += 1;
    }

    /// The country a live party is in: an individual's by its site, an agent's by the region it lives in.
    pub(crate) fn country_of_party(&self, party: PartyId) -> Missing<phx_id::CountryId> {
        let (place, slot) = self.books.parties.row(party);
        let first = self.books.parties.first_cell_place();
        let Some(kind) = place.checked_sub(first).map(usize::from) else {
            return self.geo().country_of(self.books.parties.site(party));
        };
        let region = self.agent_region(kind, slot);
        let regions = &self.geo().map.regions;
        match usize::try_from(region).ok().and_then(|r| regions.get(r)) {
            Some(r) => Missing::Present(r.country),
            None => Missing::Absent,
        }
    }

    /// The region an agent of a sited kind lives in.
    fn agent_region(&self, kind: usize, slot: Slot) -> u32 {
        let Some(kd) = self.population.kinds.get(kind) else {
            violation!(clause = "REP.1", "an agent of a kind the world does not keep", kind = kind);
        };
        let Missing::Present(at) = kd.decl.sited_by else {
            violation!(clause = "REP.41", "an agent in default of a kind that is not sited");
        };
        Population::table::<SystemBacking>(self.books.parties.cells(), kind).attr(slot, at)
    }
}
