//! The opening's small firms: the country's firms below the individuals' rank as agents, one for every `k` firms under
//! twins, apportioned over the regions by their land and over the banks by the banks' drawn sizes, each with its own
//! size drawn from the firm-size law cut at the smallest firm the rank admits.

use phx_core::{Contribution, Opening, OpeningCountry, OpeningPhase, PARTIES, StreamDef, apportion, opening_subject};
use phx_id::PartyId;
use phx_ledger::books::Books;
use phx_ledger::opening::{derived, key, whole};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_pop::population::{PopKind, Population};
use phx_rand::AliasTable;
use phx_rand::float::{from_u64, len_u64};
use phx_store::SystemBacking;

use crate::consts::{AMOUNTS, CLASSES, PERCENT, SMALL_PURPOSES};
use crate::{BANK_ATTR, FixedPrim, REGION, SIZE, SMALL_FIRM, SmallStream};

const FIRMS: &str = "FRM.firms";
const DEBT: &str = "FRM.debt";
const DEPOSITS: &str = "FRM.deposits";
const PLANT: &str = "FRM.plant";
const BANKS: &str = "BNK.banks";
/// Each small-firm agent with the persons its twins employ, their deposits and their debt.
pub const SMALL_FIRMS: &str = "FRM.small_firms";
pub const SMALL_DEPOSITS: &str = "FRM.small_deposits";
pub const SMALL_DEBT: &str = "FRM.small_debt";
/// Each small-firm agent with the bank its twins bank with.
pub const SMALL_BANKS: &str = "FRM.small_banks";
/// Each small-firm agent with its twins.
pub const SMALL_COUNTS: &str = "FRM.small_counts";

/// Parties with an amount each, as the opening's draws are kept.
type Amounts = Vec<(PartyId, u64)>;

/// The primitives the small firms' opening reads.
#[derive(Clone, Copy, Debug)]
pub struct SmallPrims {
    pub firms_per_employed: FixedPrim,
    pub size_exponent: FixedPrim,
    pub deposit_share: FixedPrim,
    pub depreciation: FixedPrim,
}

/// Each country's small firms as agents, with their employees, deposits and debt drawn for the banks' and labour's
/// lines to read.
#[clause("FRM.23", "GEN.2", "GEN.3", "GEN.4", "REP.40")]
#[derive(Debug)]
pub struct SmallFirms {
    pub prims: SmallPrims,
}

