//! The journal: what happened, in writing order, for ever.
//!
//! Columns, not records. The TypeScript journal keeps every event as a frozen object with a frozen
//! `data` object and a copied `subjects` array behind six indexes, and it retains ~896 bytes per
//! event while the world writes 178,604 a period. Here an event is a ROW: its scalars are
//! columns, its subjects are a counted slice of one flat column, and its data is a counted slice of
//! another. Nothing is allocated per event and nothing is copied to read one.
//!
//! This is the history. Nothing else stores what happened, and a reader that wants a total
//! walks this rather than keeping a second tally of it.

use crate::ids::Names;

/// What an event's payload can hold. It is TYPED, where the TypeScript `Record<string, unknown>`
/// was not: Law 8 says the unit is part of the number, and a payload that could hold anything was
/// the reason the journal had to refuse money-as-a-value at run time on every write.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    /// A count, a level, a ratio — with its unit named by the key, never by the number.
    Num(f64),
    /// A name, as a row in `Names`. An id is never a display name.
    Text(u32),
    Flag(bool),
}

#[derive(Default)]
pub struct Journal {
    period: Vec<u32>,
    cycle: Vec<u16>,
    kind: Vec<u32>,
    public: Vec<bool>,
    subject_at: Vec<u32>,
    subject_len: Vec<u16>,
    data_at: Vec<u32>,
    data_len: Vec<u16>,

    subjects: Vec<u32>,
    keys: Vec<u32>,
    values: Vec<Value>,

    /// The kinds and the keys, named once. A kind is compared as a row, never as a string.
    pub kinds: Names,
    pub keys_named: Names,

    /// The events of one period, so a reader does not walk the world's whole history to
    /// find this week's. Written where an event is written; there is no second history.
    by_period: Vec<(u32, u32)>,
    /// And the events of one KIND. A mechanism asking *when did this company last
    /// publish* had to walk every event this world has ever recorded — 323,000 a period — so the
    /// read that would have answered it was one nobody could afford to take. Register A3's
    /// both-directions rule: the same rows, indexed the other way, written where they are written.
    by_kind: std::collections::HashMap<u32, Vec<u32>>,
}

impl Journal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.period.len()
    }

    pub fn is_empty(&self) -> bool {
        self.period.is_empty()
    }

    /// An event names its subjects, so it can be checked against the state.
    pub fn say(
        &mut self,
        period: u32,
        cycle: u16,
        kind: u32,
        subjects: &[u32],
        data: &[(u32, Value)],
        public: bool,
    ) -> u32 {
        let row = self.period.len() as u32;
        self.period.push(period);
        self.cycle.push(cycle);
        self.kind.push(kind);
        self.public.push(public);
        self.subject_at.push(self.subjects.len() as u32);
        self.subject_len.push(subjects.len() as u16);
        self.subjects.extend_from_slice(subjects);
        self.data_at.push(self.keys.len() as u32);
        self.data_len.push(data.len() as u16);
        for &(k, v) in data {
            self.keys.push(k);
            self.values.push(v);
        }
        // The period's range, extended as it is written. Events arrive in period order because the
        // period loop is the only writer, so a period's events are contiguous.
        match self.by_period.last_mut() {
            Some(last) if self.period[last.0 as usize] == period => last.1 = row + 1,
            _ => self.by_period.push((row, row + 1)),
        }
        self.by_kind.entry(kind).or_default().push(row);
        row
    }

    /// Every event of one kind, oldest first. A read over the rows, never a second history.
    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }

    pub fn period_of(&self, row: u32) -> u32 {
        self.period[row as usize]
    }

    pub fn kind_of(&self, row: u32) -> u32 {
        self.kind[row as usize]
    }

    pub fn is_public(&self, row: u32) -> bool {
        self.public[row as usize]
    }

    pub fn subjects_of(&self, row: u32) -> &[u32] {
        let at = self.subject_at[row as usize] as usize;
        let len = self.subject_len[row as usize] as usize;
        &self.subjects[at..at + len]
    }

    /// One field of an event's payload, by the key's row. Missing is missing: there is no zero.
    pub fn says(&self, row: u32, key: u32) -> Option<Value> {
        let at = self.data_at[row as usize] as usize;
        let len = self.data_len[row as usize] as usize;
        for i in at..at + len {
            if self.keys[i] == key {
                return Some(self.values[i]);
            }
        }
        None
    }

    /// This period's events, as rows, without walking the history.
    pub fn in_period(&self, period: u32) -> std::ops::Range<u32> {
        for &(from, to) in &self.by_period {
            if self.period[from as usize] == period {
                return from..to;
            }
        }
        0..0
    }

    pub fn all(&self) -> std::ops::Range<u32> {
        0..self.period.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_event_names_its_subjects_and_keeps_its_payload_typed() {
        let mut j = Journal::new();
        let paid = j.kinds.declare("wage.paid");
        let amount = j.keys_named.declare("amount");
        let row = j.say(3, 1, paid, &[7, 9], &[(amount, Value::Num(1250.0))], true);
        assert_eq!(j.kind_of(row), paid);
        assert_eq!(j.subjects_of(row), &[7, 9]);
        assert_eq!(j.says(row, amount), Some(Value::Num(1250.0)));
        // Missing is missing, and it is not zero.
        assert_eq!(j.says(row, 999), None);
    }

    #[test]
    fn a_period_is_found_without_walking_the_history() {
        let mut j = Journal::new();
        let k = j.kinds.declare("k");
        for p in 1..=3u32 {
            for _ in 0..4 {
                j.say(p, 0, k, &[], &[], true);
            }
        }
        assert_eq!(j.in_period(2).len(), 4);
        assert_eq!(j.all().len(), 12);
        for row in j.in_period(2) {
            assert_eq!(j.period_of(row), 2);
        }
    }
}
