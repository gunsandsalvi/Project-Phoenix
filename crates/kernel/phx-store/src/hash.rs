use phx_num::violation;

use crate::arena::{CellListRef, ChunkArena, ListRef};
use crate::backing::Backing;
use crate::block_list::{BlockList, BlockPool};
use crate::column::Column;
use crate::consts::{SIP_C_ROUNDS, SIP_D_ROUNDS, SIP_INIT, SIP_LEN_SHIFT, SIP_ROT, SIP_TWEAK_128, SIP_TWEAK_SECOND};
use crate::convert::{to_u64, to_usize};
use crate::descriptor::{ColumnDescriptor, FieldTag};
use crate::pod::{Pod, as_bytes};
use crate::table::SlotAlloc;

const WORD: usize = size_of::<u64>();

/// SipHash-2-4 with a 128-bit result, fed incrementally: a keyed hash whose collisions no input can arrange.
#[derive(Clone, Debug)]
pub struct Sip128 {
    v: [u64; 4],
    tail: u64,
    tail_bytes: u32,
    len: u64,
}

impl Sip128 {
    #[must_use]
    pub fn new(key: [u64; 2]) -> Sip128 {
        let [k0, k1] = key;
        let [i0, i1, i2, i3] = SIP_INIT;
        Sip128 { v: [k0 ^ i0, k1 ^ i1 ^ SIP_TWEAK_128, k0 ^ i2, k1 ^ i3], tail: 0, tail_bytes: 0, len: 0 }
    }

    fn rounds(&mut self, n: usize) {
        let [r0, r1, r2, r3, r4, r5] = SIP_ROT;
        let [mut v0, mut v1, mut v2, mut v3] = self.v;
        for _ in 0..n {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(r0) ^ v0;
            v0 = v0.rotate_left(r1);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(r2) ^ v2;
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(r3) ^ v0;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(r4) ^ v2;
            v2 = v2.rotate_left(r5);
        }
        self.v = [v0, v1, v2, v3];
    }

    fn compress(&mut self, m: u64) {
        let [_, _, _, v3] = &mut self.v;
        *v3 ^= m;
        self.rounds(SIP_C_ROUNDS);
        let [v0, ..] = &mut self.v;
        *v0 ^= m;
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.len = self.len.wrapping_add(to_u64(bytes.len()));
        let mut rest = bytes;
        while self.tail_bytes != 0 {
            let Some((b, more)) = rest.split_first() else { return };
            self.tail |= u64::from(*b) << (u8::BITS * self.tail_bytes);
            self.tail_bytes += 1;
            rest = more;
            if to_usize(self.tail_bytes) == WORD {
                self.compress(self.tail);
                (self.tail, self.tail_bytes) = (0, 0);
            }
        }
        let (words, remainder) = rest.as_chunks::<WORD>();
        for w in words {
            self.compress(u64::from_le_bytes(*w));
        }
        for b in remainder {
            self.tail |= u64::from(*b) << (u8::BITS * self.tail_bytes);
            self.tail_bytes += 1;
        }
    }

    #[must_use]
    pub fn finish(mut self) -> u128 {
        let [len_byte, ..] = self.len.to_le_bytes();
        let last = (u64::from(len_byte) << SIP_LEN_SHIFT) | self.tail;
        self.compress(last);
        let [_, _, v2, _] = &mut self.v;
        *v2 ^= SIP_TWEAK_128;
        self.rounds(SIP_D_ROUNDS);
        let [v0, v1, v2, v3] = self.v;
        let low = v0 ^ v1 ^ v2 ^ v3;
        let [_, v1, ..] = &mut self.v;
        *v1 ^= SIP_TWEAK_SECOND;
        self.rounds(SIP_D_ROUNDS);
        let [v0, v1, v2, v3] = self.v;
        let high = v0 ^ v1 ^ v2 ^ v3;
        u128::from(low) | (u128::from(high) << u64::BITS)
    }
}

/// The world's logical content as stored: live slots' rows in slot order and lists read through their references,
/// never offsets, dead words, freed slots or page tails, so two layouts of the same content hash equal.
#[derive(Clone, Debug)]
pub struct LogicalHasher {
    sip: Sip128,
}

impl LogicalHasher {
    #[must_use]
    pub fn new(key: [u64; 2]) -> LogicalHasher {
        LogicalHasher { sip: Sip128::new(key) }
    }

