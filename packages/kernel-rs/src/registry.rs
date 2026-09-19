//! THE REGISTRY: the data every other store's ids point AT.
//!
//! @spec Money A2 · Currency A2 · Currency B1 · Seed B3 · Money D2 · Law 2, Law 4, Law 8, Law 15 ·
//! @spec ARCHITECTURE 4.10

use std::num::NonZeroU32;
use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};

/// A country: one currency, one central bank, one treasury, one sovereign line, one FX pair and one
/// equity index.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CountryId(pub u32);

impl CountryId {
    pub const NONE: CountryId = CountryId(u32::MAX);

    pub fn at(n: u32) -> CountryId {
        CountryId(n)
    }

    #[inline]
    pub fn row(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub fn some(self) -> bool {
        self != CountryId::NONE
    }
}

/// What an index is an index OF.
pub mod tracks {
    pub const EQUITY: u32 = 0;
    pub const CREDIT: u32 = 1;
    /// Consumer prices and producer prices are TWO indices, not one wearing both names — they are
    /// built from different constituents and a cost shock moves them differently.
    pub const CONSUMER_PRICES: u32 = 2;
    pub const PRODUCER_PRICES: u32 = 3;
}

/// An index, and the country whose it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IndexId(pub u32);

impl IndexId {
    pub fn at(n: u32) -> IndexId {
        IndexId(n)
    }

    #[inline]
    pub fn row(self) -> usize {
        self.0 as usize
    }
}

/// Where a party of this kind keeps its money.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Banks {
    /// A central bank: it issues the money everybody else settles in, so it banks nowhere.
    Nowhere,
    /// A treasury banks at its central bank, which is why *no central-bank overdraft for the
    /// treasury* is a rule about a real account.
    AtTheCentralBank,
    /// Everybody else holds a deposit issued by a commercial bank.
    AtACommercialBank,
}

/// WHAT ONE OF A LINE STANDS ON, in square km. A structure that stands on nothing is not one, so
/// there is no way to write a footprint of zero.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Footprint(f64);

impl Footprint {
    pub fn new(square_km: f64) -> Option<Footprint> {
        match square_km > 0.0 && square_km.is_finite() {
            true => Some(Footprint(square_km)),
            false => None,
        }
    }

    #[inline]
    pub fn km2(self) -> f64 {
        self.0
    }
}

/// What varies by party kind, behind a dispatch the kernel reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KindProfile {
    /// Whether a party of this kind ISSUES the money others hold of it.
    pub issues_money: bool,
    pub banks: Banks,
    /// Whether a party of this kind funds a shortfall by BRINGING PAPER.
    pub issues_paper: bool,
}

