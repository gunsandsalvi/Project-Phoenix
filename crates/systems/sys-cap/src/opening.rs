//! The opening's plant: each country's net stock of each kind, its GDP times its group's stock per unit of GDP,
//! apportioned over every firm, large and small, by its employees times its industry's plant of that kind per hour
//! worked, and held by condition class as a stock grown at the country's growth rate would be.

use if_firm::known::Industry;
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::register::values::Table1;
use phx_core::{
    Contribution, DECLARATIONS, FactDef, Opening, OpeningCountry, OpeningPhase, PHYSICAL_STOCK, Prim, Register,
    StreamDef, apportion_in_units, opening_subject,
};
use phx_id::{InstrumentId, PartyId};
use phx_ledger::algebra::Terms;
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, LegRec, ReasonDecl, ReasonId};
use phx_ledger::instrument::{InstrumentFamily, NewInstrument};
use phx_ledger::opening::{currency, derived, hold, key};
use phx_macros::clause;
use phx_num::{Missing, UnitId, violation};
use phx_pop::population::Population;
use phx_rand::float::{floor_to_i64, floor_to_u64, from_i64, from_u64};
use phx_store::SystemBacking;

use crate::consts::{PERCENT, WEIGHT_PARTS};
use crate::kinds::{Kinds, Prims};
use crate::{OpeningStream, rules};

/// The large firms and their employees, the small firms' agents with the persons their twins employ, and each
/// agent's twins, as the firms' opening draws them.
const FIRMS: &str = "FRM.firms";
const SMALL_FIRMS: &str = "FRM.small_firms";
const SMALL_COUNTS: &str = "FRM.small_counts";
const SMALL_FIRM: &str = "small_firm";
/// Each large firm with the structures it holds at the opening, whose owners stand in for the landlords.
pub const STRUCTURES_HELD: &str = "CAP.structures";
/// The technology the firms' plant per hour worked is read from.
const PRODUCTS: &str = "TEC.products";
const CAPITAL: &str = "TEC.capital";
const LABOUR: &str = "TEC.labour";
/// Each kind's unit, in TEC.capital's order of kinds.
pub const UNITS: [&str; 6] = [
    "structures",
    "transport equipment",
    "ICT equipment",
    "other machinery",
    "cultivated assets",
    "intellectual property",
];

/// What the opening's instructions are for: capital on both sides, since they open the books.
const REASON: ReasonDecl = ReasonDecl {
    name: "CAP opening",
    order: 0,
    paid: Effect::Equity,
    received: Effect::Equity,
    held: phx_num::Missing::Absent,
};

fn reason(b: &Books) -> ReasonId {
    b.ledger.reasons.named(REASON.name)
}

/// The plant's declarations in the books: its opening's reason.
#[clause("CAP.1")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "plant declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let _ = books::of(opening).ledger.reasons.declare(REASON);
    }
}

/// Every firm's plant at the opening, by kind and condition class.
#[clause("CAP.1", "GEN.5", "REP.24", "CAP.12")]
#[derive(Debug)]
pub struct Plant {
    pub prims: Prims,
    pub stock: Prim<Table1>,
}

/// A firm at the opening: its party, the persons it employs over all its twins, its twins and its industry.
struct Firm {
    party: PartyId,
    heads: u64,
    twins: u64,
    industry: usize,
    large: bool,
}

fn drawn(b: &Books, name: &str, c: &OpeningCountry) -> Vec<(PartyId, u64)> {
    let Some(list) = b.drawn.get(&key(name, c.id)) else {
        violation!(clause = "GEN.3", "firms read before they are drawn", country = c.id.get());
    };
    list.clone()
}

