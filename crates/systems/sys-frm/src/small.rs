//! The opening's small firms: the country's firms below the promotion rank, their sizes by the firm-size law cut at
//! the smallest firm the rank admits, counted by employment size class, apportioned over the regions by their land
//! and over the banks by the banks' drawn sizes, and landed as cells keyed by region, size class and bank.

use phx_core::register::values::Partition;
use phx_core::{
    Contribution, Opening, OpeningCountry, OpeningPhase, PARTIES, Prim, StreamDef, apportion, opening_subject,
};
use phx_id::{LineId, PartyId};
use phx_ledger::algebra::Side;
use phx_ledger::books::Books;
use phx_ledger::opening::{derived, key, whole};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_pop::check::LineKinks;
use phx_pop::key::KeyRecord;
use phx_pop::landing::{Drawn, TenB, land_drawn};
use phx_pop::population::{PopKind, Population};
use phx_pop::profile::{Profile, ProfileLayout};
use phx_rand::float::{from_u64, len_u64};
use phx_store::SystemBacking;

use crate::consts::{AMOUNTS, CLASSES, PERCENT, SHARE_PARTS, SMALL_PURPOSES};
use crate::{BANK_ATTR, FixedPrim, REGION, SIZE, SMALL_FIRM, SmallStream};

const FIRMS: &str = "FRM.firms";
const DEBT: &str = "FRM.debt";
const DEPOSITS: &str = "FRM.deposits";
const BANKS: &str = "BNK.banks";
/// Each small-firm cell with the persons its firms employ, their deposits and their debt.
pub const SMALL_FIRMS: &str = "FRM.small_firms";
pub const SMALL_DEPOSITS: &str = "FRM.small_deposits";
pub const SMALL_DEBT: &str = "FRM.small_debt";
/// Each small-firm cell with the bank its firms bank with.
pub const SMALL_BANKS: &str = "FRM.small_banks";
/// Each small-firm cell with its firms.
pub const SMALL_COUNTS: &str = "FRM.small_counts";

/// The primitives the small firms' opening reads.
#[derive(Clone, Copy, Debug)]
pub struct SmallPrims {
    pub firms_per_employed: FixedPrim,
    pub size_exponent: FixedPrim,
    pub deposit_share: FixedPrim,
    pub classes: Prim<Partition>,
}

/// Each country's small firms landed as cells, with their employees, deposits and debt drawn for the banks' and
/// labour's lines to read.
#[clause("FRM.23", "GEN.2", "GEN.3", "GEN.4", "REP.14")]
#[derive(Debug)]
pub struct SmallFirms {
    pub prims: SmallPrims,
}

/// The small firms' rows are none as they land, so no line's kink can part them.
struct NoRows;

impl LineKinks for NoRows {
    fn points(&self, _: LineId, _: Side) -> Vec<i64> {
        Vec::new()
    }
}

/// The share of firms of a Pareto law of exponent `alpha` from one person up whose whole size is `k`, and so falls
/// between `k` and `k + 1`.
fn at_size(k: u64, alpha: f64) -> f64 {
    let k = from_u64(k);
    k.powf(-alpha) - (k + 1.0).powf(-alpha)
}

/// For each size class the law's firms below `cut` fall in, their share of those firms and their mean whole size, which
/// a class holding none has not: the classes' first sizes are `bounds`, each running to the next, the last to the cut.
#[clause("GEN.2")]
#[must_use]
pub fn classes(bounds: &[u64], cut: u64, alpha: f64) -> Vec<(f64, Missing<f64>)> {
    let below: f64 = (1..cut).map(|k| at_size(k, alpha)).sum();
    if below <= 0.0 {
        violation!(clause = "GEN.2", "no firm below the promotion rank's smallest", cut = cut);
    }
    bounds
        .iter()
        .enumerate()
        .map(|(i, first)| {
            let end = bounds.get(i + 1).copied().filter(|e| *e < cut).unwrap_or(cut);
            let (share, persons) = (*first..end).fold((0.0, 0.0), |(s, p), k| {
                let at = at_size(k, alpha);
                (s + at, p + at * from_u64(k))
            });
            if share > 0.0 { (share / below, Missing::Present(persons / share)) } else { (0.0, Missing::Absent) }
        })
        .collect()
}

fn weight(x: f64) -> u64 {
    let Ok(w) = u64::try_from(whole(x * SHARE_PARTS)) else {
        violation!(clause = "GEN.2", "a share below nothing");
    };
    w
}

fn drawn(b: &Books, name: &str, c: &OpeningCountry) -> Vec<(PartyId, u64)> {
    let Some(list) = b.drawn.get(&key(name, c.id)) else {
        violation!(clause = "GEN.3", "a small firms' draw read before it is drawn", country = c.id.get());
    };
    list.clone()
}

