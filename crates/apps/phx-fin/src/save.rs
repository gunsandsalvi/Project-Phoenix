//! `-F save`: the design point's party slots saved, read back and passed through the load's rebuild pass, whose time is
//! read — each base that leaves an index out of its save adds its store here with its own step; and the save's
//! encoding read a core, and its frames hashed on the pool over the design point's saved bytes.

use std::collections::BTreeMap;

use phx_exec::trace::{Reading, Spent};
use phx_exec::{Pool, PoolSpec};
use phx_id::PartyId;
use phx_rand::uniform::below_u64;
use phx_store::consts::SAVE_FRAME_BYTES;
use phx_store::hash::{frame_hash, frame_root};
use phx_store::{AddressSpace, Parties, Pod, Reader, Saved, Transform, Writer, encode_rows};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, slots};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the save contract is measured under.
pub const BASE: &str = "save";

/// A decimal millisecond in nanoseconds.
const MS: f64 = 1_000_000.0;

/// A decimal megabyte.
const MB: f64 = 1_000_000.0;

/// The ledger's lines no save holds: the build's code, the day's buffers, slack and the save's own buffers.
const UNSAVED: [&str; 4] = ["process", "day buffers", "slack", "save buffers"];

/// The key the frames are hashed under in the measure.
const KEY: [u64; 2] = [3, 5];

/// A party's row of primary words, as a column set encodes them.
type Row = [u64; 4];

/// The party slots and their rows' primary words, the saved bytes the design point holds, the pool the frames are hashed
/// on, what the last pass rebuilt and the CPU it took, the encoding's rate a core, and the frames' hashing time.
#[derive(Default)]
pub struct Save {
    parties: Option<Parties>,
    rows: Vec<Row>,
    saved_mb: f64,
    pool: Option<Pool>,
    rebuilt: u64,
    rebuild_ns: Option<u64>,
    encode_mb_s_core: Option<f64>,
    hash_ms: Option<f64>,
}

impl std::fmt::Debug for Save {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Save").field("saved_mb", &self.saved_mb).finish_non_exhaustive()
    }
}

/// A count as a float, through its text: every count here is far below the float's exact integers.
fn real(n: u64) -> Result<f64, FinError> {
    n.to_string().parse().map_err(|e| FinError(format!("{n}: {e}")))
}

impl Save {
    /// The rows encoded as a save writes them, its rate read a core; then as many 1 MiB frames of the encoded bytes as
    /// the design point saves, hashed on the pool and combined, its wall time read.
    fn encode_and_hash(&mut self, m: &mut Measures<'_>) -> Result<(), FinError> {
        let mut fields = Vec::new();
        Row::layout(0, Transform::Plain, &mut fields);
        let rows = &self.rows;
        let input = real(crate::kept::wide(size_of_val(rows.as_slice())))?;
        let (stream, spent) = m.read(BASE, "encode", 1, || {
            let before = Reading::now();
            let stream = encode_rows(&fields, rows);
            (stream, Spent::between(Some(1), &before, &Reading::now()))
        });
        self.encode_mb_s_core = spent.cpu_ns.and_then(|ns| real(ns).ok()).map(|ns| input / MB / (ns / (MS * 1_000.0)));
        let frames: Vec<&[u8]> = stream.chunks(SAVE_FRAME_BYTES).collect();
        let saved = format!("{:.0}", self.saved_mb * MB).parse::<u64>().map_err(|e| FinError(e.to_string()))?;
        let count = crate::kept::index(saved.div_ceil(crate::kept::wide(SAVE_FRAME_BYTES)))?;
        let pool = self.pool.as_ref();
        let chunks = crate::kept::slots(crate::kept::wide(count))?;
        let root = m.read(BASE, "hash", 1, || {
            let leaves = phx_exec::for_chunks(pool, chunks, |i| {
                let at = crate::kept::index(u64::from(i)).ok()?;
                frames.get(at % frames.len()).map(|f| frame_hash(KEY, f))
            });
            let leaves: Option<Vec<u128>> = leaves.into_iter().collect();
            leaves.map(|l| frame_root(KEY, &l))
        });
        if root.is_none() {
            return Err(FinError("no encoded frame to hash".to_owned()));
        }
        let wall = m.iter().find(|((b, op), _)| b == BASE && op == "hash").and_then(|(_, w)| w.wall_ns);
        self.hash_ms = wall.and_then(|ns| real(ns).ok()).map(|ns| ns / MS);
        Ok(())
    }
}

impl FinBase for Save {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] directory_slots` parties begun in one kind's table.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let rows = count(&design.store, "directory_slots", "store")?;
        let mut space = AddressSpace::empty();
        let mut parties = Parties::new(&mut space, 1, slots(rows)?, phx_core::consts::CHUNK_ROWS);
        for id in 1..=rows {
            let _ = parties.begin(PartyId::new(id));
        }
        self.parties = Some(parties);
        // Words of mixed widths, as a row's identity, counts, amounts and dates encode, each row its own subject's draws.
        let widths = [u64::from(u32::MAX), 1 << 12, 1 << 40, 1 << 24];
        self.rows = (0..rows)
            .map(|row| {
                let mut d = streams.draws(BASE, row, 0);
                widths.map(|w| below_u64(&mut d, w))
            })
            .collect();
        self.saved_mb =
            design.ledger()?.iter().filter(|(name, _)| !UNSAVED.contains(&name.as_str())).map(|(_, mb)| mb).sum();
        self.pool = Some(Pool::new(&PoolSpec::detect()).map_err(|e| FinError(e.0))?);
        Ok(Filled { rows })
    }

    /// On a business day, the store saved, read back and rebuilt; the pass's time kept.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        if day != DayType::B {
            return Ok(());
        }
        let Some(parties) = self.parties.as_ref() else {
            return Err(FinError("the save measured before its fill".to_owned()));
        };
        let mut bytes = Vec::new();
        let mut w = Writer::new(&mut bytes).map_err(|e| FinError(e.to_string()))?;
        parties.save(&mut w);
        w.finish().map_err(|e| FinError(e.to_string()))?;
        let mut source = bytes.as_slice();
        let mut r = Reader::new(&mut source).map_err(|e| FinError(e.to_string()))?;
        let mut back: Parties = Parties::load(&mut r).map_err(|e| FinError(format!("{e:?}")))?;
        let (rebuilt, spent) = m.read(BASE, "rebuild", 1, || {
            let before = Reading::now();
            let rebuilt = phx_store::rebuild(&mut back);
            (rebuilt, Spent::between(Some(1), &before, &Reading::now()))
        });
        self.rebuild_ns = spent.cpu_ns;
        self.rebuilt = phx_store::StoreStats::rows_live(&rebuilt.map_err(|e| FinError(e.to_string()))?);
        self.encode_and_hash(m)
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.parties.as_ref().map_or(0, phx_store::StoreStats::bytes), resident: 0 }
    }

    /// The pass's milliseconds and the rows it rebuilt; the encoding's MB a second a core; the frames' hashing time
    /// and the MB it hashed.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let ms = self.rebuild_ns.and_then(|ns| real(ns).ok()).map(|ns| ns / MS);
        [
            ("rebuild_ms", ms),
            ("rebuilt_rows", real(self.rebuilt).ok()),
            ("encode_mb_s_core", self.encode_mb_s_core),
            ("hash_ms", self.hash_ms),
            ("saved_mb", Some(self.saved_mb)),
        ]
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .collect()
    }
}
