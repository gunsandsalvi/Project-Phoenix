//! A value for each of some parties, read at the party's place — by kind, then slot — rather than searched for, and
//! walked in the parties' order.

use phx_id::{PartyKey, Slot};

#[derive(Clone, Debug, phx_macros::Saved)]
pub struct PartyMap<T: phx_store::Saved> {
    by: Vec<Vec<Option<T>>>,
}

impl<T: phx_store::Saved> Default for PartyMap<T> {
    fn default() -> Self {
        PartyMap { by: Vec::new() }
    }
}

fn place(party: PartyKey) -> (usize, usize) {
    (usize::from(party.kind()), usize::try_from(party.slot().get()).unwrap_or(usize::MAX))
}

impl<T: phx_store::Saved> PartyMap<T> {
    #[must_use]
    pub fn get(&self, party: PartyKey) -> Option<&T> {
        let (k, s) = place(party);
        self.by.get(k)?.get(s)?.as_ref()
    }

    pub fn get_mut(&mut self, party: PartyKey) -> Option<&mut T> {
        let (k, s) = place(party);
        self.by.get_mut(k)?.get_mut(s)?.as_mut()
    }

    #[must_use]
    pub fn contains_key(&self, party: PartyKey) -> bool {
        self.get(party).is_some()
    }

    fn cell(&mut self, party: PartyKey) -> Option<&mut Option<T>> {
        let (k, s) = place(party);
        if self.by.len() <= k {
            self.by.resize_with(k + 1, Vec::new);
        }
        let row = self.by.get_mut(k)?;
        if row.len() <= s {
            row.resize_with(s + 1, || None);
        }
        row.get_mut(s)
    }

    /// The party's value set; what it held before.
    pub fn insert(&mut self, party: PartyKey, value: T) -> Option<T> {
        self.cell(party).and_then(|c| c.replace(value))
    }

    pub fn remove(&mut self, party: PartyKey) -> Option<T> {
        let (k, s) = place(party);
        self.by.get_mut(k)?.get_mut(s)?.take()
    }

    /// The party's value, begun at its default where it has none.
    pub fn entry_or_default(&mut self, party: PartyKey) -> Option<&mut T>
    where
        T: Default,
    {
        let cell = self.cell(party)?;
        Some(cell.get_or_insert_with(T::default))
    }

    /// The parties with a value, in their order.
    pub fn keys(&self) -> impl Iterator<Item = PartyKey> + '_ {
        (0_u8..).zip(&self.by).flat_map(|(k, row)| {
            (0_u32..).zip(row).filter(|(_, v)| v.is_some()).map(move |(s, _)| PartyKey::new(k, Slot::new(s)))
        })
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.by.iter_mut().flatten().filter_map(Option::as_mut)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by.iter().flatten().all(Option::is_none)
    }
}
