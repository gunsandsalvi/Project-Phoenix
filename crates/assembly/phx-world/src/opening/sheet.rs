//! A country's balance sheet at the opening, in shares of its GDP: its group's (`GEN.balance_sheet`,
//! `GEN.real_assets`) moved by its drawn profile. The levels the profile draws — households' and firms' debt, the
//! government's, the banks' deposits, capital and reserves — replace the group's, the group's splits of who holds
//! what stay, and the dataset's own closures balance the rest, so every instrument's assets are its liabilities and
//! firms, banks and the central bank are worth nothing beyond their equity, whatever was drawn.

use phx_core::{OpeningCountry, Register};
use phx_macros::clause;

use crate::consts::sheet::{
    BANKS, BANKS_BONDS, BANKS_EQUITY, CENTRAL_BANK, CENTRAL_BANK_LOANS, CURRENCY, DEPOSITS, FIRMS, FIRMS_BONDS,
    FIRMS_EQUITY, GOVERNMENT, GOVERNMENT_PAPER, HOUSEHOLDS, INSTRUMENTS, LOANS_TO_FIRMS, LOANS_TO_HOUSEHOLDS, RESERVES,
    SECTORS,
};
use crate::opening::economy::{Stocks, stocks_breaks, table};

/// The levels a country's profile draws, each a share of its GDP but the two ratios, which are shares of the banks'
/// assets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawn {
    pub household_debt: f64,
    pub firm_debt: f64,
    pub public_debt: f64,
    pub deposits: f64,
    pub capital_ratio: f64,
    pub reserves_ratio: f64,
}

/// A country's balance sheet: each instrument's holdings by sector, assets positive, and each sector's real assets.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub financial: Vec<[f64; SECTORS]>,
    pub real: Vec<[f64; SECTORS]>,
}

impl Sheet {
    /// A sector's holding of an instrument, a share of GDP.
    #[must_use]
    pub fn at(&self, instrument: usize, sector: usize) -> f64 {
        self.financial.get(instrument).and_then(|r| r.get(sector)).copied().unwrap_or(f64::NAN)
    }
}

fn cell(m: &[Vec<f64>], r: usize, c: usize) -> f64 {
    m.get(r).and_then(|row| row.get(c)).copied().unwrap_or(f64::NAN)
}

/// Writes an instrument's holdings, sector by sector.
fn put(m: &mut [[f64; SECTORS]], instrument: usize, cells: &[(usize, f64)]) {
    if let Some(row) = m.get_mut(instrument) {
        for (sector, v) in cells {
            if let Some(c) = row.get_mut(*sector) {
                *c = *v;
            }
        }
    }
}