/// The share of firms of a Pareto law of exponent `alpha` from one person up whose whole size is `k`, and so falls
/// between `k` and `k + 1`.
fn at_size(k: u64, alpha: f64) -> f64 {
    let k = from_u64(k);
    k.powf(-alpha) - (k + 1.0).powf(-alpha)
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

/// A small firm drawn: its region, its bank's place and party, its size.
struct Firm {
    region: u32,
    bank: (u32, PartyId),
    size: u64,
}

/// The country's small-firm agents apportioned over the regions by their land and each region's over the banks by the
/// banks' drawn sizes, each with its size drawn from the firm-size law below the cut.
#[clause("FRM.23", "GEN.2", "REP.41")]
fn draw_firms(
    c: &OpeningCountry,
    agents: u64,
    (cut, alpha): (u64, f64),
    banks: &[(PartyId, u64)],
    lot: &mut phx_rand::Draws,
) -> Vec<Firm> {
    let tiles: Vec<u64> = c.regions.iter().map(|(_, t)| len_u64(t.len())).collect();
    let bank_weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
    let sizes = AliasTable::new(&(1..cut).map(|k| at_size(k, alpha)).collect::<Vec<f64>>());
    let mut firms = Vec::new();
    for ((region, _), m) in c.regions.iter().zip(apportion(agents, &tiles, lot)) {
        for ((place, (bank, _)), k) in (1_u32..).zip(banks).zip(apportion(m, &bank_weights, lot)) {
            for _ in 0..k {
                let size = len_u64(sizes.draw(lot)) + 1;
                firms.push(Firm { region: *region, bank: (place, *bank), size });
            }
        }
    }
    firms
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
        let adults = 1.0 - derived(c, "GEN.share_under_15") / PERCENT;
        let employed = from_u64(c.people) * adults * derived(c, "GEN.employment_rate") / PERCENT;
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
        let k = u64::from(population.representation.multiplicity);
        let Some(kd) = population.kinds.get(at) else { violation!(clause = "FRM.23", "a kind beyond the world's") };
        let (region_at, size_at, bank_at) = (attr(kd, REGION.name), attr(kd, SIZE.name), attr(kd, BANK_ATTR));
        let alpha = p.size_exponent.shared(register).to_f64();
        let firms_drawn = draw_firms(c, small / k, (cut, alpha), &banks, &mut lot);
        let (tables, directory, space) = books.parties.cells_mut();
        let table = Population::table_mut::<SystemBacking>(tables, at);
        let mut cells: Vec<(PartyId, u64)> = Vec::with_capacity(firms_drawn.len());
        let mut banked: Vec<(PartyId, u64)> = Vec::with_capacity(firms_drawn.len());
        let mut counts: Vec<(PartyId, u64)> = Vec::with_capacity(firms_drawn.len());
        for f in &firms_drawn {
            let mut attrs = vec![0_u32; kd.decl.attrs.len()];
            let Ok(size) = u32::try_from(f.size) else { capacity_exceeded!("a small firm's size", u32::MAX, f.size) };
            for (i, v) in [(region_at, f.region), (size_at, size), (bank_at, f.bank.0)] {
                if let Some(x) = attrs.get_mut(i) {
                    *x = v;
                }
            }
            let Ok(twins) = u32::try_from(k) else { capacity_exceeded!("twins of an agent", u32::MAX, k) };
            let twins = phx_core::Weight::new(twins);
            let (_, party) = phx_pop::table::begin(table, directory, space, (*day, twins, &attrs));
            cells.push((party, f.size * k));
            banked.push((party, f.bank.1.get()));
            counts.push((party, k));
        }
        let wear = p.depreciation.shared(register).to_f64();
        let capital = derived(c, "GEN.investment") / PERCENT / (derived(c, "GEN.growth") / PERCENT + wear) * c.gdp;
        let deposits = p.deposit_share.shared(register).to_f64() * derived(c, "GEN.bank_deposits") / PERCENT * c.gdp;
        let debt = derived(c, "GEN.firm_debt") / PERCENT * c.gdp;
        let firms: Vec<(PartyId, u64)> = large.iter().chain(&cells).copied().collect();
        let heads: Vec<u64> = firms.iter().map(|(_, n)| *n).collect();
        let mut split = ctx.draws(&SmallStream::DECL, subject(AMOUNTS));
        // Each total apportioned over every firm by its drawn employees, the large firms first, a twin-th at a time,
        // so an agent's amount is a whole share for each of its twins.
        let mut share = |total: f64| -> (Amounts, Amounts) {
            let parts: Vec<(PartyId, u64)> = firms
                .iter()
                .zip(apportion(whole_u64(total) / k, &heads, &mut split))
                .map(|((f, _), a)| (*f, a * k))
                .collect();
            let (mine, theirs) = parts.split_at(large.len());
            (mine.to_vec(), theirs.to_vec())
        };
        let (large_deposits, deposits) = share(deposits);
        let (large_debt, debt) = share(debt);
        let (plant, _) = share(capital);
        let heads: Vec<u64> = cells.iter().map(|(_, n)| *n).collect();
        let employed_small: u64 = heads.iter().sum();
        let firms_small = k * len_u64(firms_drawn.len());
        let agents = firms_drawn.len();
        for (name, list) in [
            (SMALL_FIRMS, cells),
            (SMALL_DEPOSITS, deposits),
            (SMALL_DEBT, debt),
            (SMALL_BANKS, banked),
            (SMALL_COUNTS, counts),
            (DEPOSITS, large_deposits),
            (DEBT, large_debt),
            (PLANT, plant),
        ] {
            books.drawn.insert(key(name, c.id), list);
        }
        population.count(at, (firms_small, 0), (0, 0));
        report.distributions.push((
            key(SMALL_FIRMS, c.id),
            format!(
                "country {}: {firms_small} small firms below the individuals' rank's smallest of {cut} persons, as \
                 {agents} agents of {k} twins, employing {employed_small} of {employed:.0} employed; regions by land, \
                 banks by the banks' drawn sizes, each firm's size by the firm-size law (FRM.size_exponent); the \
                 firms' deposits, debt and plant shared over every firm by its employees, the small firms' plant not \
                 yet held",
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
        &[SMALL_FIRMS, SMALL_DEPOSITS, SMALL_DEBT, SMALL_BANKS, SMALL_COUNTS, DEPOSITS, DEBT, PLANT]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[SMALL_FIRMS, SMALL_BANKS, SMALL_COUNTS, DEPOSITS, DEBT, PLANT]
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

#[cfg(test)]
mod tests {
    use super::at_size;

    #[test]
    fn the_law_shares_sum_to_the_firms_from_one_person() {
        let alpha = 1.059;
        let within: f64 = (1..100_000_u64).map(|k| at_size(k, alpha)).sum();
        assert!((within - (1.0 - 100_000_f64.powf(-alpha))).abs() < 1e-12);
    }
}
