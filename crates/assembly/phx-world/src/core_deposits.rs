//! Deposits on the core. The right to extract a deposit is a holding over its tile: at the opening each is
//! held by the firm of its region making the product that draws on its resource sited nearest it, and passes with its
//! holder's goods. An extractor decides on its extraction schedule, by Hotelling's rule, whether to work what it holds;
//! working, it can make no more than its deposits give at its way's draw a unit, and each unit it makes takes that
//! draw from them, a finite deposit never below nothing. What each finite deposit has given and holds is kept apart,
//! and the deposits family holds their sum to what it opened with.

use std::collections::BTreeMap;

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_geo::GeoState;
use phx_geo::deposits::Opening;
use phx_id::{Day, PartyKey, Slot};
use phx_macros::{clause, opening};
use phx_num::{Missing, QtyRaw, violation};
use phx_rand::float::from_i64;

use crate::consts::firm::{PRODUCT, REGION, SITE};
use crate::core::{Core, kind_number};

/// The deposits: each one's holder, what it has given and, where finite, what it holds and opened with, and its
/// resource, by its place among the map's; each product's resource and draw a unit, the days between an extractor's decisions, each extractor's deposits and
/// whether it works them as it last decided.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Deposits {
    pub holders: Vec<Option<PartyKey>>,
    pub extracted: Vec<i64>,
    pub remaining: Vec<Option<i64>>,
    pub opening: Vec<Option<i64>>,
    pub resources: Vec<u16>,
    draws: Vec<Option<(u16, i64)>>,
    days: u32,
    pub held: BTreeMap<PartyKey, Vec<u32>>,
    pub working: BTreeMap<PartyKey, bool>,
}

/// A draw a unit as the technology states it.
fn per_unit(raw: i64) -> if_base::PerUnit {
    if_base::PerUnit::from_raw(raw)
}

impl Core {
    /// Each deposit's right given to the firm of its region making the product that draws on its resource, sited
    /// nearest it; a deposit no such firm's region holds is held by no one.
    ///
    /// # Errors
    /// Products or tables the register refuses.
    #[clause("GDS.3", "GEO.6", "GEN.2")]
    #[opening]
    pub(crate) fn open_rights(&mut self, geo: &GeoState, register: &phx_core::Register) -> Result<(), String> {
        let draws: Vec<Option<(u16, i64)>> =
            sys_tec::deposit_draws(register)?.into_iter().map(|d| d.map(|(r, per)| (r, per.raw()))).collect();
        let days = u32::try_from(register.count(sys_gds::EXTRACTION_DAYS.id)?).map_err(|e| e.to_string())?;
        let Some(firm) = self.bound.kinds.firm else { return Ok(()) };
        let word = |store: &phx_core::store::KindStore<_>, s: Slot, at: usize| match store.record(s).get(at) {
            Some(w) => match w.get() {
                Missing::Present(v) => u32::try_from(v).ok(),
                Missing::Absent => None,
            },
            None => None,
        };
        let mut firms: Vec<(Slot, u16, u32, u32)> = Vec::new();
        if let Some(store) = self.kinds.get(firm) {
            for s in store.parties.live_slots() {
                let (Some(p), Some(r), Some(t)) =
                    (word(store, s, PRODUCT), word(store, s, REGION), word(store, s, SITE))
                else {
                    continue;
                };
                if let Ok(p) = u16::try_from(p) {
                    firms.push((s, p, r, t));
                }
            }
        }
        let mut d = Deposits { draws, days, ..Deposits::default() };
        for (i, dep) in (0_u32..).zip(&geo.deposits) {
            let region = match geo.zone_of(dep.tile) {
                Missing::Present(z) => geo.zone_region(z),
                Missing::Absent => Missing::Absent,
            };
            let mut nearest: Option<(u64, Slot)> = None;
            for (s, p, r, t) in &firms {
                let draws_on =
                    d.draws.get(usize::from(*p)).copied().flatten().is_some_and(|(res, _)| res == dep.resource);
                if !draws_on || region != Missing::Present(*r) {
                    continue;
                }
                let far = geo.map.grid.plane_m(phx_id::TileId::new(*t), dep.tile);
                if nearest.is_none_or(|(f, _)| far < f) {
                    nearest = Some((far, *s));
                }
            }
            let holder = nearest.map(|(_, s)| PartyKey::new(kind_number(firm), s));
            if let Some(h) = holder {
                d.held.entry(h).or_default().push(i);
            }
            let opening = match dep.opening {
                Opening::Finite(q) => Some(i64::try_from(q).map_err(|_| format!("deposit {i}'s quantity"))?),
                Opening::Unbounded => None,
            };
            d.holders.push(holder);
            d.extracted.push(0);
            d.remaining.push(opening);
            d.opening.push(opening);
            d.resources.push(dep.resource);
        }
        self.deposits = d;
        Ok(())
    }

    /// The resource and draw a unit of a product extracted; none for any other.
    pub(crate) fn draw_of(&self, product: u16) -> Option<(u16, i64)> {
        self.deposits.draws.get(usize::from(product)).copied().flatten()
    }

    /// The units of a product its maker's deposits give now: none where it does not work them or holds none of its
    /// resource, without end where one it works is.
    #[clause("GDS.12", "GEO.9")]
    pub(crate) fn deposit_room(&self, maker: PartyKey, product: u16) -> f64 {
        let Some((resource, raw)) = self.draw_of(product) else { return f64::INFINITY };
        if self.deposits.working.get(&maker) != Some(&true) {
            return 0.0;
        }
        let per = per_unit(raw).to_f64();
        let mut room = 0.0;
        for i in self.deposits.held.get(&maker).into_iter().flatten() {
            let at = usize::try_from(*i).unwrap_or(usize::MAX);
            if self.deposit_resource(at) != Some(resource) {
                continue;
            }
            match self.deposits.remaining.get(at).copied().flatten() {
                Some(left) => room += from_i64(left) / per,
                None => return f64::INFINITY,
            }
        }
        room
    }