impl Contribution for Plant {
    fn name(&self) -> &'static str {
        "plant"
    }
    fn phase(&self) -> OpeningPhase {
        PHYSICAL_STOCK
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, SMALL_FIRMS, SMALL_COUNTS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[STRUCTURES_HELD, "CAP.plant_held"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[STRUCTURES_HELD]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["CAP.plant_held"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let register = opening.register;
        let Ok(kinds) = Kinds::compile(&self.prims, register) else {
            violation!(clause = "CAP.13", "kinds of plant the assembly let through");
        };
        let units: Vec<UnitId> = UNITS
            .iter()
            .map(|name| match register.units().named(name) {
                Missing::Present(u) => u,
                Missing::Absent => violation!(clause = "NUM.3", "a kind of plant with no unit"),
            })
            .collect();
        if units.len() != kinds.kinds.len() {
            violation!(clause = "CAP.13", "kinds of plant other than their units", kinds = kinds.kinds.len());
        }
        let countries = opening.countries;
        for c in countries {
            self.open_country(opening, c, (&kinds, &units));
        }
    }
}

impl Plant {
    fn open_country(&self, opening: &mut Opening<'_>, c: &OpeningCountry, (kinds, units): (&Kinds, &[UnitId])) {
        let (register, date) = (opening.register, opening.date);
        let intensity = intensities(register, c, kinds.kinds.len());
        let mut lot = opening.ctx.draws(&OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 0));
        let firms = firms(opening, c);
        let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
        let (b, report) = books::split(opening);
        let ccy = currency(c.id);
        let dates = ScheduleDates {
            anchor: date,
            period: months,
            eom: EndOfMonth::Plain,
            convention: BusinessDayConvention::Following,
            country: c.id,
        };
        let terms = b.ledger.terms.intern(Terms::account(ccy, dates));
        // Each kind's classes, newest first, one chain a kind tagged by its place.
        let mut classes: Vec<Vec<InstrumentId>> = Vec::with_capacity(kinds.kinds.len());
        for (tag, unit) in (0_u32..).zip(units) {
            let chain: Vec<InstrumentId> = (0..kinds.classes)
                .map(|_| {
                    b.ledger.instruments.issue(NewInstrument {
                        family: InstrumentFamily::RealAsset,
                        issuer: Missing::Absent,
                        unit: *unit,
                        ccy,
                        terms,
                    })
                })
                .collect();
            let _ = b.ledger.chains.declare(tag, chain.clone());
            classes.push(chain);
        }
        let growth = derived(c, "GEN.growth") / PERCENT;
        let stock = self.stock.get(register, c.id);
        let mut legs: Vec<Vec<LegRec>> = firms.iter().map(|_| Vec::new()).collect();
        let mut structures: Vec<(PartyId, u64)> = Vec::new();
        for (k, kind) in kinds.kinds.iter().enumerate() {
            let Some(ratio) =
                crate::kinds::decimals(stock, crate::kinds::places(&crate::STOCK_PER_GDP)).get(k).copied()
            else {
                violation!(clause = "GEN.2", "a kind of plant with no stock per unit of GDP", kind = k);
            };
            let Some(total) = floor_to_u64(ratio * c.gdp) else {
                violation!(clause = "GEN.2", "a country's plant beyond counting", country = c.id.get());
            };
            let weights: Vec<u64> = firms
                .iter()
                .map(|f| {
                    let per_hour = intensity.get(f.industry).and_then(|i| i.get(k)).copied().unwrap_or_else(|| {
                        violation!(clause = "TEC.4", "a firm of an industry the products do not declare")
                    });
                    floor_to_u64(from_u64(f.heads) * per_hour * WEIGHT_PARTS).unwrap_or_else(|| {
                        violation!(clause = "GEN.4", "a firm's weight beyond counting", firm = f.party.get())
                    })
                })
                .collect();
            if total == 0 || weights.iter().all(|w| *w == 0) {
                continue;
            }
            let twins: Vec<u64> = firms.iter().map(|f| f.twins).collect();
            let parts = apportion_in_units(total, &weights, &twins, &mut lot);
            let values = kind.values(kinds.classes);
            let steady = rules::wear::steady_weights(kinds.classes, kind.life, growth);
            let (Some(chain), Some(unit)) = (classes.get(k), units.get(k)) else { continue };
            for ((f, part), firm_legs) in firms.iter().zip(&parts).zip(legs.iter_mut()) {
                let per_twin = part / f.twins;
                if per_twin == 0 {
                    continue;
                }
                if k == 0 && f.large {
                    structures.push((f.party, per_twin));
                }
                let spread = rules::wear::steady_units(from_u64(per_twin), &steady, &values);
                for ((q, v), instrument) in spread.iter().zip(&values).zip(chain) {
                    let (Some(units_held), Some(cost)) = (floor_to_i64(*q), floor_to_i64(q * v)) else {
                        violation!(clause = "GEN.5", "a firm's plant beyond counting", firm = f.party.get());
                    };
                    if units_held > 0 {
                        firm_legs.push(hold(f.party, *instrument, units_held, *unit, cost, f.party.get()));
                    }
                }
            }
        }
        let reason = reason(b);
        for (f, firm_legs) in firms.iter().zip(legs) {
            if !firm_legs.is_empty() {
                b.open(reason, firm_legs, f.party.get(), report);
            }
        }
        b.drawn.insert(key(STRUCTURES_HELD, c.id), structures);
        report.distributions.push((
            key("CAP.plant", c.id),
            format!(
                "country {}: each kind's net stock the group's stock per unit of GDP times the country's GDP \
                 (CAP.stock_per_gdp), apportioned over every firm by its employees times its industry's plant of the \
                 kind per hour worked (TEC.capital over TEC.labour), and held in {} condition classes as a stock \
                 grown at {growth:.4} a year would be",
                c.id.get(),
                kinds.classes
            ),
        ));
    }
}