    pub fn u64(&mut self, v: u64) {
        self.sip.write(&v.to_le_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.sip.write(b);
    }

    /// Each live slot and its row, in slot order.
    pub fn column<T: Pod, B: Backing>(&mut self, column: &Column<T, B>, slots: &SlotAlloc<B>) {
        let rows = column.slice();
        for slot in slots.live_slots() {
            let Some(row) = rows.get(to_usize(slot.get())) else {
                violation!(clause = "SET.12", "a live slot with no row in a column", slot = slot.get());
            };
            self.u64(u64::from(slot.get()));
            self.bytes(as_bytes(std::slice::from_ref(row)));
        }
    }

    pub fn list<B: Backing>(&mut self, arena: &ChunkArena<B>, list: ListRef) {
        self.u64(u64::from(list.len));
        self.bytes(as_bytes(arena.read(list)));
    }

    pub fn cell_list<B: Backing>(&mut self, arena: &ChunkArena<B>, owner: u32, r: CellListRef) {
        self.list(arena, arena.resolve(owner, r));
    }

    pub fn block_list<B: Backing>(&mut self, pool: &BlockPool<B>, list: BlockList) {
        self.u64(u64::from(list.len()));
        for entry in pool.iter(list) {
            self.bytes(&entry.to_le_bytes());
        }
    }

    #[must_use]
    pub fn finish(self) -> u128 {
        self.sip.finish()
    }
}

/// Maps a stored reference to the permanent identity it names, which only the directory above the stores knows.
pub trait SlotIdentity {
    fn identity(&self, tag: FieldTag, raw: u64) -> u64;
}

fn is_reference(tag: FieldTag) -> bool {
    matches!(tag, FieldTag::PartyRef | FieldTag::LineRef | FieldTag::InstrumentRef | FieldTag::TileRef)
}

/// The world's content by permanent identity: the caller feeds rows in identity order, references are translated
/// to what they name, and lists of references are hashed sorted by identity, so two worlds equal but for where
/// things are stored hash equal.
#[derive(Clone, Debug)]
pub struct IdentityHasher {
    sip: Sip128,
}

impl IdentityHasher {
    #[must_use]
    pub fn new(key: [u64; 2]) -> IdentityHasher {
        IdentityHasher { sip: Sip128::new(key) }
    }

    pub fn u64(&mut self, v: u64) {
        self.sip.write(&v.to_le_bytes());
    }

    /// One row's fields as its descriptor lays them out, each reference replaced by its identity.
    pub fn row<I: SlotIdentity + ?Sized>(&mut self, desc: &ColumnDescriptor, row: &[u8], ids: &I) {
        if row.len() != usize::from(desc.elem_bytes) {
            violation!(clause = "SET.12", "a row of another width than its descriptor", len = row.len());
        }
        for f in desc.fields {
            let at = usize::from(f.offset);
            let Some(field) = row.get(at..at + usize::from(f.width)) else {
                violation!(clause = "SET.12", "a field outside its row", offset = f.offset);
            };
            let mut word = [0; WORD];
            if let Some(dst) = word.get_mut(..field.len()) {
                dst.copy_from_slice(field);
            }
            let raw = u64::from_le_bytes(word);
            self.u64(if is_reference(f.tag) { ids.identity(f.tag, raw) } else { raw });
        }
    }

    /// A list of references, translated and hashed in identity order rather than storage order.
    pub fn references<I: SlotIdentity + ?Sized>(
        &mut self,
        tag: FieldTag,
        raw: impl IntoIterator<Item = u64>,
        ids: &I,
        scratch: &mut Vec<u64>,
    ) {
        scratch.clear();
        scratch.extend(raw.into_iter().map(|r| ids.identity(tag, r)));
        scratch.sort_unstable();
        self.u64(to_u64(scratch.len()));
        for id in scratch.iter() {
            self.u64(*id);
        }
    }

