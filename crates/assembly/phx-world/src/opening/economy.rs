//! The opening dataset's identities, checked at assembly: a country group's flows, in shares of its GDP, must balance
//! before anything is drawn from them, and a dataset that does not is refused, naming the primitive and the residue;
//! each country's stocks are checked as its sheet is closed. Every number is stored to a declared number of places, so a sum of stored numbers may miss by
//! the rounding of its terms and no more.

use phx_core::Register;
use phx_core::register::values::ValueType;
use phx_id::CountryId;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::{from_i64, len_u64};

use crate::consts::SECTORS_WORTH_NOTHING;

/// A group's flows: each activity's output; the inputs of each activity (row) per unit of each activity's (column)
/// output, in value; the taxes on products per unit of each activity's output and then of each final use's spending;
/// each activity's value added by part; and what each final use takes of each activity.
#[derive(Clone, Debug, PartialEq)]
pub struct Flows {
    pub output: Vec<f64>,
    pub inputs: Vec<Vec<f64>>,
    pub taxes: Vec<f64>,
    pub added: Vec<Vec<f64>>,
    pub finals: Vec<Vec<f64>>,
}

/// A group's stocks: each financial instrument's holdings by sector (assets positive, liabilities negative) and each
/// sector's real assets.
#[derive(Clone, Debug, PartialEq)]
pub struct Stocks {
    pub financial: Vec<Vec<f64>>,
    pub real: Vec<Vec<f64>>,
}

/// What a sum of `terms` stored numbers, the largest `size`, may miss by when each was rounded to `unit`.
fn slack(terms: usize, size: f64, unit: f64) -> f64 {
    phx_rand::float::from_u64(len_u64(terms)) * unit * (1.0 + size.abs())
}

fn at(v: &[f64], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(f64::NAN)
}

fn cell(m: &[Vec<f64>], r: usize, c: usize) -> f64 {
    m.get(r).map_or(f64::NAN, |row| at(row, c))
}

/// A cell of the accounts after the assembly's check has held their shape: one they lack stops the run.
#[clause("GEN.15")]
pub(crate) fn entry(row: &[f64], column: usize) -> f64 {
    match row.get(column) {
        Some(v) => *v,
        None => violation!(clause = "GEN.15", "an accounts cell outside their checked shape", column = column),
    }
}

/// A row of the accounts after the assembly's check has held their shape: one they lack stops the run.
#[clause("GEN.15")]
pub(crate) fn line(m: &[Vec<f64>], r: usize) -> &[f64] {
    match m.get(r) {
        Some(row) => row,
        None => violation!(clause = "GEN.15", "an accounts row outside their checked shape", row = r),
    }
}

/// The flows' shape, each break named: an output an activity, an input of each activity to each, an activity's value
/// added in its parts, a final use's take of each activity, the taxes on products one an activity and one a final
/// use, and the activities and final uses the opening reads by their places.
#[clause("GEN.15")]
#[must_use]
pub fn shape_breaks(f: &Flows) -> Vec<String> {
    let n = f.output.len();
    let (Some(finals), Some(parts)) = (f.finals.first().map(Vec::len), f.added.first().map(Vec::len)) else {
        return vec![format!("GEN.final_composition: {n} activities and no final use or value added of any")];
    };
    let mut out = Vec::new();
    let rows = |id: &str, m: &[Vec<f64>], (count, width): (usize, usize), out: &mut Vec<String>| {
        if m.len() != count {
            out.push(format!("{id}: {} rows for {count}", m.len()));
        }
        for (i, row) in m.iter().enumerate().filter(|(_, r)| r.len() != width) {
            out.push(format!("{id}: row {i} has {} columns for {width}", row.len()));
        }
    };
    rows("TEC.inputs", &f.inputs, (n, n), &mut out);
    rows("GEN.final_composition", &f.finals, (n, finals), &mut out);
    rows("GEN.value_added_parts", &f.added, (n, parts), &mut out);
    if f.taxes.len() != n + finals {
        out.push(format!("GEN.product_taxes: {} rates for {n} activities and {finals} final uses", f.taxes.len()));
    }
    if n <= crate::consts::firm::PUBLIC_ADMINISTRATION {
        out.push(format!("GEN.final_composition: {n} activities hold no public administration"));
    }
    if finals <= crate::consts::final_use::TAXED {
        out.push(format!("GEN.final_weights: {finals} final uses for those bought at a sale"));
    }
    out
}