/// ARCHITECTURE 4.10: all data lives here.
#[derive(Default)]
pub struct Registry {
    /// The party whose liability each money is.
    ccy_issuer: Vec<u32>,
    /// One currency per country.
    country_ccy: Vec<u32>,
    region_country: Vec<u32>,
    /// How many pieces one whole of this unit is divided into.
    unit_pieces: Vec<NonZeroU32>,
    /// By party-kind id.
    profiles: Vec<Option<KindProfile>>,
    /// What one unit of each line STANDS ON, in square km.
    line_footprint: Vec<f64>,
    /// Which indices exist, whose country each is, what it is an index OF, and the lines it is built
    /// from with the COUNT of each (B1: a weight is a count of the line, never a share).
    index_in: Vec<u32>,
    index_of: Vec<u32>,
    index_at: Vec<u32>,
    index_len: Vec<u32>,
    constituents: Vec<(u32, f64)>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }


    /// A money and the party whose liability it is.
    pub fn currency(&mut self, issuer: PartyId) -> CurrencyCode {
        assert!(issuer.some(), "Money A2: a currency is somebody's liability, and this one names nobody");
        let row = self.ccy_issuer.len() as u32;
        self.ccy_issuer.push(issuer.0);
        CurrencyCode(row)
    }

    pub fn issuer_of(&self, ccy: CurrencyCode) -> PartyId {
        PartyId(self.ccy_issuer[ccy.0 as usize])
    }

    pub fn currencies(&self) -> usize {
        self.ccy_issuer.len()
    }


    /// A country has the money.
    pub fn country(&mut self, ccy: CurrencyCode) -> CountryId {
        assert!(
            (ccy.0 as usize) < self.ccy_issuer.len(),
            "Currency A2: a country's money must be a currency this registry knows"
        );
        let row = self.country_ccy.len() as u32;
        self.country_ccy.push(ccy.0);
        CountryId(row)
    }

    /// A region is a place, and it is in exactly one country.
    pub fn region(&mut self, country: CountryId) -> RegionId {
        assert!(
            country.row() < self.country_ccy.len(),
            "Seed B3: a region is in a country, and this one is in nothing"
        );
        let row = self.region_country.len() as u32;
        self.region_country.push(country.0);
        RegionId(row)
    }

    pub fn country_of(&self, region: RegionId) -> CountryId {
        CountryId(self.region_country[region.0 as usize])
    }

    /// The region determines its money — read THROUGH the country, so the fact has one writer.
    pub fn currency_of(&self, region: RegionId) -> CurrencyCode {
        CurrencyCode(self.country_ccy[self.country_of(region).row()])
    }

    /// WHAT ONE UNIT OF THIS LINE STANDS ON.
    pub fn stands_on(&mut self, line: InstrumentId, ground: Footprint) {
        let at = line.row();
        while self.line_footprint.len() <= at {
            self.line_footprint.push(f64::NAN);
        }
        assert!(
            self.line_footprint[at].is_nan(),
            "Law 4: this line's footprint is declared twice"
        );
        self.line_footprint[at] = ground.0;
    }

    /// The footprint, or `Missing` where the line is not a structure.
    pub fn footprint_of(&self, line: InstrumentId) -> Option<Footprint> {
        match self.line_footprint.get(line.row()) {
            Some(km2) if !km2.is_nan() => Some(Footprint(*km2)),
            _ => None,
        }
    }

    pub fn countries(&self) -> usize {
        self.country_ccy.len()
    }

    pub fn regions(&self) -> usize {
        self.region_country.len()
    }


    /// A unit, and what one of it is divided into. A count of pieces, so a unit divided into half
    /// a piece cannot be written.
    pub fn unit(&mut self, pieces_per_whole: NonZeroU32) -> UnitId {
        let row = self.unit_pieces.len() as u32;
        self.unit_pieces.push(pieces_per_whole);
        UnitId(row)
    }

    pub fn pieces_per_whole(&self, unit: UnitId) -> f64 {
        f64::from(self.unit_pieces[unit.0 as usize].get())
    }

    /// Is one of these a thing nobody divides?
    pub fn indivisible(&self, unit: UnitId) -> bool {
        self.unit_pieces[unit.0 as usize].get() == 1
    }

    pub fn units(&self) -> usize {
        self.unit_pieces.len()
    }


    /// The behaviour that varies by kind, declared once at assembly.
    pub fn profile_for(&mut self, kind: u32, p: KindProfile) {
        let at = kind as usize;
        while self.profiles.len() <= at {
            self.profiles.push(None);
        }
        assert!(self.profiles[at].is_none(), "Law 4: kind {kind} is given two profiles");
        self.profiles[at] = Some(p);
    }

    /// `Missing` is missing: a kind with no profile has none, and the caller decides whether that is
    /// an answer or a fault.
    pub fn profile(&self, kind: u32) -> Option<KindProfile> {
        self.profiles.get(kind as usize).copied().flatten()
    }


    /// An index is a country's, it is ONE system, and it is built from named lines.
    pub fn index(&mut self, of: u32, country: CountryId, constituents: &[(InstrumentId, NonZeroU32)]) -> IndexId {
        assert!(
            country.row() < self.country_ccy.len(),
            "Indices D1: an index is a COUNTRY's, and this one is nobody's"
        );
        assert!(
            !constituents.is_empty(),
            "22 D5.a: an index over nothing has no level, and declaring one is a basket nobody filled"
        );
        let row = self.index_in.len() as u32;
        self.index_in.push(country.0);
        self.index_of.push(of);
        self.index_at.push(self.constituents.len() as u32);
        self.index_len.push(constituents.len() as u32);
        self.constituents.extend(constituents.iter().map(|(i, w)| (i.0, f64::from(w.get()))));
        IndexId(row)
    }

    pub fn index_country(&self, i: IndexId) -> CountryId {
        CountryId(self.index_in[i.row()])
    }

    pub fn index_subject(&self, i: IndexId) -> u32 {
        self.index_of[i.row()]
    }

    /// What it is built from.
    pub fn index_constituents(&self, i: IndexId) -> &[(u32, f64)] {
        let at = self.index_at[i.row()] as usize;
        let len = self.index_len[i.row()] as usize;
        &self.constituents[at..at + len]
    }

    /// Every index of one country — four regions, four equity indices, and the read that says
    /// whether that is true is a read over this rather than a count somebody keeps.
    pub fn indices_in(&self, country: CountryId) -> Vec<IndexId> {
        (0..self.index_in.len() as u32)
            .map(IndexId)
            .filter(|i| self.index_in[i.row()] == country.0)
            .collect()
    }

    pub fn indices(&self) -> usize {
        self.index_in.len()
    }
}

// Four refusals the TYPE now makes unconstructible: a unit divided into less than a whole piece
// and an index constituent weighted at nothing (both `NonZeroU32` — a divisor and a weight are
// COUNTS), a structure that stands on nothing (`Footprint`), and a currency, country or region
// built on an id this registry never issued (an id comes back from the call that made it).
//
// What is left runtime is what relates an argument to what the registry already holds: a currency
// naming nobody, a basket with nothing in it, and a kind given two profiles. Each panics at the
// site, and what the registry answers — a region's money read THROUGH its country, a kind with no
// profile answering `None` — is a read the world takes every period.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_is_a_count_of_pieces_and_one_is_indivisible() {
        let mut r = Registry::new();
        let tonne = r.unit(NonZeroU32::new(1_000_000).unwrap());
        let dwelling = r.unit(NonZeroU32::new(1).unwrap());
        assert_eq!(r.pieces_per_whole(tonne), 1_000_000.0);
        assert!(r.indivisible(dwelling));
        assert!(!r.indivisible(tonne));
    }

    #[test]
    fn a_region_determines_its_money_by_reading_through_its_country() {
        // A country has the money and a region is a place, so two regions of one country share its
        // money and neither keeps a copy of the fact.
        let mut r = Registry::new();
        let usd = r.currency(PartyId::at(1));
        let eur = r.currency(PartyId::at(2));
        let us = r.country(usd);
        let de = r.country(eur);
        let east = r.region(us);
        let west = r.region(us);
        let south = r.region(de);

        assert_eq!(r.currency_of(east), usd);
        assert_eq!(r.currency_of(west), usd);
        assert_eq!(r.currency_of(south), eur);
        assert_eq!(r.country_of(west), us);
    }

    #[test]
    fn a_kind_answers_through_its_profile_and_a_kind_with_none_says_so() {
        let mut r = Registry::new();
        r.profile_for(7, KindProfile { issues_money: true, banks: Banks::Nowhere, issues_paper: false });
        assert_eq!(r.profile(7).map(|p| p.banks), Some(Banks::Nowhere));
        // Missing is missing: no default profile is invented for a kind nobody declared.
        assert!(r.profile(3).is_none());
    }
}
