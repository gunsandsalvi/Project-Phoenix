//! The households' own state on their kind's store: a household's zone, its stance and preference type, whether it
//! tries for a child and its ideal number of children, its age class, its outlook of its income and the positions that
//! outlook is counted from, each a typed word of its hot or warm row. What a word holds is read back in the units the
//! rules take: a region through the map from its zone, a stance from its bits of the states byte, a memory and a
//! switching type from the preference type, a month of its last look from the run's first.

use phx_id::{PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::directory::Directory;
use phx_pop::kinds::{AttrW, KindStore, Opening, Row};
use phx_pop::layout::{HOUSEHOLD, Layout};
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::household::{STANCE_BITS, STANCE_SHIFT, TRYING};

/// The groups a visit gathers: the hot row and the warm one.
const HOT: u8 = 0;
const WARM: u8 = 1;

/// The stance's bits of the states byte, read and written in place.
const STANCE_MASK: u8 = ((1 << STANCE_BITS) - 1) << STANCE_SHIFT;
/// Every heuristic of the menu has a stance the states byte holds.
const _: () = assert!(phx_val::heuristic::MENU.len() <= 1 << STANCE_BITS);

/// The household's words' write handles.
#[derive(Clone, Copy, Debug)]
struct Words {
    residence: AttrW<u32>,
    states: AttrW<u8>,
    preference: AttrW<u16>,
    flags: AttrW<u8>,
    received: AttrW<i64>,
    after: AttrW<i64>,
    income: AttrW<i64>,
    looked: AttrW<u16>,
    window: AttrW<u8>,
    ideal: AttrW<u8>,
}

fn words() -> Words {
    let Ok(mut l) = Layout::compile(&HOUSEHOLD, &[]) else {
        violation!(clause = "REP.1", "the household's layout refused");
    };
    macro_rules! w {
        ($t:ty, $name:expr, $base:expr) => {
            match l.writer::<$t>($name, 0, $base) {
                Ok(a) => a,
                Err(_) => violation!(clause = "REP.1", "a household word's handle refused"),
            }
        };
    }
    Words {
        residence: w!(u32, "residence", "K-32"),
        states: w!(u8, "states", "K-32"),
        preference: w!(u16, "preference", "K-32"),
        flags: w!(u8, "flags", "K-32"),
        received: w!(i64, "received", "K-32"),
        after: w!(i64, "after", "K-32"),
        income: w!(i64, "income", "K-102"),
        looked: w!(u16, "looked", "K-32"),
        window: w!(u8, "window", "K-32"),
        ideal: w!(u8, "ideal", "K-32"),
    }
}

/// The region of each of the map's zones, which the zoned kinds' stores read a party's region through.
#[phx_macros::opening]
#[must_use]
pub fn zone_regions(geo: &phx_geo::GeoState) -> Vec<u32> {
    geo.map.zones.iter().map(|z| u32::from(z.region.get())).collect()
}

/// A household as the opening begins it: its zone, its preference type and first stance, its age class, and its
/// outlook of its income a year, from what it is owed over the year after the opening.
#[derive(Clone, Copy, Debug)]
pub struct HouseholdOpening {
    pub zone: u16,
    pub preference: u16,
    pub stance: u8,
    pub window: Missing<u8>,
    pub income: i64,
}

/// The households' store: their rows by slot, the map's region of each zone, and the run's first month, from which a
/// look's month is counted.
#[clause("REP.20", "REP.41")]
#[derive(Debug, phx_macros::Saved)]
pub struct HouseholdStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    zone_regions: Vec<u32>,
    first_month: i64,
    #[saved(skip, rebuild = HouseholdStore::bind)]
    words: Option<Words>,
}