    fn deposit_resource(&self, at: usize) -> Option<u16> {
        self.deposits.resources.get(at).copied()
    }

    /// What `units` of a product made take from its maker's deposits of its resource, in their order, a finite one
    /// no further than it holds; what they cannot give stops the run, as the making was held to their room.
    #[clause("GEO.9", "GEO.12", "GDS.12")]
    pub(crate) fn take_from_deposits(&mut self, maker: PartyKey, product: u16, units: i64) {
        let Some((resource, raw)) = self.draw_of(product) else { return };
        let mut left = sys_tec::ways::takes(QtyRaw::from_raw(units), per_unit(raw)).raw();
        let held = self.deposits.held.get(&maker).cloned().unwrap_or_default();
        for i in held {
            let at = usize::try_from(i).unwrap_or(usize::MAX);
            if left <= 0 || self.deposit_resource(at) != Some(resource) {
                continue;
            }
            let take = match self.deposits.remaining.get(at).copied().flatten() {
                Some(r) if r < left => r,
                _ => left,
            };
            if let Some(Some(r)) = self.deposits.remaining.get_mut(at) {
                *r -= take;
            }
            if let Some(e) = self.deposits.extracted.get_mut(at) {
                *e += take;
            }
            left -= take;
        }
        if left > 0 {
            violation!(clause = "GDS.12", "a unit made beyond what its deposits give", maker = maker.word());
        }
    }

    /// Each extractor whose schedule comes today, or which has not yet decided, decides whether to work its deposits:
    /// by Hotelling's rule, its price net of its cost now against the price it expects at its next decision,
    /// discounted at the return it requires.
    #[clause("GDS.4", "MND.20")]
    pub(crate) fn review_extraction(&mut self, regions: &[phx_id::CountryId], day: Day) {
        let Some(firm) = self.bound.kinds.firm else { return };
        let days = self.deposits.days;
        if days == 0 {
            return;
        }
        let deciding = self.point(|p| p.extract, &sys_gds::points::EXTRACT);
        let years = f64::from(days) / crate::consts::DAYS_A_YEAR;
        let mut decided = Vec::new();
        for slot in self.firm_slots(firm) {
            let key = PartyKey::new(kind_number(firm), slot);
            let due = (day.get() + slot.get()).is_multiple_of(days);
            if !due && self.deposits.working.contains_key(&key) {
                continue;
            }
            let Some(f) = self.goods_firm(regions, firm, slot) else { continue };
            if self.draw_of(f.product).is_none() {
                continue;
            }
            let (Some(cost), Some(lot)) = (self.unit_cost(&f), phx_rand::float::floor_to_i64(self.lot(f.product)))
            else {
                continue;
            };
            let price = from_i64(f.price) / from_i64(lot);
            let prefs = self.decider(deciding, key).1;
            let Missing::Present(rate) = prefs.required_return else {
                violation!(clause = "FRM.15", "an extractor with no required return", firm = key.word());
            };
            let outlook = match (self.goods.outlooks.view(&prefs), prefs.stance) {
                (Some(view), Missing::Present(h)) => {
                    match self.goods.outlooks.outlook((f.product, f.region), view, usize::from(h)) {
                        Missing::Present(mark) => mark / from_i64(lot),
                        Missing::Absent => price,
                    }
                }
                _ => price,
            };
            let finite = self.deposits.held.get(&key).into_iter().flatten().any(|i| {
                self.deposits.remaining.get(usize::try_from(*i).unwrap_or(usize::MAX)).copied().flatten().is_some()
            });
            let works = self.decide(deciding, key, |_| sys_gds::points::ExtractIn {
                price,
                cost,
                outlook,
                rate,
                years,
                finite,
            });
            decided.push((key, works));
        }
        self.deposits.working.extend(decided);
    }

    /// A holder's rights passed to its successor, as its goods pass.
    #[clause("GDS.3", "PTY.9")]
    pub(crate) fn pass_rights(&mut self, from: PartyKey, to: PartyKey) {
        let Some(held) = self.deposits.held.remove(&from) else { return };
        for i in &held {
            if let Some(h) = self.deposits.holders.get_mut(usize::try_from(*i).unwrap_or(usize::MAX)) {
                *h = Some(to);
            }
        }
        self.deposits.held.entry(to).or_default().extend(held);
        self.deposits.working.remove(&from);
    }

    /// Each finite deposit's given and held against what it opened with; a difference is a finding of the deposits
    /// family.
    #[clause("GEO.12", "N1", "II.5")]
    pub(crate) fn audit_deposits(&mut self, day: Day) {
        let d = &self.deposits;
        let mut found = Vec::new();
        for (i, ((open, left), given)) in d.opening.iter().zip(&d.remaining).zip(&d.extracted).enumerate() {
            let (Some(open), Some(left)) = (open, left) else { continue };
            if given + left != *open {
                found.push(Finding {
                    family: "deposits",
                    clause: "GEO.12",
                    owner: FindingOwner::Run,
                    size: i128::from(given + left - open),
                    unit: Unit::Count,
                    day,
                    detail: format!("deposit {i}: {given} extracted and {left} remaining of {open}"),
                });
            }
        }
        self.found.extend(found);
    }
}