/// Of two amounts, the one not above the other.
fn lesser(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

/// The central bank's side and the banks' bonds, closed around what the banks lend and hold: the banks' reserves at
/// their ratio of their assets, the central bank holding government paper for its currency and reserves and lending
/// banks what the paper cannot cover, the banks' equity at their capital ratio, and bonds, held by households, that
/// balance them. Returns (reserves, the central bank's paper, its loans, the banks' equity, the banks' bonds).
fn close_banks(d: &Drawn, (lent, deposits, currency, paper_b): (f64, f64, f64, f64)) -> (f64, f64, f64, f64, f64) {
    let reserves = lent * d.reserves_ratio / (1.0 - d.reserves_ratio);
    let paper_c = lesser(currency + reserves, d.public_debt - paper_b);
    let cb_loans = currency + reserves - paper_c;
    let assets = lent + reserves;
    let equity = assets * d.capital_ratio;
    (reserves, paper_c, cb_loans, equity, assets - deposits - equity - cb_loans)
}

/// The group's sheet moved by the drawn levels.
///
/// # Errors
/// A sheet the closures cannot balance: households holding less than no government paper, banks less than no bonds,
/// or firms owing more than they hold.
#[clause("GEN.15", "GEN.4", "Law 2")]
pub fn map(group: &Stocks, d: &Drawn) -> Result<Sheet, String> {
    let g = &group.financial;
    let currency = -cell(g, CURRENCY, CENTRAL_BANK);
    let held_by_banks = -cell(g, DEPOSITS, BANKS);
    let share = |instrument: usize, sector: usize, whole: f64| cell(g, instrument, sector) / whole;
    let firm_loans = cell(g, LOANS_TO_FIRMS, BANKS);
    let loans_share = firm_loans / (firm_loans + cell(g, FIRMS_BONDS, HOUSEHOLDS));
    let mut m = vec![[0.0; SECTORS]; INSTRUMENTS];
    put(
        &mut m,
        CURRENCY,
        &[
            (HOUSEHOLDS, share(CURRENCY, HOUSEHOLDS, currency) * currency),
            (FIRMS, share(CURRENCY, FIRMS, currency) * currency),
            (CENTRAL_BANK, -currency),
        ],
    );
    let dep = [HOUSEHOLDS, FIRMS, GOVERNMENT].map(|s| share(DEPOSITS, s, held_by_banks) * d.deposits);
    let deposits: f64 = dep.iter().sum();
    let [dep_h, dep_f, dep_g] = dep;
    put(&mut m, DEPOSITS, &[(HOUSEHOLDS, dep_h), (FIRMS, dep_f), (GOVERNMENT, dep_g), (BANKS, -deposits)]);
    let (loans_h, loans_f) = (d.household_debt, d.firm_debt * loans_share);
    let bonds_f = d.firm_debt - loans_f;
    put(&mut m, LOANS_TO_HOUSEHOLDS, &[(BANKS, loans_h), (HOUSEHOLDS, -loans_h)]);
    put(&mut m, LOANS_TO_FIRMS, &[(BANKS, loans_f), (FIRMS, -loans_f)]);
    put(&mut m, FIRMS_BONDS, &[(HOUSEHOLDS, bonds_f), (FIRMS, -bonds_f)]);
    let mut paper_b = d.public_debt * share(GOVERNMENT_PAPER, BANKS, -cell(g, GOVERNMENT_PAPER, GOVERNMENT));
    let lent = loans_h + loans_f + paper_b;
    let (mut reserves, mut paper_c, mut cb_loans, mut equity, mut bonds_b) =
        close_banks(d, (lent, deposits, currency, paper_b));
    let mut paper_h = d.public_debt - paper_b - paper_c;
    if bonds_b < 0.0 {
        // Deposits beyond what banks lend are held as more government paper, taken from households'.
        let extra = lesser(-bonds_b, paper_h);
        paper_b += extra;
        (reserves, paper_c, cb_loans, equity, bonds_b) = close_banks(d, (lent + extra, deposits, currency, paper_b));
        paper_h = d.public_debt - paper_b - paper_c;
    }
    if bonds_b < 0.0 || paper_h < 0.0 {
        return Err(format!("banks' bonds {bonds_b} or households' government paper {paper_h} below nothing"));
    }
    put(
        &mut m,
        GOVERNMENT_PAPER,
        &[(HOUSEHOLDS, paper_h), (BANKS, paper_b), (CENTRAL_BANK, paper_c), (GOVERNMENT, -d.public_debt)],
    );
    put(&mut m, RESERVES, &[(BANKS, reserves), (CENTRAL_BANK, -reserves)]);
    put(&mut m, CENTRAL_BANK_LOANS, &[(CENTRAL_BANK, cb_loans), (BANKS, -cb_loans)]);
    put(&mut m, BANKS_BONDS, &[(HOUSEHOLDS, bonds_b), (BANKS, -bonds_b)]);
    put(&mut m, BANKS_EQUITY, &[(HOUSEHOLDS, equity), (BANKS, -equity)]);
    let real: Vec<[f64; SECTORS]> =
        group.real.iter().map(|row| core::array::from_fn(|s| row.get(s).copied().unwrap_or(f64::NAN))).collect();
    let column = |rows: &[[f64; SECTORS]]| rows.iter().map(|r| r.get(FIRMS).copied().unwrap_or(f64::NAN)).sum::<f64>();
    let firms_worth = column(&real) + column(&m);
    if firms_worth < 0.0 {
        return Err(format!("firms owe more than they hold, by {}", -firms_worth));
    }
    put(&mut m, FIRMS_EQUITY, &[(HOUSEHOLDS, firms_worth), (FIRMS, -firms_worth)]);
    Ok(Sheet { financial: m, real })
}

/// A drawn level by name, a percentage made a share.
fn level(c: &OpeningCountry, name: &str) -> Result<f64, String> {
    c.derived(name).map(|v| v / crate::consts::PERCENT).ok_or_else(|| format!("no drawn `{name}`"))
}

/// A country's balance sheet: its group's moved by its drawn profile, refused where it breaks an identity.
///
/// # Errors
/// A group table or drawn level missing, a sheet the closures cannot balance, or one that breaks an identity.
#[clause("GEN.15", "GEN.4")]
pub fn country_sheet(register: &Register, c: &OpeningCountry) -> Result<Sheet, String> {
    let (financial, unit) = table(register, "GEN.balance_sheet", c.id)?;
    let (real, _) = table(register, "GEN.real_assets", c.id)?;
    let drawn = Drawn {
        household_debt: level(c, "GEN.household_debt")?,
        firm_debt: level(c, "GEN.firm_debt")?,
        public_debt: level(c, "GEN.public_debt")?,
        deposits: level(c, "GEN.bank_deposits")?,
        capital_ratio: level(c, "GEN.bank_capital_ratio")?,
        reserves_ratio: level(c, "GEN.liquid_reserves")?,
    };
    let sheet = map(&Stocks { financial, real }, &drawn)?;
    let as_rows = |m: &[[f64; SECTORS]]| m.iter().map(|r| r.to_vec()).collect::<Vec<_>>();
    let breaks = stocks_breaks(&Stocks { financial: as_rows(&sheet.financial), real: as_rows(&sheet.real) }, unit);
    if let Some(b) = breaks.first() {
        return Err(format!("country {}: {b}", c.id.get()));
    }
    Ok(sheet)
}

#[path = "sheet_tests.rs"]
mod tests;
