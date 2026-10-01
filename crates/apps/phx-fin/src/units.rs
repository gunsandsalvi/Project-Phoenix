//! `-F units`: the units registry at the design point — about a hundred and ten units a zone over its zones: goods
//! (product-grades), capital classes, and the instruments, special units and rights beside them — issued once a day
//! into an empty registry, then each found again by its key, as a party's changed units are resolved.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::catalogue::{Declared, ProductDecl, compile};
use phx_core::unit_registry::{CapitalClass, UnitKey, UnitRegistry, UnitTraits};
use phx_store::StoreStats;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::wide;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the registry is measured under.
pub const BASE: &str = "units";

/// A zone's units: product-grades, capital classes, and its instruments, special units and rights.
const GOODS: u16 = 60;
const CLASSES: u8 = 45;
const INSTRUMENTS: u32 = 2;
const SPECIALS: u32 = 2;
const RIGHTS: u32 = 1;
/// A capital kind's age classes, so a zone's classes span kinds by age.
const AGES: u8 = 9;

const TRAITS: UnitTraits = UnitTraits { storable: true, perishable: false };

const MIB: f64 = 1_048_576.0;

/// Every key the design point's registry holds, in the order they are first named.
#[derive(Debug, Default)]
pub struct Units {
    keys: Vec<UnitKey>,
    registry: Option<UnitRegistry>,
    folded: u32,
}

fn zone_of(z: u64) -> Result<u16, FinError> {
    u16::try_from(z).map_err(|e| FinError(format!("zone {z}: {e}")))
}

impl FinBase for Units {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The design point's zones' keys: each zone's goods, classes, instruments, special units and rights.
    fn fill(&mut self, design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let (zones, ids) = (store("zones")?, store("unit_ids")?);
        let names: Vec<String> = (0..GOODS).map(|p| format!("product{p}")).collect();
        let products: Vec<ProductDecl<'_>> =
            names.iter().map(|name| ProductDecl { system: "GDS", name, grades: 1 }).collect();
        let catalogue =
            compile(&Declared { products: &products, ..Declared::default() }).map_err(|e| FinError(e.join("; ")))?;
        let mut instruments = 0;
        for z in 0..zones {
            let zone = zone_of(z)?;
            self.keys.extend(catalogue.products().map(|product| UnitKey::Good {
                product: product.get(),
                grade: 0,
                zone,
            }));
            for c in 0..CLASSES {
                let class = CapitalClass { kind: c / AGES, size: 0, quality: 0, condition: 0, age: c % AGES };
                self.keys.push(UnitKey::Capital { class, zone });
            }
            for _ in 0..INSTRUMENTS {
                self.keys.push(UnitKey::Instrument { id: instruments });
                instruments += 1;
            }
            self.keys.extend((0..SPECIALS).map(|kind| UnitKey::Special { kind, place: zone }));
            self.keys.extend((0..RIGHTS).map(|r| UnitKey::Right { deposit: u32::from(zone) * RIGHTS + r }));
        }
        if wide(self.keys.len()) != ids {
            return Err(FinError(format!("{} units where the design point holds {ids}", self.keys.len())));
        }
        Ok(Filled { rows: ids })
    }

    /// Every unit issued into an empty registry, then each found again by its key.
    fn day(&mut self, _day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let keys = &self.keys;
        let n = wide(keys.len());
        let mut registry = UnitRegistry::new();
        self.folded ^= m.read(BASE, "issue", n, || {
            let mut fold = 0_u32;
            for k in keys {
                fold ^= registry.issue(*k, TRAITS).index();
            }
            black_box(fold)
        });
        let reg = &registry;
        self.folded ^= m.read(BASE, "find", n, || {
            let mut fold = 0_u32;
            for k in keys {
                if let phx_num::Missing::Present(id) = reg.find(*k) {
                    fold ^= id.index();
                }
            }
            black_box(fold)
        });
        self.registry = Some(registry);
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.registry.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let bytes = self.registry.as_ref().map(|r| r.bytes().to_string().parse::<f64>());
        match bytes {
            Some(Ok(b)) => vec![("ids_mb", b / MIB)],
            _ => Vec::new(),
        }
    }
}
