//! `-F catalogue`: the finished world's kind catalogue — its party kinds, families, reasons, lines, markets, hazards,
//! decision kinds, products, ways and capital kinds at the counts the finished world declares — compiled from its
//! declarations once a day, and every family's reason and side kinds read by handle.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::catalogue::{
    CapitalDecl, Catalogue, DecisionEntry, Declared, FamilyCode, FamilyDecl, FamilyKind, FamilyStatus, FormDecl,
    HazardEntry, KindEntry, LineDecl, MarketDecl, MarketForm, MeetingDays, ProductDecl, ReasonDecl, WayDecl, compile,
};
use phx_core::kinds::{Feature, Owners, Place};
use phx_num::Missing;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::wide;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the catalogue is measured under.
pub const BASE: &str = "catalogue";

/// The finished world's counts of each declared thing.
const FORMS: usize = 8;
const KINDS: usize = 26;
const FAMILIES: usize = 80;
const LINES: usize = 300;
const REASONS: usize = 400;
const MARKETS: usize = 60;
const HAZARDS: usize = 40;
const DECISIONS: usize = 150;
const PRODUCTS: usize = 250;
const WAYS: usize = 200;
const CAPITALS: usize = 30;
/// The register's primitives, which markets' tick sizes, points and methods are read from.
const PRIMS: usize = 5_000;

/// Each form's offices, a family's side kinds, a reason's lines, a market's participants, a way's inputs and a capital
/// kind's classes.
const OFFICES: usize = 6;
const SIDES: usize = 2;
const REASON_LINES: usize = 3;
const PARTICIPANTS: usize = 4;
const INPUTS: usize = 5;
const CLASSES: usize = 4;
/// Each product's grades, a gate for one reason in `GATED`, of `GATES` gates, and payment orders among `ORDERS`.
const GRADES: u8 = 3;
const GATED: usize = 5;
const GATES: usize = 20;
const ORDERS: usize = 16;
/// One family in `JOBS` is a job; each declares room for a million rows.
const JOBS: usize = 4;
const FAMILY_ROWS: u32 = 1 << 20;
/// Business days a market settles after.
const SETTLES_AFTER: u8 = 2;
/// Reads of every family's reason and sides a day.
const READS: u32 = 1_000;

const MIB: f64 = 1_048_576.0;

const MARKET_FORMS: [MarketForm; 9] = [
    MarketForm::PostedPrice,
    MarketForm::Call,
    MarketForm::NetworkCall,
    MarketForm::Book,
    MarketForm::Dealer,
    MarketForm::Bilateral,
    MarketForm::Administered,
    MarketForm::Search,
    MarketForm::Queue,
];

/// The declared names, owned, each table's in declaration order.
#[derive(Debug, Default)]
struct Source {
    forms: Vec<String>,
    offices: Vec<String>,
    kinds: Vec<String>,
    families: Vec<String>,
    lines: Vec<String>,
    reasons: Vec<String>,
    gates: Vec<String>,
    markets: Vec<String>,
    hazards: Vec<String>,
    decisions: Vec<String>,
    products: Vec<String>,
    ways: Vec<String>,
    capitals: Vec<String>,
    prims: Vec<String>,
    codes: Vec<FamilyCode>,
}

fn named(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}{i}")).collect()
}

/// The `k`th of `n` names after `i`, wrapping.
fn nth(names: &[String], i: usize, k: usize) -> &str {
    names.get((i + k) % names.len()).map_or("", String::as_str)
}

/// `n` names from the `i`th on, wrapping.
fn pick(names: &[String], i: usize, n: usize) -> Vec<&str> {
    (0..n).map(|k| nth(names, i, k)).collect()
}

/// The finished world's catalogue, compiled each day from its declarations.
#[derive(Debug, Default)]
pub struct CatalogueBase {
    source: Source,
    compiled: Catalogue,
    folded: u64,
}

/// The lists the declarations borrow: each form's offices, each family's sides, each reason's lines, each market's
/// participants, each way's inputs and each capital kind's classes.
struct Lists<'a> {
    offices: Vec<&'a str>,
    sides: Vec<Vec<&'a str>>,
    lines: Vec<Vec<&'a str>>,
    participants: Vec<Vec<&'a str>>,
    inputs: Vec<Vec<(&'a str, u8)>>,
    classes: Vec<Vec<&'a str>>,
    prims: Vec<&'a str>,
}