/// The flows' identities, each break named: every activity's supply is its uses, every activity's output is its
/// inputs, taxes on products and value added, and GDP by production and by expenditure is one.
#[clause("GEN.15", "GEN.4", "Law 7")]
#[must_use]
pub fn flows_breaks(f: &Flows, unit: f64) -> Vec<String> {
    let n = f.output.len();
    let finals = f.finals.first().map_or(0, Vec::len);
    let mut out = Vec::new();
    for i in 0..n {
        let x = at(&f.output, i);
        let used: f64 = (0..n).map(|j| cell(&f.inputs, i, j) * at(&f.output, j)).sum();
        let taken: f64 = (0..finals).map(|k| cell(&f.finals, i, k)).sum();
        let residue = x - used - taken;
        if residue.is_nan() || residue.abs() > slack(n + finals + 1, x, unit) {
            out.push(format!("GEN.final_composition: activity {i}'s supply misses its uses by {residue}"));
        }
    }
    for j in 0..n {
        let x = at(&f.output, j);
        let inputs: f64 = (0..n).map(|i| cell(&f.inputs, i, j)).sum();
        let added: f64 = f.added.get(j).map_or(f64::NAN, |r| r.iter().sum());
        let residue = x * (1.0 - inputs - at(&f.taxes, j)) - added;
        if residue.is_nan() || residue.abs() > slack(n + finals + 1, x, unit) {
            out.push(format!(
                "GEN.value_added_parts: activity {j}'s output misses its inputs and value added by {residue}"
            ));
        }
    }
    let spent: Vec<f64> = (0..finals).map(|k| (0..n).map(|i| cell(&f.finals, i, k)).sum()).collect();
    let final_taxes: f64 = spent.iter().enumerate().map(|(k, s)| s * at(&f.taxes, n + k)).sum();
    let product_taxes: f64 = (0..n).map(|j| at(&f.taxes, j) * at(&f.output, j)).sum();
    let added: f64 = f.added.iter().flatten().sum();
    let production = added + product_taxes + final_taxes;
    let expenditure = spent.iter().sum::<f64>() + final_taxes;
    let terms = n * (n + finals + f.added.first().map_or(0, Vec::len));
    for (name, gdp) in [("production", production), ("expenditure", expenditure)] {
        let residue = gdp - 1.0;
        if residue.is_nan() || residue.abs() > slack(terms, 1.0, unit) {
            out.push(format!("GEN.final_weights: GDP by {name} misses one by {residue}"));
        }
    }
    out
}

/// The stocks' identities, each break named: every instrument's assets are its liabilities, and the sectors worth
/// nothing beyond their equity — firms, banks, the central bank — are worth nothing.
#[clause("GEN.15", "GEN.4", "Law 7")]
#[must_use]
pub fn stocks_breaks(s: &Stocks, unit: f64) -> Vec<String> {
    let mut out = Vec::new();
    for (r, row) in s.financial.iter().enumerate() {
        let residue: f64 = row.iter().sum();
        let size = row.iter().fold(0.0_f64, |a, v| if v.abs() > a { v.abs() } else { a });
        if residue.is_nan() || residue.abs() > slack(row.len(), size, unit) {
            out.push(format!("the sheet: instrument {r}'s assets miss its liabilities by {residue}"));
        }
    }
    for sector in SECTORS_WORTH_NOTHING {
        let worth: f64 = s.financial.iter().chain(&s.real).map(|row| at(row, sector)).sum();
        if worth.is_nan() || worth.abs() > slack(s.financial.len() + s.real.len(), 1.0, unit) {
            out.push(format!("the sheet: sector {sector} is worth {worth} beyond its equity"));
        }
    }
    out
}

/// A table's values as numbers, at its declaration's places, with the unit of its last place.
pub(crate) fn table(register: &Register, id: &str, country: CountryId) -> Result<(Vec<Vec<f64>>, f64), String> {
    let (ValueType::Table1 { exp, .. } | ValueType::Table2 { exp, .. }) = register.decl_by_id(id)?.value else {
        return Err(format!("`{id}` is no table"));
    };
    let Ok(scale) = i64::try_from(phx_num::price::pow10(exp)) else {
        return Err(format!("`{id}` has too many places"));
    };
    let unit = 1.0 / from_i64(scale);
    if let Ok(t) = register.table2_in(id, country) {
        let cols = t.columns().len();
        let rows: Vec<Vec<f64>> = t
            .rows()
            .iter()
            .map(|r| t.columns().iter().map(|c| t.at(*r, *c).map_or(f64::NAN, |v| from_i64(v) * unit)).collect())
            .collect();
        if rows.iter().any(|r| r.len() != cols) {
            return Err(format!("`{id}` is ragged"));
        }
        return Ok((rows, unit));
    }
    let t = register.table1_in(id, country)?;
    Ok((vec![t.values().iter().map(|v| from_i64(*v) * unit).collect()], unit))
}