impl HouseholdStore {
    /// The households' store: room for `capacity` households of kind `kind`, the region of each of the map's zones and
    /// the run's first month.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(
        space: &mut AddressSpace,
        kind: u8,
        capacity: u32,
        (zone_regions, first_month): (Vec<u32>, i64),
    ) -> HouseholdStore {
        let Ok(layout) = Layout::compile(&HOUSEHOLD, &[]) else {
            violation!(clause = "REP.1", "the household's layout refused");
        };
        let store = KindStore::new(space, kind, &layout, capacity);
        HouseholdStore { store, kind, zone_regions, first_month, words: Some(words()) }
    }

    /// A loaded store's handles, compiled again from the household's layout.
    fn bind(&mut self) -> u64 {
        self.words = Some(words());
        0
    }

    #[inline]
    fn w(&self) -> &Words {
        match &self.words {
            Some(w) => w,
            None => violation!(clause = "REP.1", "a household store read before its handles are bound"),
        }
    }

    /// The kind its households are of.
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// A household the directory began, its words written from its opening.
    pub fn begin(&mut self, dir: &Directory<SystemBacking>, r: PartyRef, o: &HouseholdOpening) {
        let w = *self.w();
        if u32::from(o.stance) >= 1 << STANCE_BITS {
            violation!(clause = "VAL.7", "a stance past the states byte's bits", stance = o.stance);
        }
        let mut opening = vec![
            Opening::of(w.residence, u32::from(o.zone)),
            Opening::of(w.states, o.stance << STANCE_SHIFT),
            Opening::of(w.preference, o.preference),
            Opening::of(w.income, o.income),
        ];
        if let Missing::Present(c) = o.window {
            opening.push(Opening::of(w.window, c));
        }
        self.store.begin(dir, r, &opening);
    }

    /// A household's hot and warm rows at a slot, gathered once for its visit's reads: the day names a household by
    /// its place, which no other party takes before the day closes. None for a slot no household was begun at.
    #[inline]
    #[must_use]
    pub fn view(&self, slot: Slot) -> Option<HouseholdView<'_>> {
        Some(HouseholdView { hot: self.store.gather_at(slot, HOT)?, warm: self.store.gather_at(slot, WARM)?, hs: self })
    }

    fn set<T: phx_pop::kinds::Word>(&mut self, slot: Slot, a: AttrW<T>, v: Missing<T>) {
        self.store.set_at(slot, a, v);
    }

    /// The stance a household's outlooks rely on, written to its bits of the states byte.
    pub fn set_stance(&mut self, slot: Slot, stance: u16) {
        let Some(stance) = u8::try_from(stance).ok().filter(|s| u32::from(*s) < 1 << STANCE_BITS) else {
            violation!(clause = "VAL.7", "a stance past the states byte's bits", stance = stance);
        };
        let states = self.w().states;
        let Some(held) = self.view(slot).and_then(|v| present(v.hot.get(states.read()))) else {
            violation!(clause = "REP.1", "a stance written for no household", slot = slot.get());
        };
        self.set(slot, states, Missing::Present((held & !STANCE_MASK) | (stance << STANCE_SHIFT)));
    }

    /// Whether a household tries for a child, written to its flag.
    pub fn set_trying(&mut self, slot: Slot, trying: bool) {
        let flags = self.w().flags;
        let Some(held) = self.view(slot).and_then(|v| present(v.hot.get(flags.read()))) else {
            violation!(clause = "REP.1", "a flag written for no household", slot = slot.get());
        };
        let v = if trying { held | TRYING } else { held & !TRYING };
        self.set(slot, flags, Missing::Present(v));
    }

    pub fn set_ideal(&mut self, slot: Slot, ideal: Missing<u8>) {
        let w = self.w().ideal;
        self.set(slot, w, ideal);
    }

    pub fn set_window(&mut self, slot: Slot, window: Missing<u8>) {
        let w = self.w().window;
        self.set(slot, w, window);
    }

    pub fn set_received(&mut self, slot: Slot, money: i64) {
        let w = self.w().received;
        self.set(slot, w, Missing::Present(money));
    }

    pub fn set_after(&mut self, slot: Slot, money: i64) {
        let w = self.w().after;
        self.set(slot, w, Missing::Present(money));
    }

    pub fn set_income(&mut self, slot: Slot, money: i64) {
        let w = self.w().income;
        self.set(slot, w, Missing::Present(money));
    }

    /// The month of a household's last look at its income, counted in months from the calendar's year zero.
    pub fn set_looked(&mut self, slot: Slot, month: i64) {
        let Some(off) = month.checked_sub(self.first_month).and_then(|m| u16::try_from(m).ok()) else {
            violation!(clause = "TIME.10", "a look's month outside the run's offsets", month = month);
        };
        let w = self.w().looked;
        self.set(slot, w, Missing::Present(off));
    }
}

/// A household's gathered rows and the store that reads them.
#[derive(Clone, Copy, Debug)]
pub struct HouseholdView<'a> {
    hot: Row<'a>,
    warm: Row<'a>,
    hs: &'a HouseholdStore,
}

fn present<T>(m: Missing<T>) -> Option<T> {
    match m {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

impl HouseholdView<'_> {
    #[must_use]
    pub fn zone(&self) -> Option<u16> {
        present(self.hot.get(self.hs.w().residence.read())).and_then(|z| u16::try_from(z).ok())
    }

    /// The region its zone lies in.
    #[must_use]
    pub fn region(&self) -> Option<u32> {
        self.hs.zone_regions.get(usize::from(self.zone()?)).copied()
    }

    /// The heuristic its outlooks rely on.
    #[must_use]
    pub fn stance(&self) -> Option<u16> {
        let states = present(self.hot.get(self.hs.w().states.read()))?;
        Some(u16::from((states & STANCE_MASK) >> STANCE_SHIFT))
    }

    /// Its memory and switching types.
    #[must_use]
    pub fn types(&self) -> Option<(u16, u16)> {
        sys_hh::preference_parts(present(self.hot.get(self.hs.w().preference.read()))?)
    }

    #[must_use]
    pub fn trying(&self) -> Option<bool> {
        present(self.hot.get(self.hs.w().flags.read())).map(|f| f & TRYING != 0)
    }

    pub fn ideal(&self) -> Missing<u8> {
        self.warm.get(self.hs.w().ideal.read())
    }

    /// Its head's age class at the last year's close.
    pub fn window(&self) -> Missing<u8> {
        self.hot.get(self.hs.w().window.read())
    }

    /// Its income received since it last looked.
    #[must_use]
    pub fn received(&self) -> Option<i64> {
        present(self.hot.get(self.hs.w().received.read()))
    }

    /// What it held after it last spent.
    #[must_use]
    pub fn after(&self) -> Option<i64> {
        present(self.hot.get(self.hs.w().after.read()))
    }

    /// Its outlook of its income a year.
    #[must_use]
    pub fn income(&self) -> Option<i64> {
        present(self.hot.get(self.hs.w().income.read()))
    }

    /// The month it last looked at its income, counted from the calendar's year zero.
    #[must_use]
    pub fn looked(&self) -> Option<i64> {
        present(self.hot.get(self.hs.w().looked.read())).map(|m| self.hs.first_month + i64::from(m))
    }
}

#[cfg(test)]
#[path = "household_store_tests.rs"]
mod tests;
