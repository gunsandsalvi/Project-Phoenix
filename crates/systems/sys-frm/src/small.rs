//! The opening's small firms: the country's firms below the individuals' rank as agents, apportioned over the regions
//! by their land and over the banks by the banks' drawn sizes, each with its own size drawn from the firm-size law cut
//! at the smallest firm the rank admits.

use phx_core::Directory;
use phx_core::{Contribution, Opening, OpeningCountry, OpeningPhase, PARTIES, StreamDef, apportion, opening_subject};
use phx_id::{Day, PartyId};
use phx_ledger::books::Books;
use phx_ledger::opening::{derived, key, whole};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_pop::population::{PopKind, Population};
use phx_pop::table::AgentTable;
use phx_rand::AliasTable;
use phx_rand::float::{from_u64, len_u64};
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::{AMOUNTS, CLASSES, PERCENT, SMALL_INDUSTRIES, SMALL_PURPOSES};
use crate::{BANK_ATTR, FixedPrim, REGION, SIZE, SMALL_FIRM, SmallStream, TablePrim};

const FIRMS: &str = "FRM.firms";
const DEBT: &str = "FRM.debt";
const DEPOSITS: &str = "FRM.deposits";
const BANKS: &str = "BNK.banks";
/// Each small-firm agent with the persons it employs, its deposits and its debt.
pub const SMALL_FIRMS: &str = "FRM.small_firms";
pub const SMALL_DEPOSITS: &str = "FRM.small_deposits";
pub const SMALL_DEBT: &str = "FRM.small_debt";
/// Each small-firm agent with the bank it banks with.
pub const SMALL_BANKS: &str = "FRM.small_banks";
/// Each small firm agent's region, as the jobs' opening places its employees.
pub const SMALL_REGIONS: &str = "FRM.small_regions";
/// Each small firm agent's industry, as the jobs' opening weighs the occupations it employs.
pub const SMALL_INDUSTRIES_DRAWN: &str = "FRM.small_industries";

/// Parties with an amount each, as the opening's draws are kept.
type Amounts = Vec<(PartyId, u64)>;

/// The primitives the small firms' opening reads.
#[derive(Clone, Copy, Debug)]
pub struct SmallPrims {
    pub firms_per_employed: FixedPrim,
    pub size_exponent: FixedPrim,
    pub deposit_share: FixedPrim,
    pub industries: TablePrim,
}

/// Each country's small firms as agents, with their employees, deposits and debt drawn for the banks' and labour's
/// lines to read.
#[clause("FRM.23", "GEN.2", "GEN.3", "GEN.4", "REP.40")]
#[derive(Debug)]
pub struct SmallFirms {
    pub prims: SmallPrims,
}

fn drawn(b: &Books, name: &str, c: &OpeningCountry) -> Vec<(PartyId, u64)> {
    let Some(list) = b.drawn.get(&key(name, c.id)) else {
        violation!(clause = "GEN.3", "a small firms' draw read before it is drawn", country = c.id.get());
    };
    list.clone()
}

fn attr(kd: &PopKind, name: &str) -> usize {
    let Some(i) = kd.decl.attr(name) else {
        violation!(clause = "REP.41", "a small firm's attribute its kind does not hold");
    };
    i
}

/// A small firm drawn: its region, its bank's place and party, its size and its industry.
struct Firm {
    region: u32,
    bank: (u32, PartyId),
    size: u64,
    industry: u16,
}