fn row(register: &Register, id: &str, country: CountryId) -> Result<Vec<f64>, String> {
    table(register, id, country)?.0.into_iter().next().ok_or_else(|| format!("`{id}` has no row"))
}

/// The solution of `m · x = b` by elimination, `m` square and its pivots nonzero as `I − A` for inputs worth less than
/// what they make; none where a pivot is nought.
#[must_use]
pub fn solve(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let size = rhs.len();
    let mut rows: Vec<Vec<f64>> =
        matrix.iter().zip(rhs).map(|(row, v)| row.iter().copied().chain([*v]).collect()).collect();
    for k in 0..size {
        let pivot = rows.get(k)?.get(k).copied()?;
        if pivot == 0.0 {
            return None;
        }
        let top = rows.get(k)?.clone();
        for row in rows.iter_mut().skip(k + 1) {
            let factor = row.get(k).copied()? / pivot;
            for (v, t) in row.iter_mut().zip(&top).skip(k) {
                *v -= factor * t;
            }
        }
    }
    let mut solution = vec![0.0; size];
    for k in (0..size).rev() {
        let row = rows.get(k)?;
        let known: f64 = (k + 1..size).map(|j| row.get(j).copied().unwrap_or(f64::NAN) * at(&solution, j)).sum();
        let v = (row.get(size).copied()? - known) / row.get(k).copied()?;
        *solution.get_mut(k)? = v;
    }
    Some(solution)
}

/// A country's accounts from the register's primitives: the products' inputs of products their ways' at the
/// country's opening prices, the rest the dataset's own; each final use's spending at basic prices its weight less its
/// taxes on products, shared over the activities by its composition; each activity's output what its uses need,
/// (I − A)⁻¹ f; its value added its output less its inputs and taxes on products, by its parts' shares. The unit is
/// the finest the primitives are stored to.
///
/// # Errors
/// A primitive the accounts read that the register does not hold, or inputs that leave no output to solve for.
#[clause("GEN.15", "GEN.4")]
pub fn accounts(register: &Register, country: CountryId) -> Result<(Flows, f64), String> {
    let (ways, _) = table(register, "TEC.inputs", country)?;
    let price: Vec<f64> = row(register, "GDS.opening_price", country)?
        .iter()
        .zip(row(register, "GDS.price_level", country)?)
        .map(|(a, b)| a * b)
        .collect();
    let (services, _) = table(register, "GEN.service_inputs", country)?;
    let (composition, unit) = table(register, "GEN.final_composition", country)?;
    let weights = row(register, "GEN.final_weights", country)?;
    let taxes = row(register, "GEN.product_taxes", country)?;
    let (parts, _) = table(register, "GEN.value_added_parts", country)?;
    let n = composition.len();
    let products = ways.len();
    let kept = n - products;
    let inputs: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| match (i < products, j < products) {
                    (true, true) => cell(&ways, i, j) * at(&price, i) / at(&price, j),
                    (true, false) => cell(&services, kept + i, j),
                    (false, _) => cell(&services, i - products, j),
                })
                .collect()
        })
        .collect();
    let spent: Vec<f64> = weights.iter().enumerate().map(|(k, w)| w / (1.0 + at(&taxes, n + k))).collect();
    let finals: Vec<Vec<f64>> =
        composition.iter().map(|r| r.iter().zip(&spent).map(|(c, s)| c * s).collect()).collect();
    let leontief: Vec<Vec<f64>> =
        (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 } - cell(&inputs, i, j)).collect()).collect();
    let demand: Vec<f64> = finals.iter().map(|r| r.iter().sum()).collect();
    let Some(output) = solve(&leontief, &demand) else {
        return Err(format!("country {}: inputs that leave no output to solve for", country.get()));
    };
    let added: Vec<Vec<f64>> = (0..n)
        .map(|j| {
            let used: f64 = (0..n).map(|i| cell(&inputs, i, j)).sum();
            let va = at(&output, j) * (1.0 - used - at(&taxes, j));
            let (comp, other) = (cell(&parts, j, 0), cell(&parts, j, 1));
            vec![comp * va, (1.0 - comp - other) * va, other * va]
        })
        .collect();
    Ok((Flows { output, inputs, taxes, added, finals }, unit))
}

