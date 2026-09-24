use phx_core::calendar::rules::CountryRules;
use phx_core::{Calendar, CountryEntry, DataFile, Declarations, HandlerEntry, Prim, Register, Streams};
use phx_id::{CountryId, Date, Day};
use phx_rand::Seed;

use crate::graph::HandlerGraph;
use crate::opening::prims::GenPrims;

/// The kernel's own primitives, declared before any system's.
#[derive(Debug)]
pub struct KernelPrims {
    pub epoch: Prim<Date>,
    pub day_zero: Prim<Date>,
    pub calendar: Prim<CountryRules>,
    pub legal_forms: Prim<Vec<phx_core::LegalForm>>,
    pub opening: GenPrims,
    pub geo: phx_geo::GeoPrims,
    pub market: phx_market::reach::MarketPrims,
    pub acct: phx_acct::basis::AcctPrims,
}

impl KernelPrims {
    pub fn declare(d: &mut Declarations) -> KernelPrims {
        KernelPrims {
            epoch: d.prim(&phx_core::EPOCH),
            day_zero: d.prim(&phx_core::DAY_ZERO),
            calendar: d.prim(&phx_core::CALENDAR),
            legal_forms: d.prim(&phx_core::LEGAL_FORMS),
            opening: GenPrims::declare(d),
            geo: phx_geo::GeoPrims::declare(d),
            market: phx_market::reach::MarketPrims::declare(d),
            acct: phx_acct::basis::AcctPrims::declare(d),
        }
    }
}

/// What compiling the declarations gives: the register checked against the data, the calendar, the streams and the
/// handler graph.
#[derive(Debug)]
pub struct Compiled {
    pub register: Register,
    pub calendar: Calendar,
    pub day_zero: Day,
    pub streams: Streams,
    pub graph: HandlerGraph,
}

/// Compiles the declarations against the data.
///
/// # Errors
/// Every entry the register refuses, a calendar its rules cannot describe, a stream twice or a hash shared, and a
/// handler graph with two writers of one column or a write another reads.
pub fn compile(
    d: &mut Declarations,
    kernel: &KernelPrims,
    handlers: &[HandlerEntry],
    files: &[DataFile],
    countries: &[CountryEntry],
    seed: Seed,
) -> Result<Compiled, Vec<String>> {
    let register = std::mem::take(&mut d.prims).build(files, countries.len())?;
    let epoch = kernel.epoch.shared(&register);
    let day_zero_date = kernel.day_zero.shared(&register);
    let mut rules = Vec::with_capacity(countries.len());
    for i in 0..countries.len() {
        let id = CountryId::new(u8::try_from(i).map_err(|_| vec![format!("{} countries", countries.len())])?);
        rules.push((id, kernel.calendar.get(&register, id).clone()));
    }
    let calendar = Calendar::new(epoch, rules, day_zero_date.year()).map_err(|e| vec![e])?;
    let day_zero =
        calendar.day(day_zero_date).ok_or_else(|| vec![format!("day zero {day_zero_date:?} before the epoch")])?;
    let decls: Vec<_> = d.streams.iter().map(|(_, s)| *s).collect();
    let streams = Streams::new(seed, &decls)?;
    let graph = HandlerGraph::build(handlers)?;
    Ok(Compiled { register, calendar, day_zero, streams, graph })
}
