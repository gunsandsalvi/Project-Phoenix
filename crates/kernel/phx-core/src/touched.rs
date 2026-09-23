use phx_id::{Slot, TableId};
use phx_num::capacity_exceeded;

const WORD_BITS: u32 = u64::BITS;

/// The rows the day's applies touched, one bitmap per table, which the incremental audit reads at the close and then
/// clears.
#[derive(Debug, Default)]
pub struct TouchedRows {
    tables: Vec<(TableId, Vec<u64>)>,
}

fn word_and_bit(slot: Slot) -> (usize, u64) {
    let Ok(word) = usize::try_from(slot.get() / WORD_BITS) else {
        capacity_exceeded!("touched-row words", usize::MAX, slot.get());
    };
    (word, 1_u64 << (slot.get() % WORD_BITS))
}

impl TouchedRows {
    pub fn mark(&mut self, table: TableId, slot: Slot) {
        let at = match self.tables.binary_search_by_key(&table, |(t, _)| *t) {
            Ok(at) => at,
            Err(at) => {
                self.tables.insert(at, (table, Vec::new()));
                at
            }
        };
        let (word, bit) = word_and_bit(slot);
        let Some((_, words)) = self.tables.get_mut(at) else {
            return;
        };
        if words.len() <= word {
            words.resize(word + 1, 0);
        }
        if let Some(w) = words.get_mut(word) {
            *w |= bit;
        }
    }

    /// The table's touched rows, in slot order.
    pub fn rows(&self, table: TableId) -> impl Iterator<Item = Slot> + '_ {
        let words = self.tables.binary_search_by_key(&table, |(t, _)| *t).ok().and_then(|at| self.tables.get(at));
        words.into_iter().flat_map(|(_, words)| {
            words.iter().zip(0_u32..).flat_map(|(w, i)| {
                (0..WORD_BITS).filter(move |b| w & (1_u64 << b) != 0).map(move |b| Slot::new(i * WORD_BITS + b))
            })
        })
    }

    /// Every table with a touched row, in table order.
    pub fn tables(&self) -> impl Iterator<Item = TableId> + '_ {
        self.tables.iter().map(|(t, _)| *t)
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.tables.iter().flat_map(|(_, words)| words).map(|w| u64::from(w.count_ones())).sum()
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Slot, TableId};

    use super::TouchedRows;

    #[test]
    fn touched_rows_come_back_in_order() {
        let mut t = TouchedRows::default();
        let (a, b) = (TableId::new(3), TableId::new(1));
        for s in [130, 2, 64, 2] {
            t.mark(a, Slot::new(s));
        }
        t.mark(b, Slot::new(0));
        assert_eq!(t.rows(a).map(Slot::get).collect::<Vec<_>>(), vec![2, 64, 130]);
        assert_eq!((t.tables().collect::<Vec<_>>(), t.count()), (vec![b, a], 4));
        t.clear();
        assert_eq!((t.rows(a).count(), t.count()), (0, 0));
    }
}