/// Every firm of the country: the large firms, one twin each, and the small firms' agents.
fn firms(opening: &mut Opening<'_>, c: &OpeningCountry) -> Vec<Firm> {
    let Opening { books, population, .. } = opening;
    let (Some(books), Some(population)) = (books.downcast_mut::<Books>(), population.downcast_mut::<Population>())
    else {
        violation!(clause = "GEN.3", "an opening handed something other than the world's books and population");
    };
    let industry = <Industry as FactDef>::ITEM.name;
    let industry_of = |i: i64| {
        usize::try_from(i)
            .unwrap_or_else(|_| violation!(clause = "TEC.4", "a firm of no industry the products declare", at = i))
    };
    let mut out: Vec<Firm> = Vec::new();
    for (party, heads) in drawn(books, FIRMS, c) {
        let Missing::Present(i) = books.parties.fact(party, industry) else {
            violation!(clause = "TEC.4", "a firm with no industry", firm = party.get());
        };
        out.push(Firm { party, heads, twins: 1, industry: industry_of(i), large: true });
    }
    let Some(at) = population.kinds.iter().position(|k| k.decl.kind == SMALL_FIRM) else {
        violation!(clause = "FRM.23", "a world that keeps no small firm kind");
    };
    let Some(industry_at) = population.kinds.get(at).and_then(|kd| kd.decl.attr(if_firm::known::INDUSTRY.name)) else {
        violation!(clause = "REP.41", "a small firm kind without its industry");
    };
    let counts = drawn(books, SMALL_COUNTS, c);
    let first = books.parties.first_cell_place();
    for ((party, heads), (counted, twins)) in drawn(books, SMALL_FIRMS, c).into_iter().zip(counts) {
        if party != counted {
            violation!(clause = "GEN.3", "small firms' draws in different orders", firm = party.get());
        }
        let (place, slot) = books.parties.row(party);
        let Some(table) = place.checked_sub(first).map(usize::from) else {
            violation!(clause = "FRM.23", "a small firm that is no agent", firm = party.get());
        };
        let agents = Population::table::<SystemBacking>(books.parties.cells(), table);
        let industry = industry_of(i64::from(agents.attr(slot, industry_at)));
        out.push(Firm { party, heads, twins, industry, large: false });
    }
    out
}

/// Each industry's plant of each kind per hour worked, over its products' ways in the country, each kind's scaled by
/// its largest so only the industries' shares of it matter.
fn intensities(register: &Register, c: &OpeningCountry, kinds: usize) -> Vec<Vec<f64>> {
    let Ok(products) = register.products(PRODUCTS) else {
        violation!(clause = "TEC.1", "the products unread");
    };
    let (Ok(capital), Ok(labour)) = (register.table2_in(CAPITAL, c.id), register.table2_in(LABOUR, c.id)) else {
        violation!(clause = "TEC.13", "a country's ways unread", country = c.id.get());
    };
    let mut industries: Vec<&str> = Vec::new();
    for p in products {
        if !industries.contains(&p.industry.as_str()) {
            industries.push(&p.industry);
        }
    }
    let cell = |t: &phx_core::register::values::Table2, row: i64, col: i64| {
        t.at(row, col).map_or_else(|_| violation!(clause = "TEC.13", "a way's cell off its table"), from_i64)
    };
    let mut out: Vec<Vec<f64>> = industries
        .iter()
        .map(|name| {
            let made: Vec<i64> = (0_i64..).zip(products).filter(|(_, p)| p.industry == *name).map(|(j, _)| j).collect();
            let hours: f64 = made.iter().map(|j| labour.rows().iter().map(|o| cell(labour, *o, *j)).sum::<f64>()).sum();
            (0_i64..)
                .take(kinds)
                .map(|k| {
                    let plant: f64 = made.iter().map(|j| cell(capital, k, *j)).sum();
                    // An industry whose ways take no hours has no employees to weigh its firms' plant by.
                    if hours > 0.0 { plant / hours } else { 0.0 }
                })
                .collect()
        })
        .collect();
    for k in 0..kinds {
        let most = out.iter().filter_map(|i| i.get(k)).fold(0.0_f64, |m, x| if *x > m { *x } else { m });
        if most > 0.0 {
            for i in &mut out {
                if let Some(x) = i.get_mut(k) {
                    *x /= most;
                }
            }
        }
    }
    out
}