/// The country's small-firm agents apportioned over the regions by their land and each region's over the banks by the
/// banks' drawn sizes, each with its size drawn from the firm-size law up to the cut and its industry by its size.
#[clause("FRM.23", "GEN.2", "REP.41")]
fn draw_firms(
    c: &OpeningCountry,
    agents: u64,
    (cut, law, by_size): (u64, crate::law::Law, &crate::industry::Industries),
    banks: &[(PartyId, u64)],
    (lot, trade): (&mut phx_rand::Draws, &mut phx_rand::Draws),
) -> Vec<Firm> {
    let tiles: Vec<u64> = c.regions.iter().map(|(_, t)| len_u64(t.len())).collect();
    let bank_weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
    let sizes = AliasTable::new(&(1..=cut).map(|k| law.at_size(k)).collect::<Vec<f64>>());
    let mut firms = Vec::new();
    for ((region, _), m) in c.regions.iter().zip(apportion(agents, &tiles, lot)) {
        for ((place, (bank, _)), k) in (1_u32..).zip(banks).zip(apportion(m, &bank_weights, lot)) {
            for _ in 0..k {
                let size = len_u64(sizes.draw(lot)) + 1;
                let industry = by_size.draw(size, trade);
                firms.push(Firm { region: *region, bank: (place, *bank), size, industry });
            }
        }
    }
    firms
}

/// Each drawn firm begun as an agent, its region, size, bank and industry at their `places` among the kind's `width`
/// attributes: the agents with the persons they employ, their banks, their regions and their industries.
fn begin_agents(
    (table, directory, space): (&mut AgentTable<SystemBacking>, &mut Directory, &mut AddressSpace),
    (day, width): (Day, usize),
    places: [usize; 4],
    firms: &[Firm],
) -> [Amounts; 4] {
    let mut cells: Amounts = Vec::with_capacity(firms.len());
    let mut banked: Amounts = Vec::with_capacity(firms.len());
    let mut regions: Amounts = Vec::with_capacity(firms.len());
    let mut industries: Amounts = Vec::with_capacity(firms.len());
    for f in firms {
        let mut attrs = vec![0_u32; width];
        let Ok(size) = u32::try_from(f.size) else { capacity_exceeded!("a small firm's size", u32::MAX, f.size) };
        for (i, v) in places.into_iter().zip([f.region, size, f.bank.0, u32::from(f.industry)]) {
            if let Some(x) = attrs.get_mut(i) {
                *x = v;
            }
        }
        let (_, party) = phx_pop::table::begin(table, directory, space, (day, &attrs));
        cells.push((party, f.size));
        banked.push((party, f.bank.1.get()));
        regions.push((party, u64::from(f.region)));
        industries.push((party, u64::from(f.industry)));
    }
    [cells, banked, regions, industries]
}

