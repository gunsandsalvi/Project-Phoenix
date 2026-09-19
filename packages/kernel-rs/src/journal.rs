//! The journal: what happened, in writing order, for ever.

use crate::ids::Names;

/// What an event's payload can hold.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    /// A count, a level, a ratio — with its unit named by the key, never by the number.
    Num(f64),
    /// A name, as a row in `Names`.
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

    /// The kinds and the keys, named once.
    pub kinds: Names,
    pub keys_named: Names,

    /// The events of one period, so a reader does not walk the world's whole history to find this
    /// week's.
    by_period: Vec<(u32, u32)>,
    /// And the events of one KIND, so asking when a company last published does not walk the world's
    /// whole history.
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
        // The period's range, extended as it is written. A period's events must be CONTIGUOUS: out
        // of order they split across two ranges and `in_period` finds only the first, which is a
        // wrong answer rather than a refused one.
        if let Some(last) = self.by_period.last() {
            let newest = self.period[last.0 as usize];
            assert!(period >= newest, "Audit C1: an event in period {period} written after one in {newest}");
        }
        match self.by_period.last_mut() {
            Some(last) if self.period[last.0 as usize] == period => last.1 = row + 1,
            _ => self.by_period.push((row, row + 1)),
        }
        self.by_kind.entry(kind).or_default().push(row);
        row
    }

    /// Every event of one kind, oldest first.
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

    /// One field of an event's payload, by the key's row.
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

// The journal has no test, and both it had were reads of a `Vec` it had just written.
//
// `says` returns `Option<Value>`, so no caller can take a field that is not there; `kind_of` and
// `subjects_of` return what `say` stored. The one claim the round-trip did not hold is the period
// index agreeing with the period column, and that is a CONTRACT now, asserted in `say` where the
// two are written together.
