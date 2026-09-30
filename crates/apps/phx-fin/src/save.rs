//! `-F save`, its rebuild part: the design point's party slots saved, read back and passed through the load's rebuild
//! pass, whose time is read. Each base that leaves an index out of its save adds its store here with its own step.

use std::collections::BTreeMap;

use phx_exec::trace::{Reading, Spent};
use phx_id::PartyId;
use phx_store::{AddressSpace, Parties, Reader, Saved, Writer};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, slots};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the save contract is measured under.
pub const BASE: &str = "save";

/// A decimal millisecond in nanoseconds.
const MS: f64 = 1_000_000.0;

/// The party slots, the rows the last pass rebuilt, and the CPU it took.
#[derive(Debug, Default)]
pub struct Save {
    parties: Option<Parties>,
    rebuilt: u64,
    rebuild_ns: Option<u64>,
}

impl FinBase for Save {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] directory_slots` parties begun in one kind's table.
    fn fill(&mut self, design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        let rows = count(&design.store, "directory_slots", "store")?;
        let mut space = AddressSpace::empty();
        let mut parties = Parties::new(&mut space, 1, slots(rows)?, phx_core::consts::CHUNK_ROWS);
        for id in 1..=rows {
            let _ = parties.begin(PartyId::new(id));
        }
        self.parties = Some(parties);
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
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.parties.as_ref().map_or(0, phx_store::StoreStats::bytes), resident: 0 }
    }

    /// The pass's milliseconds, and the rows it rebuilt.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let ms = self.rebuild_ns.and_then(|ns| ns.to_string().parse::<f64>().ok()).map(|ns| ns / MS);
        let rows = self.rebuilt.to_string().parse::<f64>().ok();
        [("rebuild_ms", ms), ("rebuilt_rows", rows)].into_iter().filter_map(|(k, v)| Some((k, v?))).collect()
    }
}