impl SmallFirms {
    #[clause("FRM.23", "GEN.2", "GEN.4")]
    fn open_country(&self, opening: &mut Opening<'_>, c: &OpeningCountry) {
        let Opening { ctx, register, books, population, report, day, .. } = opening;
        let (Some(books), Some(population)) = (books.downcast_mut::<Books>(), population.downcast_mut::<Population>())
        else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's books and population");
        };
        let p = self.prims;
        let employed = phx_ledger::opening::employed(c);
        let large = drawn(books, FIRMS, c);
        let Some(firms) = phx_rand::float::floor_to_u64(employed * p.firms_per_employed.get(register, c.id).to_f64())
        else {
            violation!(clause = "GEN.2", "a country's firms beyond counting", country = c.id.get());
        };
        let Some(small) = firms.checked_sub(len_u64(large.len())) else {
            violation!(clause = "REP.2", "a promotion rank beyond the country's firms", country = c.id.get());
        };
        // The large firms are drawn largest first, so the last is the smallest the rank admits.
        let Some(&(_, cut)) = large.last() else {
            violation!(clause = "REP.2", "a country with no firm within the promotion rank", country = c.id.get());
        };
        let subject = |purpose: u32| opening_subject(u32::from(c.id.get()) * SMALL_PURPOSES + purpose, 0);
        let mut lot = ctx.draws(&SmallStream::DECL, subject(CLASSES));
        let banks = drawn(books, BANKS, c);
        let Some(at) = population.kinds.iter().position(|k| k.decl.kind == SMALL_FIRM.name) else {
            violation!(clause = "FRM.23", "a world that keeps no small firm kind");
        };
        let Some(kd) = population.kinds.get(at) else { violation!(clause = "FRM.23", "a kind beyond the world's") };
        let at_of = |name: &str| attr(kd, name);
        let [region_at, size_at, bank_at, industry_at] =
            [REGION.name, SIZE.name, BANK_ATTR, if_firm::known::INDUSTRY.name].map(at_of);
        // The small firms take the persons the large leave, the law's scale set so they do in expectation.
        let persons = employed - from_u64(large.iter().map(|(_, n)| *n).sum());
        let alpha = p.size_exponent.shared(register).to_f64();
        let law = crate::law::Law { scale: crate::law::scale_to(persons, small, cut, alpha), alpha };
        let law = (cut, law, &crate::opening::industries(p.industries, register, c));
        let mut trade = ctx.draws(&SmallStream::DECL, subject(SMALL_INDUSTRIES));
        let firms_drawn = draw_firms(c, small, law, &banks, (&mut lot, &mut trade));
        let (tables, directory, space) = books.parties.cells_mut();
        let table = Population::table_mut::<SystemBacking>(tables, at);
        let places = [region_at, size_at, bank_at, industry_at];
        let [cells, banked, regions, industries] =
            begin_agents((table, directory, space), (*day, kd.decl.attrs.len()), places, &firms_drawn);
        let deposits = p.deposit_share.shared(register).to_f64() * derived(c, "GEN.bank_deposits") / PERCENT * c.gdp;
        let debt = derived(c, "GEN.firm_debt") / PERCENT * c.gdp;
        let firms: Vec<(PartyId, u64)> = large.iter().chain(&cells).copied().collect();
        let heads: Vec<u64> = firms.iter().map(|(_, n)| *n).collect();
        let mut split = ctx.draws(&SmallStream::DECL, subject(AMOUNTS));
        // Each total apportioned over every firm by its drawn employees.
        let mut share = |total: f64| -> (Amounts, Amounts) {
            let parts: Vec<(PartyId, u64)> =
                firms.iter().zip(apportion(whole_u64(total), &heads, &mut split)).map(|((f, _), a)| (*f, a)).collect();
            let (mine, theirs) = parts.split_at(large.len());
            (mine.to_vec(), theirs.to_vec())
        };
        let (large_deposits, deposits) = share(deposits);
        let (large_debt, debt) = share(debt);
        let heads: Vec<u64> = cells.iter().map(|(_, n)| *n).collect();
        let employed_small: u64 = heads.iter().sum();
        let firms_small = len_u64(firms_drawn.len());
        for (name, list) in [
            (SMALL_FIRMS, cells),
            (SMALL_DEPOSITS, deposits),
            (SMALL_DEBT, debt),
            (SMALL_BANKS, banked),
            (SMALL_REGIONS, regions),
            (SMALL_INDUSTRIES_DRAWN, industries),
            (DEPOSITS, large_deposits),
            (DEBT, large_debt),
        ] {
            books.drawn.insert(key(name, c.id), list);
        }
        population.count(at, (firms_small, 0), (0, 0));
        report.distributions.push((
            key(SMALL_FIRMS, c.id),
            format!(
                "country {}: {firms_small} small firms of up to the individuals' rank's smallest, {cut} persons, \
                 employing {employed_small} of {employed:.0} employed; regions by land, \
                 banks by the banks' drawn sizes, each firm's size by the firm-size law (FRM.size_exponent) and its \
                 industry by its size (FRM.industry_by_size); the \
                 firms' deposits and debt shared over every firm by its employees",
                c.id.get(),
            ),
        ));
    }
}

fn whole_u64(x: f64) -> u64 {
    let Ok(n) = u64::try_from(whole(x)) else { violation!(clause = "GEN.4", "an amount below nothing") };
    n
}

impl Contribution for SmallFirms {
    fn name(&self) -> &'static str {
        "small firms"
    }
    fn phase(&self) -> OpeningPhase {
        PARTIES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, BANKS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[SMALL_FIRMS, SMALL_DEPOSITS, SMALL_DEBT, SMALL_BANKS, DEPOSITS, DEBT]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[SMALL_FIRMS, SMALL_BANKS, DEPOSITS, DEBT]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[SMALL_DEPOSITS, SMALL_DEBT]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let countries = opening.countries;
        for c in countries {
            self.open_country(opening, c);
        }
    }
}