fn attr(kd: &PopKind, name: &str) -> usize {
    let Some(i) = kd.decl.key_attrs.iter().position(|a| a.item.name == name) else {
        violation!(clause = "REP.19", "a small firm's key attribute its kind does not hold");
    };
    i
}

fn to_u32(n: u64) -> u32 {
    let Ok(n) = u32::try_from(n) else { capacity_exceeded!("members of a cell", u32::MAX, n) };
    n
}

/// Each size class's small firms apportioned over the regions by their land and each region's over the banks by the
/// banks' drawn sizes, a part for each (class, region, bank) with the persons its firms employ and its bank.
#[clause("FRM.23", "GEN.2", "REP.19")]
fn strata(
    kd: &PopKind,
    c: &OpeningCountry,
    (by_class, shares): (&[u64], &[(f64, Missing<f64>)]),
    banks: &[(PartyId, u64)],
    lot: &mut phx_rand::Draws,
) -> Vec<(Drawn, u64, PartyId)> {
    let tiles: Vec<u64> = c.regions.iter().map(|(_, t)| len_u64(t.len())).collect();
    let bank_weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
    let (region_at, size_at, bank_at) = (attr(kd, REGION.name), attr(kd, SIZE.name), attr(kd, BANK_ATTR));
    let layout = ProfileLayout::new(&kd.decl.groups);
    let mut parts = Vec::new();
    for ((class, n), (_, mean)) in (0_u32..).zip(by_class).zip(shares) {
        if *n == 0 {
            continue;
        }
        let Missing::Present(mean) = *mean else {
            violation!(clause = "GEN.4", "firms apportioned to a size class the law gives none", class = class);
        };
        for ((region, _), m) in c.regions.iter().zip(apportion(*n, &tiles, lot)) {
            if m == 0 {
                continue;
            }
            for ((place, (bank, _)), k) in (1_u32..).zip(banks).zip(apportion(m, &bank_weights, lot)) {
                if k == 0 {
                    continue;
                }
                let mut record = KeyRecord::default();
                kd.decl.key.set(&mut record, region_at, *region);
                kd.decl.key.set(&mut record, size_at, class);
                kd.decl.key.set(&mut record, bank_at, place);
                let d = Drawn {
                    key: record,
                    weight: phx_core::Weight::new(to_u32(k)),
                    profile: Profile::empty(&layout),
                    rows: Vec::new(),
                };
                parts.push((d, whole_u64(from_u64(k) * mean), *bank));
            }
        }
    }
    parts
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
        let bounds: Vec<u64> = p
            .classes
            .shared(register)
            .bounds
            .iter()
            .map(|b| {
                let Ok(b) = u64::try_from(*b) else { violation!(clause = "GEN.2", "a size class below nothing") };
                b
            })
            .collect();
        let shares = classes(&bounds, cut, p.size_exponent.shared(register).to_f64());
        let subject = |purpose: u32| opening_subject(u32::from(c.id.get()) * SMALL_PURPOSES + purpose, 0);
        let mut lot = ctx.draws(&SmallStream::DECL, subject(CLASSES));
        let by_class = apportion(small, &shares.iter().map(|(s, _)| weight(*s)).collect::<Vec<_>>(), &mut lot);
        let banks = drawn(books, BANKS, c);
        let Some(at) = population.kinds.iter().position(|k| k.decl.kind == SMALL_FIRM.name) else {
            violation!(clause = "FRM.23", "a world that keeps no small firm kind");
        };
        let Some(kd) = population.kinds.get_mut(at) else { violation!(clause = "FRM.23", "a kind beyond the world's") };
        let parts = strata(kd, c, (&by_class, &shares), &banks, &mut lot);
        let persons: Vec<(u64, PartyId, u64)> =
            parts.iter().map(|(d, n, bank)| (*n, *bank, u64::from(d.weight.get()))).collect();
        let landed = land(books, kd, at, *day, parts.into_iter().map(|(d, _, _)| d).collect());
        let mut cells: Vec<(PartyId, u64)> = Vec::with_capacity(landed.resolved.len());
        let mut banked: Vec<(PartyId, u64)> = Vec::with_capacity(landed.resolved.len());
        let mut counts: Vec<(PartyId, u64)> = Vec::with_capacity(landed.resolved.len());
        for (id, cell) in &landed.resolved {
            let Some((n, bank, firms)) = usize::try_from(id.seq).ok().and_then(|i| persons.get(i)) else {
                violation!(clause = "GEN.3", "a landed part the opening did not draw", seq = id.seq);
            };
            cells.push((*cell, *n));
            banked.push((*cell, bank.get()));
            counts.push((*cell, *firms));
        }
        let total_small = |name: &str, total: f64| {
            let held: u64 = drawn(books, name, c).iter().map(|(_, a)| *a).sum();
            let Some(left) = whole_u64(total).checked_sub(held) else {
                violation!(clause = "GEN.4", "the large firms holding more than all firms", country = c.id.get());
            };
            left
        };
        let deposits = total_small(
            DEPOSITS,
            p.deposit_share.shared(register).to_f64() * derived(c, "GEN.bank_deposits") / PERCENT * c.gdp,
        );
        let debt = total_small(DEBT, derived(c, "GEN.firm_debt") / PERCENT * c.gdp);
        let heads: Vec<u64> = cells.iter().map(|(_, n)| *n).collect();
        let mut split = ctx.draws(&SmallStream::DECL, subject(AMOUNTS));
        let share = |total: u64, lot: &mut phx_rand::Draws| -> Vec<(PartyId, u64)> {
            let parts = apportion(total, &heads, lot);
            cells.iter().zip(parts).map(|((cell, _), a)| (*cell, a)).collect()
        };
        let (deposits, debt) = (share(deposits, &mut split), share(debt, &mut split));
        let employed_small: u64 = heads.iter().sum();
        let firms_small = small;
        for (name, list) in [
            (SMALL_FIRMS, cells),
            (SMALL_DEPOSITS, deposits),
            (SMALL_DEBT, debt),
            (SMALL_BANKS, banked),
            (SMALL_COUNTS, counts),
        ] {
            books.drawn.insert(key(name, c.id), list);
        }
        population.count(at, firms_small, 0);
        report.distributions.push((
            key(SMALL_FIRMS, c.id),
            format!(
                "country {}: {firms_small} small firms below the promotion rank's smallest of {cut} persons, employing \
                 {employed_small} of {employed:.0} employed, in {} cells by size class (FRM.size_classes), region \
                 (by land) and bank (by the banks' drawn sizes); the firms' deposits and debt the large firms do not \
                 hold, by their employees",
                c.id.get(),
                landed.new_cells,
            ),
        ));
    }
}