impl<'a> Lists<'a> {
    fn of(s: &'a Source) -> Lists<'a> {
        Lists {
            offices: pick(&s.offices, 0, OFFICES),
            sides: (0..FAMILIES).map(|i| pick(&s.kinds, i, SIDES)).collect(),
            lines: (0..REASONS).map(|i| pick(&s.lines, i, REASON_LINES)).collect(),
            participants: (0..MARKETS).map(|i| pick(&s.kinds, i, PARTICIPANTS)).collect(),
            inputs: (0..WAYS)
                .map(|i| pick(&s.products, i, INPUTS).into_iter().map(|p| (p, GRADES - 1)).collect())
                .collect(),
            classes: (0..CAPITALS).map(|i| pick(&s.products, i, CLASSES)).collect(),
            prims: s.prims.iter().map(String::as_str).collect(),
        }
    }
}

/// The classes of holding every form may hold, and the one the families' contracts are held as.
const CLASSES_HELD: [&str; 4] = ["money", "loans", "securities", "shares"];
const LOANS: &str = "loans";
const FEATURES: [Feature; 3] = [Feature::SeparateParty, Feature::LimitedLiability, Feature::HasOwners];
const ENDINGS: [&str; 1] = ["insolvency"];

/// Every table's declarations but the markets'.
struct Tables<'a> {
    forms: Vec<FormDecl<'a>>,
    kinds: Vec<KindEntry<'a>>,
    families: Vec<FamilyDecl<'a>>,
    lines: Vec<LineDecl<'a>>,
    reasons: Vec<ReasonDecl<'a>>,
    hazards: Vec<HazardEntry<'a>>,
    decisions: Vec<DecisionEntry<'a>>,
    products: Vec<ProductDecl<'a>>,
    ways: Vec<WayDecl<'a>>,
    capitals: Vec<CapitalDecl<'a>>,
}

impl<'a> Tables<'a> {
    fn of(s: &'a Source, l: &'a Lists<'a>) -> Tables<'a> {
        let (owners, offices) = (Owners::Shareholders, l.offices.as_slice());
        Tables {
            forms: s
                .forms
                .iter()
                .map(|name| FormDecl {
                    system: "FRM",
                    name,
                    may_hold: &CLASSES_HELD,
                    features: &FEATURES,
                    endings: &ENDINGS,
                    owners,
                    offices,
                })
                .collect(),
            kinds: s
                .kinds
                .iter()
                .enumerate()
                .map(|(i, name)| KindEntry {
                    system: "PTY",
                    name,
                    form: nth(&s.forms, i, 0),
                    place: Place::Sited,
                    store: "firms",
                })
                .collect(),
            families: s
                .families
                .iter()
                .zip(&l.sides)
                .enumerate()
                .map(|(i, (name, kinds))| FamilyDecl {
                    system: "CON",
                    name,
                    reason: nth(&s.reasons, i, 0),
                    kinds,
                    jobs: i % JOBS == 0,
                    slots: FAMILY_ROWS,
                    class: Missing::Present(LOANS),
                    holders: kinds.get(..1).unwrap_or_default(),
                })
                .collect(),
            lines: s.lines.iter().map(|name| LineDecl { system: "ACC", name }).collect(),
            reasons: s
                .reasons
                .iter()
                .zip(&l.lines)
                .enumerate()
                .map(|(i, (name, lines))| ReasonDecl {
                    system: "SET",
                    name,
                    lines,
                    payment_order: u8::try_from(i % ORDERS).map_or(Missing::Absent, Missing::Present),
                    gate: if i % GATED == 0 { Missing::Present(nth(&s.gates, i, 0)) } else { Missing::Absent },
                })
                .collect(),
            hazards: s
                .hazards
                .iter()
                .enumerate()
                .map(|(i, name)| HazardEntry { system: "HAZ", name, acts_on: nth(&s.kinds, i, 0) })
                .collect(),
            decisions: s
                .decisions
                .iter()
                .enumerate()
                .map(|(i, name)| DecisionEntry {
                    system: "MND",
                    name,
                    taker: nth(&s.kinds, i, 0),
                    office: nth(&s.offices, i, 0),
                })
                .collect(),
            products: s.products.iter().map(|name| ProductDecl { system: "GDS", name, grades: GRADES }).collect(),
            ways: s.ways.iter().zip(&l.inputs).map(|(name, inputs)| WayDecl { system: "TEC", name, inputs }).collect(),
            capitals: s
                .capitals
                .iter()
                .zip(&l.classes)
                .map(|(name, classes)| CapitalDecl { system: "CAP", name, classes })
                .collect(),
        }
    }
}

/// The markets' declarations, each reading three of the register's primitives.
fn markets<'a>(s: &'a Source, l: &'a Lists<'a>) -> Result<Vec<MarketDecl<'a>>, FinError> {
    let mut out = Vec::with_capacity(MARKETS);
    for (i, (name, participants)) in s.markets.iter().zip(&l.participants).enumerate() {
        let form = MARKET_FORMS.get(i % MARKET_FORMS.len()).ok_or_else(|| FinError("no market form".to_owned()))?;
        out.push(MarketDecl {
            system: "MKT",
            name,
            form: *form,
            operator: nth(&s.kinds, i, 0),
            days: MeetingDays::BusinessDays,
            settles_after: SETTLES_AFTER,
            participants,
            trades: nth(&s.families, i, 0),
            tick: nth(&s.prims, 3 * i, 0),
            points: nth(&s.prims, 3 * i, 1),
            valuer_method: Missing::Present(nth(&s.prims, 3 * i, 2)),
        });
    }
    Ok(out)
}