    #[must_use]
    pub fn finish(self) -> u128 {
        self.sip.finish()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use phx_id::TableId;

    use super::{IdentityHasher, LogicalHasher, Sip128, SlotIdentity};
    use crate::arena::{ChunkArena, ListRef};
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::descriptor::{ColumnDescriptor, FieldDescriptor, FieldTag, Transform};
    use crate::pod::as_bytes;
    use crate::region::Region;
    use crate::table::Table;

    type Heap = HeapBacking<4096>;

    /// The reference implementation's 128-bit vectors: key 00..0f, message 00..(n − 1) for n from 0 to 63.
    const VECTORS: [&str; 64] = [
        "a3817f04ba25a8e66df67214c7550293",
        "da87c1d86b99af44347659119b22fc45",
        "8177228da4a45dc7fca38bdef60affe4",
        "9c70b60c5267a94e5f33b6b02985ed51",
        "f88164c12d9c8faf7d0f6e7c7bcd5579",
        "1368875980776f8854527a07690e9627",
        "14eeca338b208613485ea0308fd7a15e",
        "a1f1ebbed8dbc153c0b84aa61ff08239",
        "3b62a9ba6258f5610f83e264f31497b4",
        "264499060ad9baabc47f8b02bb6d71ed",
        "00110dc378146956c95447d3f3d0fbba",
        "0151c568386b6677a2b4dc6f81e5dc18",
        "d626b266905ef35882634df68532c125",
        "9869e247e9c08b10d029934fc4b952f7",
        "31fcefac66d7de9c7ec7485fe4494902",
        "5493e99933b0a8117e08ec0f97cfc3d9",
        "6ee2a4ca67b054bbfd3315bf85230577",
        "473d06e8738db89854c066c47ae47740",
        "a426e5e423bf4885294da481feaef723",
        "78017731cf65fab074d5208952512eb1",
        "9e25fc833f2290733e9344a5e83839eb",
        "568e495abe525a218a2214cd3e071d12",
        "4a29b54552d16b9a469c10528eff0aae",
        "c9d184ddd5a9f5e0cf8ce29a9abf691c",
        "2db479ae78bd50d8882a8a178a6132ad",
        "8ece5f042d5e447b5051b9eacb8d8f6f",
        "9c0b53b4b3c307e87eaee08678141f66",
        "abf248af69a6eae4bfd3eb2f129eeb94",
        "0664da1668574b88b935f3027358aef4",
        "aa4b9dc4bf337de90cd4fd3c467c6ab7",
        "ea5c7f471faf6bde2b1ad7d4686d2287",
        "2939b0183223fafc1723de4f52c43d35",
        "7c3956ca5eeafc3e363e9d556546eb68",
        "77c6077146f01c32b6b69d5f4ea9ffcf",
        "37a6986cb8847edf0925f0f1309b54de",
        "a705f0e69da9a8f907241a2e923c8cc8",
        "3dc47d1f29c448461e9e76ed904f6711",
        "0d62bf01e6fc0e1a0d3c4751c5d3692b",
        "8c03468bca7c669ee4fd5e084bbee7b5",
        "528a5bb93baf2c9c4473cce5d0d22bd9",
        "df6a301e95c95dad97ae0cc8c6913bd8",
        "801189902c857f39e73591285e70b6db",
        "e617346ac9c231bb3650ae34ccca0c5b",
        "27d93437efb721aa401821dcec5adf89",
        "89237d9ded9c5e78d8b1c9b166cc7342",
        "4a6d8091bf5e7d651189fa94a250b14c",
        "0e33f96055e7ae893ffc0e3dcf492902",
        "e61c432b720b19d18ec8d84bdc63151b",
        "f7e5aef549f782cf379055a608269b16",
        "438d030fd0b7a54fa837f2ad201a6403",
        "a590d3ee4fbf04e3247e0d27f286423f",
        "5fe2c1a172fe93c4b15cd37caef9f538",
        "2c97325cbd06b36eb2133dd08b3a017c",
        "92c814227a6bca949ff0659f002ad39e",
        "dce850110bd8328cfbd50841d6911d87",
        "67f14984c7da791248e32bb5922583da",
        "1938f2cf72d54ee97e94166fa91d2a36",
        "74481e9646ed49fe0f6224301604698e",
        "57fca5de98a9d6d8006438d0583d8a1d",
        "9fecde1cefdc1cbed4763674d9575359",
        "e3040c00eb28f15366ca73cbd872e740",
        "7697009a6a831dfecca91c5993670f7a",
        "5853542321f567a005d547a4f04759bd",
        "5150d1772f50834a503e069a973fbd7c",
    ];

    #[test]
    fn siphash_test_vectors() {
        let key = [u64::from_le_bytes([0, 1, 2, 3, 4, 5, 6, 7]), u64::from_le_bytes([8, 9, 10, 11, 12, 13, 14, 15])];
        let message: Vec<u8> = (0..64).collect();
        for (n, expected) in VECTORS.iter().enumerate() {
            let hash = |pieces: &[&[u8]]| {
                let mut h = Sip128::new(key);
                for p in pieces {
                    h.write(p);
                }
                let out = h.finish();
                let [low, high] = [out, out >> 64].map(|w| u64::try_from(w & u128::from(u64::MAX)).unwrap());
                let bytes = [low.to_le_bytes(), high.to_le_bytes()].concat();
                bytes.iter().fold(String::new(), |mut hex, b| {
                    write!(hex, "{b:02x}").unwrap();
                    hex
                })
            };
            assert_eq!(hash(&[&message[..n]]), *expected, "whole, n={n}");
            // Fed in uneven pieces, the stream hashes the same.
            let (a, rest) = message[..n].split_at(n / 3);
            let (b, c) = rest.split_at(rest.len() / 2);
            assert_eq!(hash(&[a, b, c]), *expected, "pieces, n={n}");
        }
    }

    #[test]
    fn logical_hash_ignores_layout() {
        let mut space = AddressSpace::empty();
        let mut arena: ChunkArena<Heap> = ChunkArena::new(&mut space, 1 << 12);
        let mut scratch: Region<u64, Heap> = Region::reserve(&mut space, 1 << 12);
        let mut refs = [ListRef::EMPTY; 3];
        for round in 0..20_u64 {
            for (i, r) in (0_u64..).zip(refs.iter_mut()) {
                arena.append(r, &[round * 10 + i]);
            }
        }
        arena.remove(&mut refs[1], 0, 5);
        let mut table: Table<Heap> = Table::new(&mut space, TableId::new(0), 16, 8);
        let mut weights = table.column::<u32>(&mut space);
        for w in [5, 6, 7, 8] {
            let _ = table.slots.alloc();
            weights.push(w);
        }
        table.slots.release(phx_id::Slot::new(2));
        let hash = |arena: &ChunkArena<Heap>, refs: &[ListRef], weights: &crate::column::Column<u32, Heap>| {
            let mut h = LogicalHasher::new([1, 2]);
            h.column(weights, &table.slots);
            for r in refs {
                h.list(arena, *r);
            }
            h.finish()
        };
        let before = hash(&arena, &refs, &weights);
        assert!(arena.dead_words() > 0);
        arena.compact(refs.as_mut_slice(), &mut scratch);
        assert_eq!(hash(&arena, &refs, &weights), before);
        weights.set(phx_id::Slot::new(2), 99);
        assert_eq!(hash(&arena, &refs, &weights), before, "a freed slot's row is not content");
        weights.set(phx_id::Slot::new(1), 99);
        assert_ne!(hash(&arena, &refs, &weights), before);
    }

    /// Slot `s` of world one names identity 100 + s; world two stores the same parties in reverse.
    struct Reversed(bool);

    impl SlotIdentity for Reversed {
        fn identity(&self, tag: FieldTag, raw: u64) -> u64 {
            assert_eq!(tag, FieldTag::PartyRef);
            if self.0 { 100 + (3 - raw) } else { 100 + raw }
        }
    }

    #[test]
    fn identity_hash_ignores_storage_order() {
        const FIELDS: &[FieldDescriptor] = &[
            FieldDescriptor {
                name: "owner",
                offset: 0,
                width: 4,
                transform: Transform::Plain,
                tag: FieldTag::PartyRef,
            },
            FieldDescriptor { name: "amount", offset: 4, width: 4, transform: Transform::Plain, tag: FieldTag::Plain },
        ];
        let desc = ColumnDescriptor::checked::<[u32; 2]>("claims", 8, FIELDS).unwrap();
        // Rows by identity 100..104: (owner identity, amount); world two stores slot s at 3 − s.
        let one: [[u32; 2]; 4] = [[1, 10], [3, 20], [0, 30], [2, 40]];
        let mut two = [[0; 2]; 4];
        for (s, [owner, amount]) in one.iter().enumerate() {
            two[3 - s] = [3 - owner, *amount];
        }
        let hash = |rows: &[[u32; 2]; 4], reversed: bool| {
            let ids = Reversed(reversed);
            let mut h = IdentityHasher::new([3, 4]);
            let order: Vec<usize> = if reversed { vec![3, 2, 1, 0] } else { vec![0, 1, 2, 3] };
            for s in order {
                h.row(&desc, as_bytes(&rows[s..=s]), &ids);
            }
            let holders: Vec<u64> = if reversed { vec![3, 1] } else { vec![0, 2] };
            h.references(FieldTag::PartyRef, holders, &ids, &mut Vec::new());
            h.finish()
        };
        assert_eq!(hash(&one, false), hash(&two, true));
        assert_ne!(hash(&one, false), hash(&one, true));
    }
}
