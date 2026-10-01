//! Where the parties of a kind placed by a site, a region or a country are, each a word of its kind's store: the tile
//! of its site, its region, or its country, by what the kind declares, from which its country is read.

use phx_core::Place;
use phx_id::{PartyRef, Slot};
use phx_num::{Missing, violation};
use phx_pop::directory::Directory;
use phx_pop::kinds::{AttrW, KindStore, Opening};
use phx_pop::layout::{COUNTRIED, Layout, REGIONED, SITED};
use phx_store::{AddressSpace, SystemBacking};

/// A place word's handle: a site's tile or a region, or a country.
#[derive(Clone, Copy, Debug)]
enum Word {
    Wide(AttrW<u32>),
    Country(AttrW<u8>),
}

/// The places of one kind's parties: their rows by slot, what the kind declares its place by, and its word's handle.
#[derive(Debug, phx_macros::Saved)]
pub struct PlaceStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    /// What the kind declares its place by: its site, its region or its country, in that order.
    by: u8,
    #[saved(skip, rebuild = PlaceStore::bind)]
    word: Option<Word>,
}

/// The layout and the word's handle of a place declared by a site (0), a region (1) or a country (2).
fn compiled(by: u8) -> (Layout, Word) {
    let compile = |map| match Layout::compile(map, &[]) {
        Ok(l) => l,
        Err(_) => violation!(clause = "PTY.5", "a place's layout refused"),
    };
    let wide = |mut l: Layout, name: &str| match l.writer::<u32>(name, 0, "K-32") {
        Ok(a) => (l, Word::Wide(a)),
        Err(_) => violation!(clause = "PTY.5", "a place's word refused"),
    };
    match by {
        0 => wide(compile(&SITED), "site"),
        1 => wide(compile(&REGIONED), "region"),
        _ => {
            let mut l = compile(&COUNTRIED);
            match l.writer::<u8>("country", 0, "K-32") {
                Ok(a) => (l, Word::Country(a)),
                Err(_) => violation!(clause = "PTY.5", "a place's word refused"),
            }
        }
    }
}

impl PlaceStore {
    /// A kind's places, room for `capacity` parties; none for a kind placed by its zone, whose own store holds it, or by
    /// its household, whose zone places it.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, place: Place, capacity: u32) -> Option<PlaceStore> {
        let by = match place {
            Place::Site => 0,
            Place::Region => 1,
            Place::Country => 2,
            Place::Zone | Place::Household => return None,
        };
        let (layout, word) = compiled(by);
        Some(PlaceStore { store: KindStore::new(space, kind, &layout, capacity), kind, by, word: Some(word) })
    }

    /// A loaded store's handle, compiled again from what its kind declares.
    fn bind(&mut self) -> u64 {
        self.word = Some(compiled(self.by).1);
        0
    }

    /// What the kind declares its place by.
    #[must_use]
    pub fn place(&self) -> Place {
        match self.by {
            0 => Place::Site,
            1 => Place::Region,
            _ => Place::Country,
        }
    }

    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    fn w(&self) -> Word {
        match self.word {
            Some(w) => w,
            None => violation!(clause = "REP.1", "a place store read before its handle is bound"),
        }
    }

    /// A party the directory began at its place; a place past its word stops the run.
    pub fn begin(&mut self, dir: &Directory<SystemBacking>, r: PartyRef, at: u32) {
        let opening = match self.w() {
            Word::Wide(a) => Opening::of(a, at),
            Word::Country(a) => match u8::try_from(at) {
                Ok(c) => Opening::of(a, c),
                Err(_) => violation!(clause = "PTY.5", "a country past its word", country = at),
            },
        };
        self.store.begin(dir, r, &[opening]);
    }

    /// A party's place: its site's tile, its region or its country, as its kind declares; none for a slot no party was
    /// begun at.
    #[must_use]
    pub fn at(&self, slot: Slot) -> Option<u32> {
        let row = self.store.gather_at(slot, 0)?;
        match self.w() {
            Word::Wide(a) => match row.get(a.read()) {
                Missing::Present(v) => Some(v),
                Missing::Absent => None,
            },
            Word::Country(a) => match row.get(a.read()) {
                Missing::Present(v) => Some(u32::from(v)),
                Missing::Absent => None,
            },
        }
    }
}