impl CatalogueBase {
    /// The declarations compiled, the compile measured.
    fn compile(&mut self, m: &mut Measures<'_>) -> Result<(), FinError> {
        let lists = Lists::of(&self.source);
        let t = Tables::of(&self.source, &lists);
        let markets = markets(&self.source, &lists)?;
        let declared = Declared {
            forms: &t.forms,
            kinds: &t.kinds,
            families: &t.families,
            lines: &t.lines,
            reasons: &t.reasons,
            markets: &markets,
            hazards: &t.hazards,
            decisions: &t.decisions,
            products: &t.products,
            ways: &t.ways,
            capitals: &t.capitals,
            prims: &lists.prims,
            codes: &self.source.codes,
        };
        let compiled = m.read(BASE, "compile", 1, || compile(&declared));
        self.compiled = compiled.map_err(|refused| FinError(refused.join("; ")))?;
        Ok(())
    }
}

impl FinBase for CatalogueBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The finished world's names, each table's at its count.
    fn fill(&mut self, _design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        self.source = Source {
            forms: named("form", FORMS),
            offices: named("office", OFFICES),
            kinds: named("kind", KINDS),
            families: named("family", FAMILIES),
            lines: named("line", LINES),
            reasons: named("reason", REASONS),
            gates: named("gate", GATES),
            markets: named("market", MARKETS),
            hazards: named("hazard", HAZARDS),
            decisions: named("decision", DECISIONS),
            products: named("product", PRODUCTS),
            ways: named("way", WAYS),
            capitals: named("capital", CAPITALS),
            prims: named("prim", PRIMS),
            codes: Vec::new(),
        };
        // Each family at its own fixed code, as the family codes give one.
        for (code, name) in self.source.families.iter().enumerate() {
            let code = u8::try_from(code).map_err(|e| FinError(e.to_string()))?;
            let (kind, status, step) = (FamilyKind::Contract, FamilyStatus::Declared, String::new());
            self.source.codes.push(FamilyCode { code, name: name.clone(), kind, step, status });
        }
        let rows = KINDS + FAMILIES + LINES + REASONS + MARKETS + HAZARDS + DECISIONS + PRODUCTS + WAYS + CAPITALS;
        Ok(Filled { rows: wide(rows) })
    }

    /// The declarations compiled, then every family's reason, payment order and side kinds read by handle.
    fn day(&mut self, _day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        self.compile(m)?;
        let c = &self.compiled;
        let handles: Vec<_> = c.families().collect();
        let reads = u64::from(READS) * wide(handles.len());
        self.folded ^= m.read(BASE, "read", reads, || {
            let mut fold = 0_u64;
            for _ in 0..READS {
                for h in &handles {
                    let family = c.family(*h);
                    let order = c.reason(family.reason).payment_order;
                    fold ^= u64::from(order) ^ wide(c.family_kinds(black_box(*h)).len());
                }
            }
            black_box(fold)
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: wide(self.compiled.bytes()), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let bytes = self.compiled.bytes().to_string().parse::<f64>().ok();
        bytes.map(|b| vec![("mb", b / MIB)]).unwrap_or_default()
    }
}
