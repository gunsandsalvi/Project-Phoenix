use phx_core::kind_tables::{KindTable, ListKind, RunHead};
use phx_exec::mix64;
use phx_id::{PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, BlockList, BlockPool};

use crate::consts::HOLDER_SHARDS;

/// A table of holders, as the ledger reaches their lists: rows, holdings, lots and named units live in the holder's
/// own arena, contiguous with its other lists, and nothing outside the arena names a position in it.
pub trait HolderArenas {
    /// The table's identity, which a holder key carries.
    fn table(&self) -> TableId;
    /// The kind of party the table holds, which a line side's declared holder kinds are checked against.
    fn kind(&self) -> &'static str;
    fn party(&self, holder: Slot) -> PartyId;
    /// A holder's list, as its arena's words.
    fn read(&self, holder: Slot, list: ListKind) -> &[u64];
    /// Words appended to a holder's list.
    fn append(&mut self, holder: Slot, list: ListKind, words: &[u64]);
    /// Words written over a holder's list from a word on.
    fn overwrite(&mut self, holder: Slot, list: ListKind, at: usize, words: &[u64]);
    /// Words removed from a holder's list.
    fn remove(&mut self, holder: Slot, list: ListKind, at: usize, count: usize);
    /// A holder's due-day run head.
    fn run_head(&self, holder: Slot) -> RunHead;
    fn set_run_head(&mut self, holder: Slot, head: RunHead);
    /// Words put into a holder's list before the word at `at`, the rest moved after them.
    fn insert(&mut self, holder: Slot, list: ListKind, at: usize, words: &[u64]) {
        let Some(tail) = self.read(holder, list).get(at..).map(<[u64]>::to_vec) else {
            violation!(clause = "REG.14", "words put beyond the end of a holder's list", at = at);
        };
        self.remove(holder, list, at, tail.len());
        self.append(holder, list, words);
        self.append(holder, list, &tail);
    }
}

fn word(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a holder's list", u32::MAX, n);
    };
    w
}

impl<B: Backing> HolderArenas for KindTable<B> {
    fn table(&self) -> TableId {
        self.id()
    }

    fn kind(&self) -> &'static str {
        KindTable::kind(self)
    }

    fn party(&self, holder: Slot) -> PartyId {
        KindTable::party(self, holder)
    }

    fn read(&self, holder: Slot, list: ListKind) -> &[u64] {
        self.words(holder, list)
    }

    fn append(&mut self, holder: Slot, list: ListKind, words: &[u64]) {
        self.edit_list(holder, list, |arena, r| arena.append(r, words));
    }

    fn overwrite(&mut self, holder: Slot, list: ListKind, at: usize, words: &[u64]) {
        self.edit_list(holder, list, |arena, r| {
            let Some(target) = arena.read_mut(*r).get_mut(at..at + words.len()) else {
                violation!(clause = "REG.14", "words written beyond a holder's list", at = at, len = words.len());
            };
            target.copy_from_slice(words);
        });
    }

    fn remove(&mut self, holder: Slot, list: ListKind, at: usize, count: usize) {
        if count > 0 {
            self.edit_list(holder, list, |arena, r| arena.remove(r, word(at), word(count)));
        }
    }

    fn run_head(&self, holder: Slot) -> RunHead {
        KindTable::run_head(self, holder)
    }

    fn set_run_head(&mut self, holder: Slot, head: RunHead) {
        KindTable::set_run_head(self, holder, head);
    }
}

/// A holder as a line's or instrument's holder list keeps it: its table in the high bits and its slot below, the
/// split set by how many holder tables the world has, so a list names holders of any table in one sorted order.
#[clause("REG.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HolderKeys {
    slot_bits: u32,
}

impl HolderKeys {
    /// The split for a world of `tables` holder tables: as many high bits as number them, the rest for slots.
    #[must_use]
    pub fn new(tables: u16) -> HolderKeys {
        let table_bits = u32::from(tables).next_power_of_two().trailing_zeros();
        HolderKeys { slot_bits: u32::BITS - table_bits }
    }

    /// The most rows a holder table may have for its slots to fit, which assembly checks each table against.
    #[must_use]
    pub fn max_slots(self) -> u64 {
        1_u64 << self.slot_bits
    }