/// What the accounts must hold beyond their identities: no activity's output or value added below nothing, every part
/// of the holdings a share of its whole, and no real asset below nothing.
fn level_breaks(register: &Register, country: CountryId, f: &Flows) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (j, x) in f.output.iter().enumerate() {
        let va: f64 = f.added.get(j).map_or(f64::NAN, |r| r.iter().sum());
        if x.is_nan() || *x < 0.0 || va.is_nan() || va < 0.0 {
            out.push(format!("GEN.final_composition: activity {j}'s output {x} or value added {va} is below nothing"));
        }
    }
    let held = row(register, "GEN.holdings", country)?;
    for (i, v) in held.iter().enumerate().skip(1) {
        if v.is_nan() || *v < 0.0 || *v > 1.0 {
            out.push(format!("GEN.holdings: part {i}, {v}, is no share of its whole"));
        }
    }
    let deposits: f64 =
        [crate::consts::sheet::holdings::DEPOSITS_HOUSEHOLDS, crate::consts::sheet::holdings::DEPOSITS_FIRMS]
            .iter()
            .map(|i| at(&held, *i))
            .sum();
    if deposits > 1.0 {
        out.push(format!("GEN.holdings: households and firms hold {deposits} of the deposits"));
    }
    for (i, v) in row(register, "GEN.real_assets", country)?.iter().enumerate() {
        if v.is_nan() || *v < 0.0 {
            out.push(format!("GEN.real_assets: asset {i}, {v}, below nothing"));
        }
    }
    Ok(out)
}

/// What the labour and firm tables must hold against the accounts: the ways' hours one column an activity, none below
/// nothing, and every activity the world staffs — the products and public administration — asking some; a share of
/// each product's employed self-employed and a number of firms per person employed, a share each; each occupation's
/// women a share of it.
fn labour_breaks(register: &Register, country: CountryId, f: &Flows) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let activities = f.output.len();
    let (ways, _) = table(register, "TEC.labour", country)?;
    let products = table(register, "TEC.inputs", country)?.0.len();
    for (o, r) in ways.iter().enumerate() {
        if r.len() != activities {
            out.push(format!("TEC.labour: occupation {o} has {} columns for {activities} activities", r.len()));
        }
        if r.iter().any(|h| h.is_nan() || *h < 0.0) {
            out.push(format!("TEC.labour: occupation {o} asks hours below nothing"));
        }
    }
    for a in (0..products).chain([crate::consts::firm::PUBLIC_ADMINISTRATION]) {
        let asked: f64 = ways.iter().map(|r| at(r, a)).sum();
        if asked.is_nan() || asked <= 0.0 {
            out.push(format!("TEC.labour: activity {a}, which the world staffs, asks no hours"));
        }
    }
    let share = |id: &str, n: usize, v: &[f64], out: &mut Vec<String>| {
        if v.len() != n {
            out.push(format!("{id}: {} values for {n}", v.len()));
        }
        for (i, x) in v.iter().enumerate() {
            if x.is_nan() || *x < 0.0 || *x > 1.0 {
                out.push(format!("{id}: value {i}, {x}, is no share"));
            }
        }
    };
    share("LAB.self_employed_shares", products, &row(register, "LAB.self_employed_shares", country)?, &mut out);
    let density = row(register, "FRM.firms_per_employed", country)?;
    share("FRM.firms_per_employed", products, &density, &mut out);
    if let Some(i) = density.iter().position(|d| *d <= 0.0) {
        out.push(format!("FRM.firms_per_employed: product {i} makes no firm"));
    }
    share("LAB.women_by_occupation", ways.len(), &row(register, "LAB.women_by_occupation", country)?, &mut out);
    Ok(out)
}

/// Every country's dataset checked; the breaks, each naming its primitive and residue.
///
/// # Errors
/// Each identity a country's dataset breaks, or a primitive the checks cannot read.
#[clause("GEN.15", "GEN.4")]
pub fn check(register: &Register, countries: usize) -> Result<(), Vec<String>> {
    let mut out = Vec::new();
    for c in 0..countries {
        let Ok(id) = u8::try_from(c) else { return Err(vec![format!("{countries} countries")]) };
        let country = CountryId::new(id);
        let read = || -> Result<Vec<String>, String> {
            let (flows, unit) = accounts(register, country)?;
            let mut breaks = shape_breaks(&flows);
            breaks.extend(flows_breaks(&flows, unit));
            breaks.extend(level_breaks(register, country, &flows)?);
            breaks.extend(labour_breaks(register, country, &flows)?);
            Ok(breaks)
        };
        match read() {
            Ok(breaks) => out.extend(breaks.into_iter().map(|b| format!("country {c}: {b}"))),
            Err(e) => out.push(format!("country {c}: {e}")),
        }
    }
    if out.is_empty() { Ok(()) } else { Err(out) }
}

#[path = "economy_tests.rs"]
mod tests;
