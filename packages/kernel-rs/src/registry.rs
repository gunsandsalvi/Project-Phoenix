//! THE REGISTRY: the data every other store's ids point AT.
//!
//! @spec Money A2 · Currency A2 · Currency B1 · Seed B3 · Money D2 · Law 2, Law 4, Law 8, Law 15 ·
//! @spec ARCHITECTURE 4.10

use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use std::num::NonZeroU32;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreditQuality {
    InvestmentGrade,
    HighYield,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Capitalisation {
    All,
    Small,
    Large,
}

/// What an index measures. Credit contracts remain distinct because a bond, a CDS and a tradable
/// term loan are three different markets even when they refer to the same borrower.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndexSubject {
    Equity(Capitalisation),
    FixedBond(CreditQuality),
    Cds(CreditQuality),
    TradableTermLoan(CreditQuality),
    ConsumerPrices,
    ProducerPrices,
}

impl IndexSubject {
    pub fn code(self) -> u32 {
        match self {
            IndexSubject::Equity(Capitalisation::All) => 0,
            IndexSubject::Equity(Capitalisation::Small) => 1,
            IndexSubject::Equity(Capitalisation::Large) => 2,
            IndexSubject::FixedBond(CreditQuality::InvestmentGrade) => 3,
            IndexSubject::FixedBond(CreditQuality::HighYield) => 4,
            IndexSubject::Cds(CreditQuality::InvestmentGrade) => 5,
            IndexSubject::Cds(CreditQuality::HighYield) => 6,
            IndexSubject::TradableTermLoan(CreditQuality::HighYield) => 7,
            IndexSubject::TradableTermLoan(CreditQuality::InvestmentGrade) => 8,
            IndexSubject::ConsumerPrices => 9,
            IndexSubject::ProducerPrices => 10,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndexScope {
    Currency(CurrencyCode),
    Global,
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

/// Which accumulated state can end a party's legal life. This is a declared capability of a kind,
/// not a switch hidden in the mortality mechanism.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailureMode {
    Never,
    Household,
    Operating,
    BalanceSheet,
    Bank,
    Sovereign,
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

/// ONE WAY OF MAKING A LINE: fixed input quantities per unit of output, plus labour, plus capital
/// services, what it yields, its smallest run and how long it takes. It is TECHNOLOGY about the
/// good and true for every maker of it, which is why it is declared once here rather than handed to
/// a mechanism. What it MAKES is the row it is declared under, so a way of making something else
/// cannot be written down.
#[derive(Clone, Debug, PartialEq)]
pub struct Way {
    /// Quantities, in the input's own physical unit.
    pub per_unit: Vec<(InstrumentId, f64)>,
    pub labour_per_unit: f64,
    pub capital_services_per_unit: f64,
    /// Not everything started is finished.
    pub yields: f64,
    /// The smallest run of the line — a furnace charge, a print run, a shift.
    pub batch: f64,
    pub periods_to_make: u32,
}

impl Way {
    /// Whether this is a way of making anything at all: it must yield some of what it starts and no
    /// more, run in a batch of something, and take time. The writer refuses one that is not.
    pub fn runnable(&self) -> bool {
        self.yields > 0.0 && self.yields <= 1.0 && self.batch > 0.0 && self.periods_to_make > 0
    }
}

/// What a kind of plant IS, and the presence of a useful life is what makes a good a capital good.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plant {
    pub life: u32,
    /// What it costs to keep, every week, whether or not it runs.
    pub upkeep_per_period: f64,
    /// Capacity is a function of the stock.
    pub capacity_per_period: f64,
}

/// What varies by party kind, behind a dispatch the kernel reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KindProfile {
    /// Whether a party of this kind ISSUES the money others hold of it.
    pub issues_money: bool,
    pub banks: Banks,
    /// Whether a party of this kind funds a shortfall by BRINGING PAPER.
    pub issues_paper: bool,
    pub failure: FailureMode,
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
    /// How each line is made and with what plant, by the line's own row; what each plant IS, by
    /// its own row; and the lines that are made at all, so a reader does not walk every instrument.
    line_ways: Vec<Vec<Way>>,
    line_plant: Vec<Option<InstrumentId>>,
    plant_is: Vec<Option<Plant>>,
    made: Vec<InstrumentId>,
    /// Which indices exist, their currency/global scope and market family, and the lines they are
    /// built from with the COUNT of each (B1: a weight is a count, never a share).
    index_scope: Vec<IndexScope>,
    index_of: Vec<IndexSubject>,
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
        assert!(
            issuer.some(),
            "Money A2: a currency is somebody's liability, and this one names nobody"
        );
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

    pub fn currency_of_country(&self, country: CountryId) -> CurrencyCode {
        CurrencyCode(self.country_ccy[country.row()])
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

    /// HOW A LINE IS MADE, and with what. The ways and the plant are declared together because a
    /// way that draws capital services with no plant to draw them from is not a way.
    pub fn made_by(&mut self, line: InstrumentId, ways: Vec<Way>, with: InstrumentId) {
        assert!(
            !ways.is_empty(),
            "37 A2: a line with no way of making it is not a line"
        );
        for w in &ways {
            assert!(
                w.runnable(),
                "37 B3, B4, B5.b: a way that yields {} of what it starts, in runs of {}, over {} weeks is not a way of making it",
                w.yields,
                w.batch,
                w.periods_to_make
            );
        }
        let at = line.row();
        while self.line_ways.len() <= at {
            self.line_ways.push(Vec::new());
            self.line_plant.push(None);
        }
        assert!(
            self.line_ways[at].is_empty(),
            "Law 4: this line's ways are declared twice"
        );
        self.line_ways[at] = ways;
        self.line_plant[at] = Some(with);
        self.made.push(line);
    }

    /// WHAT A PLANT IS, declared once for the line and true for every holder of it.
    pub fn is_plant(&mut self, plant: InstrumentId, what: Plant) {
        assert!(
            what.life > 0,
            "33 A3: a good with no useful life is not a capital good"
        );
        let at = plant.row();
        while self.plant_is.len() <= at {
            self.plant_is.push(None);
        }
        assert!(
            self.plant_is[at].is_none(),
            "Law 4: this plant's technology is declared twice"
        );
        self.plant_is[at] = Some(what);
    }

    /// Every line this world knows how to make.
    pub fn made(&self) -> &[InstrumentId] {
        &self.made
    }

    /// The ways of making one, which is empty for a line nobody makes.
    pub fn ways_of(&self, line: InstrumentId) -> &[Way] {
        match self.line_ways.get(line.row()) {
            Some(ways) => ways,
            None => &[],
        }
    }

    /// The plant it is made with, or `Missing` where nothing says.
    pub fn made_with(&self, line: InstrumentId) -> Option<InstrumentId> {
        self.line_plant.get(line.row()).copied().flatten()
    }

    /// What a plant is, or `Missing` where the line is not one.
    pub fn plant_of(&self, plant: InstrumentId) -> Option<Plant> {
        self.plant_is.get(plant.row()).copied().flatten()
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
        assert!(
            self.profiles[at].is_none(),
            "Law 4: kind {kind} is given two profiles"
        );
        self.profiles[at] = Some(p);
    }

    /// `Missing` is missing: a kind with no profile has none, and the caller decides whether that is
    /// an answer or a fault.
    pub fn profile(&self, kind: u32) -> Option<KindProfile> {
        self.profiles.get(kind as usize).copied().flatten()
    }

    /// An index has one explicit scope and market family and is built from named lines.
    pub fn index(
        &mut self,
        of: IndexSubject,
        scope: IndexScope,
        constituents: &[(InstrumentId, NonZeroU32)],
    ) -> IndexId {
        if let IndexScope::Currency(currency) = scope {
            assert!(
                currency.row() < self.ccy_issuer.len(),
                "Indices D1: an index names an undeclared currency"
            );
        }
        assert!(
            !constituents.is_empty(),
            "22 D5.a: an index over nothing has no level, and declaring one is a basket nobody filled"
        );
        let row = self.index_scope.len() as u32;
        self.index_scope.push(scope);
        self.index_of.push(of);
        self.index_at.push(self.constituents.len() as u32);
        self.index_len.push(constituents.len() as u32);
        self.constituents
            .extend(constituents.iter().map(|(i, w)| (i.0, f64::from(w.get()))));
        IndexId(row)
    }

    pub fn index_scope(&self, i: IndexId) -> IndexScope {
        self.index_scope[i.row()]
    }

    pub fn index_subject(&self, i: IndexId) -> IndexSubject {
        self.index_of[i.row()]
    }

    /// What it is built from.
    pub fn index_constituents(&self, i: IndexId) -> &[(u32, f64)] {
        let at = self.index_at[i.row()] as usize;
        let len = self.index_len[i.row()] as usize;
        &self.constituents[at..at + len]
    }

    /// Every currency-scoped index for a country; global definitions are intentionally separate.
    pub fn indices_in(&self, country: CountryId) -> Vec<IndexId> {
        let currency = self.currency_of_country(country);
        (0..self.index_scope.len() as u32)
            .map(IndexId)
            .filter(|i| self.index_scope[i.row()] == IndexScope::Currency(currency))
            .collect()
    }

    pub fn indices(&self) -> usize {
        self.index_scope.len()
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
// profile answering `None` — is a read the world takes every week.

// A REGION'S MONEY HAS ONE WRITER, and it is the shape of this store rather than a test: there is
// no region-to-currency column. `region_country` and `country_ccy` are the only two, so
// `currency_of` has to read through the country and two regions of one country cannot disagree.
//
// The rest is the type. A unit's divisor and an index weight are `NonZeroU32`, a footprint is a
// `Footprint`, a kind with no profile answers `None`, and an id comes back from the call that made
// it. What relates an argument to what the registry already holds — a currency naming nobody, a
// basket with nothing in it, a line declared a structure twice, a kind given two profiles — panics
// where it is written.
