//! Injection: one family's discrepancy put into a world read back from a save, and the audit run over it without a
//! day stepped, to show that family alone sees it.

use std::any::Any;

use phx_core::{
    AuditStream, EventStore, FactStore, FamilyMode, InjectTarget, KernelTable, NewEvent, RecordStore, SubStep,
};
use phx_id::{Day, PartyId, Slot};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::Subject;

use crate::world::{OwnState, World};

/// The loaded stores an injection may change, and the audit's sink its legs go to.
struct Target<'a> {
    day: Day,
    books: &'a mut phx_ledger::books::Books,
    markets: &'a mut phx_market::markets::Markets,
    accounts: &'a mut phx_acct::accounts::Accounts,
    records: &'a mut RecordStore,
    events: &'a mut EventStore,
    event_kinds: usize,
    tables: &'a mut [KernelTable],
    own: &'a mut [(&'static str, OwnState)],
    stream: &'a mut dyn AuditStream,
}

impl InjectTarget for Target<'_> {
    fn day(&self) -> Day {
        self.day
    }

    fn unhanded_party(&self) -> PartyId {
        PartyId::new(self.books.parties.directory().next())
    }

    fn live_party(&self) -> Option<PartyId> {
        let directory = self.books.parties.directory();
        (1..directory.next()).map(PartyId::new).find(|p| matches!(directory.resolve(*p), phx_core::Resolved::Live(..)))
    }

    fn add_record(&mut self, subject: PartyId, day: Day, substep: SubStep) -> Result<(), String> {
        let Some(kind) = self.records.kinds().first().map(|k| k.name) else {
            return Err("the save declares no record kind".to_owned());
        };
        self.records.write(kind, subject, day, substep, &[]);
        Ok(())
    }

    fn fact(&self, table: &str, fact: &'static str, slot: Slot) -> Missing<i64> {
        match self.tables.iter().find(|t| t.name == table) {
            Some(t) if t.columns.facts().any(|f| f == fact) && slot.get() < t.columns.rows() => {
                t.columns.value(fact, slot)
            }
            _ => Missing::Absent,
        }
    }

    fn set_fact(&mut self, table: &str, fact: &'static str, slot: Slot, value: i64) -> Result<(), String> {
        let Some(t) = self.tables.iter_mut().find(|t| t.name == table) else {
            return Err(format!("the save keeps no table `{table}`"));
        };
        if !t.columns.facts().any(|f| f == fact) || slot.get() >= t.columns.rows() {
            return Err(format!("table `{table}` keeps no fact `{fact}` at row {}", slot.get()));
        }
        t.columns.write(fact, slot, value);
        Ok(())
    }

    fn add_event(&mut self, subject: Subject, day: Day, substep: SubStep) -> Result<(), String> {
        if self.event_kinds == 0 {
            return Err("the save declares no event kind".to_owned());
        }
        let subjects = [subject];
        self.events.record(NewEvent {
            day,
            substep,
            kind: 0,
            subjects: &subjects,
            details: &[],
            public: true,
            develops_from: Missing::Absent,
        });
        Ok(())
    }

    fn books(&mut self) -> &mut dyn Any {
        self.books
    }

    fn markets(&mut self) -> &mut dyn Any {
        self.markets
    }

    fn accounts(&mut self) -> &mut dyn Any {
        self.accounts
    }

    fn own(&mut self, system: &str) -> Option<&mut dyn Any> {
        let (_, state) = self.own.iter_mut().find(|(code, _)| *code == system)?;
        Some(&mut **state)
    }

    fn stream(&mut self) -> &mut dyn AuditStream {
        self.stream
    }
}

impl World {
    /// One family's injection into this world, read back from a save, then the audit over the closes of a full
    /// rolling cycle from the day after the save's, no day stepped: the families that found something, in the
    /// order they first did. The world is discarded after; it is never run on.
    ///
    /// # Errors
    /// A world not read from a save, a family the audit does not run, or an injection the save cannot take.
    #[clause("N1")]
    pub fn inject(&mut self, family: &str) -> Result<Vec<&'static str>, String> {
        if !self.loaded {
            return Err("an injection goes into a save loaded apart, never into the world being run".to_owned());
        }
        let day = self.today.succ();
        let cycle = self
            .audit
            .families()
            .filter_map(|f| match f.mode {
                FamilyMode::Rolling { cycle_days } => Some(u32::from(cycle_days)),
                _ => None,
            })
            .fold(1, |longest, c| if c > longest { c } else { longest });
        let Some((injected, stream)) = self.audit.injecting(family) else {
            return Err(format!("the audit runs no family `{family}`"));
        };
        let mut target = Target {
            day,
            books: &mut self.books,
            markets: &mut self.markets,
            accounts: &mut self.accounts,
            records: &mut self.records,
            events: &mut self.events,
            event_kinds: self.event_kinds.len(),
            tables: &mut self.tables,
            own: &mut self.own,
            stream,
        };
        injected.inject(&mut target)?;
        let before = self.findings.len();
        let mut close = day;
        for _ in 0..cycle {
            let _ = self.audit_close(close, None);
            close = close.succ();
        }
        let mut lit: Vec<&'static str> = Vec::new();
        for f in self.findings.all().iter().skip(before) {
            if !lit.contains(&f.family) {
                lit.push(f.family);
            }
        }
        Ok(lit)
    }
}