fn whole_u64(x: f64) -> u64 {
    let Ok(n) = u64::try_from(whole(x)) else { violation!(clause = "GEN.4", "an amount below nothing") };
    n
}

/// The country's small firms landed as cells in their kind's table.
fn land(
    books: &mut Books,
    kd: &mut PopKind,
    at: usize,
    today: phx_id::Day,
    drawn: Vec<Drawn>,
) -> phx_pop::landing::Landed {
    let Books { ledger, parties, .. } = books;
    let (cells, directory, space) = parties.cells_mut();
    let table = Population::table_mut::<SystemBacking>(cells, at);
    let PopKind { decl, keys, index, levels, place, .. } = kd;
    let mut ctx =
        TenB { ledger, table, place: *place, keys, directory, space, kind: decl, levels, kinks: &NoRows, today };
    land_drawn(&mut ctx, index, drawn)
}

impl Contribution for SmallFirms {
    fn name(&self) -> &'static str {
        "small firms"
    }
    fn phase(&self) -> OpeningPhase {
        PARTIES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, DEBT, DEPOSITS, BANKS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[SMALL_FIRMS, SMALL_DEPOSITS, SMALL_DEBT, SMALL_BANKS, SMALL_COUNTS]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[SMALL_FIRMS, SMALL_BANKS, SMALL_COUNTS]
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
    use super::classes;

    #[test]
    fn classes_share_the_firms_below_the_cut_and_hold_their_sizes() {
        let got = classes(&[1, 2, 10], 50, 1.0);
        let share: f64 = got.iter().map(|(s, _)| s).sum();
        assert!((share - 1.0).abs() < 1e-12);
        let [(_, one), (_, micro), (_, small)] = [got[0], got[1], got[2]];
        let at = |m: phx_num::Missing<f64>| match m {
            phx_num::Missing::Present(v) => v,
            phx_num::Missing::Absent => f64::NAN,
        };
        assert!((at(one) - 1.0).abs() < 1e-12);
        assert!(at(micro) > 2.0 && at(micro) < 10.0);
        assert!(at(small) > 10.0 && at(small) < 50.0);
    }

    #[test]
    fn a_class_beyond_the_cut_holds_no_firm() {
        let got = classes(&[1, 2, 10, 50], 20, 1.059);
        assert_eq!(got[3], (0.0, phx_num::Missing::Absent));
        assert!((got.iter().map(|(s, _)| s).sum::<f64>() - 1.0).abs() < 1e-12);
    }
}