    /// A holder's key, from its table's place among the holder tables and its slot.
    #[must_use]
    pub fn key(self, table: u16, slot: Slot) -> u32 {
        if u64::from(slot.get()) >= self.max_slots() {
            capacity_exceeded!("a holder table's slots for its holder keys", self.max_slots(), slot.get());
        }
        let high = u64::from(table) << self.slot_bits;
        let Ok(key) = u32::try_from(high | u64::from(slot.get())) else {
            capacity_exceeded!("holder tables for their holder keys", u32::MAX, table);
        };
        key
    }

    /// A key's table place and slot.
    pub fn split(self, key: u32) -> (u16, Slot) {
        let table = u64::from(key) >> self.slot_bits;
        let slot = u64::from(key) & (self.max_slots() - 1);
        let (Ok(table), Ok(slot)) = (u16::try_from(table), u32::try_from(slot)) else {
            violation!(clause = "REG.4", "a holder key beyond its table and slot bits", key = key);
        };
        (table, Slot::new(slot))
    }
}

/// Holder lists, each a sorted block list of holder keys owned by a line or an instrument, kept in several pools with
/// each list's pool chosen by its owner's identity, so parallel updates never share a pool and block identities never
/// depend on which worker made them.
#[clause("REG.4")]
#[derive(Debug)]
pub struct HolderLists<B: Backing> {
    pools: Vec<BlockPool<B>>,
    keys: HolderKeys,
}

impl<B: Backing> HolderLists<B> {
    /// Room for `blocks` blocks in each pool.
    pub fn new(space: &mut AddressSpace, blocks: u32, keys: HolderKeys) -> HolderLists<B> {
        HolderLists { pools: (0..HOLDER_SHARDS).map(|_| BlockPool::new(space, blocks)).collect(), keys }
    }

    #[must_use]
    pub fn keys(&self) -> HolderKeys {
        self.keys
    }

    fn shard(owner: u32) -> usize {
        let Ok(s) = usize::try_from(mix64(u64::from(owner)) % u64::from(HOLDER_SHARDS)) else {
            violation!(clause = "REG.4", "a holder list's pool beyond the machine's words", owner = owner);
        };
        s
    }

    fn pool(&self, owner: u32) -> &BlockPool<B> {
        let Some(p) = self.pools.get(Self::shard(owner)) else {
            violation!(clause = "REG.4", "a holder list's pool that was never made", owner = owner);
        };
        p
    }

    fn pool_mut(&mut self, owner: u32) -> &mut BlockPool<B> {
        let Some(p) = self.pools.get_mut(Self::shard(owner)) else {
            violation!(clause = "REG.4", "a holder list's pool that was never made", owner = owner);
        };
        p
    }

    /// A holder entered on its owner's list, once.
    pub(crate) fn enter(&mut self, owner: u32, list: &mut BlockList, table: u16, holder: Slot) {
        let key = self.keys.key(table, holder);
        if self.pool(owner).contains(*list, key) {
            violation!(clause = "REG.4", "a holder entered twice on one holder list", owner = owner, key = key);
        }
        self.pool_mut(owner).insert_sorted(list, key);
    }

    /// A holder taken off its owner's list.
    pub(crate) fn leave(&mut self, owner: u32, list: &mut BlockList, table: u16, holder: Slot) {
        let key = self.keys.key(table, holder);
        if !self.pool(owner).contains(*list, key) {
            violation!(clause = "REG.4", "a holder left a holder list it is not on", owner = owner, key = key);
        }
        self.pool_mut(owner).remove_sorted(list, key);
    }

    /// A list's holders, as their keys, in order.
    pub fn iter(&self, owner: u32, list: BlockList) -> impl Iterator<Item = u32> + '_ {
        self.pool(owner).iter(list)
    }
}

impl<B: Backing> HolderLists<B> {
    /// The blocks each pool has room for, which a save records to make the pools again.
    pub(crate) fn blocks(&self) -> u32 {
        self.pools.first().map_or(0, BlockPool::max_blocks)
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;

    use super::HolderKeys;

    #[test]
    fn holder_keys_round_trip() {
        let keys = HolderKeys::new(5);
        assert_eq!(keys.max_slots(), 1 << 29, "three bits number five tables");
        let k = keys.key(4, Slot::new(123_456));
        assert_eq!(keys.split(k), (4, Slot::new(123_456)));
        assert!(keys.key(1, Slot::new(0)) > keys.key(0, Slot::new(99)), "sorted by table, then slot");
    }
}
