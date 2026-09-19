//! THE REGISTRY: the data every other store's ids point AT.
//!
//! @spec Money A2 · Currency A2 · Currency B1 · Seed B3 · Money D2 · Law 2, Law 4, Law 8, Law 15 ·
//! @spec ARCHITECTURE 4.10

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
    unit_pieces: Vec<f64>,
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
    pub fn stands_on(&mut self, line: InstrumentId, square_km: f64) {
        assert!(square_km > 0.0, "21i: a structure that stands on nothing is not one");
        let at = line.row();
        while self.line_footprint.len() <= at {
            self.line_footprint.push(f64::NAN);
        }
        assert!(
            self.line_footprint[at].is_nan(),
            "Law 4: this line's footprint is declared twice"
        );
        self.line_footprint[at] = square_km;
    }

    /// The footprint, or `Missing` where the line is not a structure.
    pub fn footprint_of(&self, line: InstrumentId) -> Option<f64> {
        match self.line_footprint.get(line.row()) {
            Some(km2) if !km2.is_nan() => Some(*km2),
            _ => None,
        }
    }

    pub fn countries(&self) -> usize {
        self.country_ccy.len()
    }

    pub fn regions(&self) -> usize {
        self.region_country.len()
    }


    /// A unit, and what one of it is divided into.
    pub fn unit(&mut self, pieces_per_whole: f64) -> UnitId {
        assert!(
            pieces_per_whole >= 1.0 && pieces_per_whole.is_finite(),
            "Law 8: a unit divided into {pieces_per_whole} pieces is not a unit anybody counts in"
        );
        let row = self.unit_pieces.len() as u32;
        self.unit_pieces.push(pieces_per_whole);
        UnitId(row)
    }

    pub fn pieces_per_whole(&self, unit: UnitId) -> f64 {
        self.unit_pieces[unit.0 as usize]
    }

    /// Is one of these a thing nobody divides?
    pub fn indivisible(&self, unit: UnitId) -> bool {
        self.pieces_per_whole(unit) == 1.0
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
    pub fn index(&mut self, of: u32, country: CountryId, constituents: &[(InstrumentId, f64)]) -> IndexId {
        assert!(
            country.row() < self.country_ccy.len(),
            "Indices D1: an index is a COUNTRY's, and this one is nobody's"
        );
        assert!(
            !constituents.is_empty(),
            "22 D5.a: an index over nothing has no level, and declaring one is a basket nobody filled"
        );
        for (_, weight) in constituents {
            assert!(*weight > 0.0, "Indices B1: a weight is a COUNT of the line, and {weight} is not one");
        }
        let row = self.index_in.len() as u32;
        self.index_in.push(country.0);
        self.index_of.push(of);
        self.index_at.push(self.constituents.len() as u32);
        self.index_len.push(constituents.len() as u32);
        self.constituents.extend(constituents.iter().map(|(i, w)| (i.0, *w)));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn a_currency_names_the_party_whose_liability_it_is() {
        let mut r = Registry::new();
        let cb = party(1);
        let usd = r.currency(cb);
        assert_eq!(r.issuer_of(usd), cb);
    }

    #[test]
    #[should_panic(expected = "somebody's liability")]
    fn a_currency_nobody_owes_cannot_be_written() {
        Registry::new().currency(PartyId::NONE);
    }

    #[test]
    fn a_region_determines_its_money_by_reading_through_its_country() {
        // A country has the money and a region is a place, so two regions of one country share its
        // money and neither keeps a copy of the fact.
        let mut r = Registry::new();
        let usd = r.currency(party(1));
        let eur = r.currency(party(2));
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
    #[should_panic(expected = "a region is in a country")]
    fn a_region_in_no_country_is_nowhere() {
        let mut r = Registry::new();
        r.region(CountryId::at(0));
    }

    #[test]
    fn a_unit_says_what_one_of_it_is_divided_into() {
        // A dwelling counted on a tonne's grid made a cell member hold four tenths of a roof, which
        // is the occupancy it lives under and not a thing anybody holds.
        let mut r = Registry::new();
        let tonne = r.unit(1_000_000.0);
        let dwelling = r.unit(1.0);
        assert_eq!(r.pieces_per_whole(tonne), 1_000_000.0);
        assert!(r.indivisible(dwelling));
        assert!(!r.indivisible(tonne));
    }

    #[test]
    #[should_panic(expected = "is not a unit anybody counts in")]
    fn a_unit_divided_into_less_than_one_piece_is_not_a_unit() {
        Registry::new().unit(0.5);
    }

    #[test]
    fn a_kind_answers_through_its_profile_and_a_kind_with_none_says_so() {
        // The integer stays the id; what goes behind it is what varies.
        let mut r = Registry::new();
        r.profile_for(7, KindProfile { issues_money: true, banks: Banks::Nowhere, issues_paper: false });
        r.profile_for(6, KindProfile { issues_money: false, banks: Banks::AtTheCentralBank, issues_paper: true });

        assert_eq!(r.profile(7).map(|p| p.banks), Some(Banks::Nowhere));
        assert!(r.profile(7).unwrap().issues_money);
        assert_eq!(r.profile(6).map(|p| p.banks), Some(Banks::AtTheCentralBank));
        // Missing is missing: no default profile is invented for a kind nobody declared.
        assert!(r.profile(3).is_none());
    }

    #[test]
    fn an_index_is_a_countrys_and_it_is_built_from_named_lines() {
        // Nothing declared an index anywhere, so no basket had a level to read.
        let mut r = Registry::new();
        let usd = r.currency(party(1));
        let eur = r.currency(party(2));
        let us = r.country(usd);
        let de = r.country(eur);

        let line = |n: u32| InstrumentId::at(n);
        let us_equity = r.index(0, us, &[(line(10), 100.0), (line(11), 250.0)]);
        let de_equity = r.index(0, de, &[(line(20), 90.0)]);
        let us_credit = r.index(1, us, &[(line(12), 40.0)]);

        assert_eq!(r.index_country(us_equity), us);
        assert_eq!(r.index_subject(us_credit), 1);
        assert_eq!(r.index_constituents(us_equity), &[(10, 100.0), (11, 250.0)]);
        // Four regions, four equity indices: the read that says so is a read over the declarations.
        assert_eq!(r.indices_in(us).len(), 2);
        assert_eq!(r.indices_in(de), vec![de_equity]);
        assert_eq!(r.indices(), 3);
    }

    #[test]
    #[should_panic(expected = "an index over nothing")]
    fn an_index_with_an_empty_basket_is_not_declared() {
        // The empty-basket refusal, exercised at the declaration rather than at the read.
        let mut r = Registry::new();
        let usd = r.currency(party(1));
        let us = r.country(usd);
        r.index(0, us, &[]);
    }

    #[test]
    #[should_panic(expected = "a weight is a COUNT of the line")]
    fn a_constituent_weight_is_a_count_and_never_a_share() {
        let mut r = Registry::new();
        let usd = r.currency(party(1));
        let us = r.country(usd);
        r.index(0, us, &[(InstrumentId::at(1), 0.0)]);
    }

    #[test]
    #[should_panic(expected = "is given two profiles")]
    fn a_kind_has_one_profile() {
        let mut r = Registry::new();
        let p = KindProfile { issues_money: false, banks: Banks::AtACommercialBank, issues_paper: false };
        r.profile_for(1, p);
        r.profile_for(1, p);
    }
}
