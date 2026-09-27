//! The firms' stances as the kernel reads them at 5a, after the public outlooks: a public series a method was
//! surprised by wakes the reviews of the firms whose stance reads it by that method, and the day's stances are
//! counted — the methods in use and the firms relying on each heuristic.

use std::collections::BTreeSet;

use if_firm::facts::Method;
use if_firm::known::Product;
use phx_core::{FactDef, FactStore, WakeKind};
use phx_id::{Day, MarketId, Slot};
use phx_ledger::goods::GoodKey;
use phx_macros::clause;
use phx_num::Missing;
use phx_pop::population::Population;
use phx_store::SystemBacking;

use crate::world::World;

/// What the day's stances were: the public surprises recorded, the reviews they woke, the methods held, and the firms
/// relying on each heuristic, by its place on the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StanceDay {
    pub day: Day,
    pub surprises: u64,
    pub woken: u64,
    pub methods: u64,
    pub by_heuristic: Vec<u64>,
}

impl World {
    /// The series a firm's stance reads: its product's market between firms where it stands, or else its country's
    /// retail market for it.
    fn stance_series(&self, product: u16, zone: phx_id::ZoneId) -> Missing<MarketId> {
        let key = GoodKey { product, grade: 0, zone };
        for name in ["GDS.between_firms", "GDS.commodities"] {
            if let Missing::Present(kind) = self.market_kinds.kind(phx_ledger::instruction::name_code(name))
                && let Missing::Present(m) = self.markets.made.of(kind, key.code())
            {
                return Missing::Present(m);
            }
        }
        let Missing::Present(country) = self.geo().zone_country(zone) else { return Missing::Absent };
        let subject = crate::retail::retail_subject(product, country);
        self.trade
            .retail
            .iter()
            .find_map(|r| match self.markets.made.of(r.kind, subject) {
                Missing::Present(m) => Some(m),
                Missing::Absent => None,
            })
            .map_or(Missing::Absent, Missing::Present)
    }

    /// 5a: each visit that answers surprises wakes the rows whose stance reads a series its method was surprised by,
    /// its review drawn for the first day it can still run; and the day's stances counted.
    #[clause("REP.35", "VAL.4", "VAL.7")]
    pub(crate) fn stance_wakes(&mut self, day: Day) {
        let surprised: BTreeSet<(MarketId, u16)> = std::mem::take(&mut self.market_day.surprises).into_iter().collect();
        let types = self.val_methods.iter().map(|(m, _)| m.memory).collect::<BTreeSet<_>>().len();
        let mut stances = StanceDay {
            day,
            surprises: phx_rand::float::len_u64(surprised.len()),
            woken: 0,
            methods: 0,
            by_heuristic: vec![0; phx_val::heuristic::MENU.len()],
        };
        let mut held: BTreeSet<u16> = BTreeSet::new();
        let first = self.books.parties.first_cell_place();
        let visits: Vec<crate::visits::Bound> =
            self.visits.iter().filter(|b| b.decl.wakes.contains(&WakeKind::Surprise)).copied().collect();
        let mut wake: Vec<(crate::visits::Bound, Slot)> = Vec::new();
        let mut counted: BTreeSet<u16> = BTreeSet::new();
        for b in &visits {
            let place = b.table.get();
            let rows = crate::goods::Rows { place, individuals: b.individuals };
            let k = if b.individuals {
                0
            } else {
                let Some(k) = place.checked_sub(first).map(usize::from) else { continue };
                k
            };
            let slots: Vec<Slot> = if b.individuals {
                self.books.parties.table(place).slots().collect()
            } else {
                Population::table::<SystemBacking>(self.books.parties.cells(), k).slots().collect()
            };
            let facts: Vec<(Slot, Missing<i64>, Missing<i64>)> = {
                let store: &mut dyn FactStore = if b.individuals {
                    self.books.parties.table_mut(place)
                } else {
                    Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, k)
                };
                slots
                    .iter()
                    .map(|s| (*s, store.read(Product::ITEM.name, *s), store.read(Method::ITEM.name, *s)))
                    .collect()
            };
            // A table's firms are counted once however many of its visits answer surprises.
            let count = counted.insert(place);
            for (slot, product, method) in facts {
                let (Missing::Present(product), Missing::Present(method)) = (product, method) else { continue };
                let (Ok(product), Ok(method)) = (u16::try_from(product), u16::try_from(method)) else { continue };
                if count {
                    held.insert(method);
                    if let Some(n) =
                        usize::from(method).checked_div(types).and_then(|h| stances.by_heuristic.get_mut(h))
                    {
                        *n += 1;
                    }
                }
                if surprised.is_empty() {
                    continue;
                }
                let Some(row) = self.goods_row(rows, slot) else { continue };
                if let Missing::Present(series) = self.stance_series(product, row.zone)
                    && surprised.contains(&(series, method))
                {
                    wake.push((*b, slot));
                }
            }
        }
        for (b, slot) in wake {
            let agenda = &mut self.population.visits;
            // A review due today can no longer be drawn for today once the day's agenda is read, so it is tomorrow's.
            let at = if agenda.today() < day { day } else { day.succ() };
            if agenda.next(b.table, slot, b.reason).is_none_or(|d| d > at) {
                agenda.set_next(b.table, slot, b.reason, at);
                stances.woken += 1;
            }
        }
        stances.methods = phx_rand::float::len_u64(held.len());
        self.stance_days.push(stances);
    }
}
