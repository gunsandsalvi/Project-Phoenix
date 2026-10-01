/// Events per chunk of their column.
pub const EVENT_ROWS_PER_CHUNK: u32 = 1 << 14;
/// The world hash's key: any fixed value, so the same content always hashes the same.
pub const HASH_KEY: [u64; 2] = [0x5048_5820_574f_524c, 0x4420_4841_5348_2031];

/// The whole population, in the percent a setup's split is written in.
pub const WHOLE: u64 = 100;

/// Rows of a kind table per chunk, as the population's tables chunk theirs.
pub const KIND_ROWS_PER_CHUNK: u32 = 1 << 12;
/// A firm's making or its input orders, declared at a production visit's cost: about 800 ns of phone core time.
pub const FIRM_VISIT_COST: u32 = 800;
/// A hazard's follow of a household from its booking to today, declared at a hit's cost: about 800 ns of phone core
/// time.
pub const HAZARD_FOLLOW_COST: u64 = 800;
/// Rows of an agent table per chunk.
pub const AGENT_ROWS_PER_CHUNK: u32 = 1 << 12;

/// A save's format: a change of what a store holds or how it is written is a new format, and a load refuses others.
pub const SAVE_FORMAT: u32 = 25;
/// The file every save writes last, which makes it complete.
pub const SAVE_MANIFEST: &str = "manifest.json";
/// The suffix a save's directory carries until it is complete.
pub const SAVE_PARTIAL: &str = ".partial";
/// One agent in this many, by its identity's mix, is measured for the realised rates: enough for a year's sampling
/// error to be small against the rates' own, few enough to cost a small share of 3b.
pub const RATE_SAMPLE: u64 = 64;

/// Ten, the base a unit's price exponent counts powers of.
pub const DECADE: i64 = 10;

/// A half, added before a floor to round to the nearest.
pub const HALF: f64 = 0.5;

/// Metres in a kilometre, the unit buyers weigh a seller's distance in.
pub const METRES_PER_KM: f64 = 1_000.0;
/// Days of a week, over which weekly hours and applications are spread.
pub const DAYS_A_WEEK: f64 = 7.0;
/// The days the least match takes: a vacancy posted today is applied to tomorrow, offered the day after and
/// accepted the day after that; one filled so fast was offered too high.
pub const LEAST_MATCH_DAYS: u32 = 3;
/// The Gregorian calendar's mean days a year, over which a yearly return is spread.
pub const DAYS_A_YEAR: f64 = 365.2425;
/// Months of a year, for a loan's term in years and a month's share of a loan-year.
pub const MONTHS_A_YEAR: f64 = 12.0;
/// Months of a year, whole, for numbering months across years.
pub const MONTHS: i64 = 12;
/// Weeks of a year, for a bill's term in years.
pub const WEEKS_A_YEAR: f64 = DAYS_A_YEAR / DAYS_A_WEEK;
/// A rate of one in phx-num's rate scale, for a yearly rate as the ledger holds it.
pub const RATE_ONE: f64 = 1_000_000_000_000.0;
/// The opening balance sheet's sectors worth nothing beyond their equity, by their column: firms, banks and the central
/// bank (households are 0, the government 4).
pub const SECTORS_WORTH_NOTHING: [usize; 3] = [1, 2, 3];
/// The balance sheet's sectors by their column, and its instruments and closing real assets by their row, as the
/// dataset's derivation lays them out.
/// The families of dated contracts on the core, by name: every name a save of the core holds for them.
/// The final uses a sale is taxed as, by the tax on products per unit spent the accounts give each.
pub mod final_use {
    pub const HOUSEHOLDS: usize = 0;
    pub const COLLECTIVE: usize = 1;
    pub const INVESTMENT: usize = 2;
    /// The final uses bought at a sale; the changes in inventories are no one's purchase.
    pub const TAXED: usize = 3;
}

pub mod families {
    pub const EMPLOYMENT: &str = "LAB.employment";
    pub const PUBLIC_EMPLOYMENT: &str = "LAB.public_employment";
    pub const HOUSEHOLD_LOANS: &str = "BNK.household_loans";
    pub const FIRM_LOANS: &str = "BNK.firm_loans";
    pub const BENEFIT: &str = "SOC.benefit";
    pub const PENSION: &str = "SOC.pension";
    pub const DEPOSIT_FACILITY: &str = "CB.deposit_facility";
    pub const LENDING_FACILITY: &str = "CB.lending_facility";
    pub const BILLS: &str = "SOV.bills";
    pub const COLLECTED: &str = "TAX.collected";
    pub const COLLECTED_PUBLIC: &str = "TAX.collected_public";
    /// Each wage family and the family its payers owe the tax they withhold in: one per payer kind, as the wages are.
    pub const COLLECTORS: [(&str, &str); 2] = [(EMPLOYMENT, COLLECTED), (PUBLIC_EMPLOYMENT, COLLECTED_PUBLIC)];
    /// The families whose contracts are jobs: an employer's to the household whose person holds each.
    pub const JOBS: [&str; 2] = [EMPLOYMENT, PUBLIC_EMPLOYMENT];
    pub const ALL: [&str; 11] = [
        EMPLOYMENT,
        PUBLIC_EMPLOYMENT,
        HOUSEHOLD_LOANS,
        FIRM_LOANS,
        BENEFIT,
        PENSION,
        DEPOSIT_FACILITY,
        LENDING_FACILITY,
        BILLS,
        COLLECTED,
        COLLECTED_PUBLIC,
    ];
}

/// The statistics' numbers: the base the published places are powers of, and the money stock's classes — the banks'
/// reserves, then deposits held by households, by firms and by everyone else.
pub mod stats {
    pub const TEN: u64 = 10;
    pub const MONEY_CLASSES: usize = 4;
}
pub mod sheet {
    pub const HOUSEHOLDS: usize = 0;
    pub const FIRMS: usize = 1;
    pub const BANKS: usize = 2;
    pub const CENTRAL_BANK: usize = 3;
    pub const GOVERNMENT: usize = 4;
    pub const CURRENCY: usize = 0;
    pub const DEPOSITS: usize = 1;
    pub const LOANS_TO_HOUSEHOLDS: usize = 2;
    pub const LOANS_TO_FIRMS: usize = 3;
    pub const FIRMS_BONDS: usize = 4;
    pub const GOVERNMENT_PAPER: usize = 5;
    pub const RESERVES: usize = 6;
    pub const CENTRAL_BANK_LOANS: usize = 7;
    pub const BANKS_BONDS: usize = 8;
    pub const BANKS_EQUITY: usize = 9;
    pub const FIRMS_EQUITY: usize = 10;
    pub const INSTRUMENTS: usize = 11;
    pub const SECTORS: usize = 5;
    /// `GEN.holdings`' places: the currency over GDP, households' part of it, households' and firms' parts of the
    /// deposits, the part of firms' debt borrowed from banks, and banks' part of the government's paper.
    pub mod holdings {
        pub const CURRENCY: usize = 0;
        pub const CURRENCY_HOUSEHOLDS: usize = 1;
        pub const DEPOSITS_HOUSEHOLDS: usize = 2;
        pub const DEPOSITS_FIRMS: usize = 3;
        pub const FIRM_DEBT_LOANS: usize = 4;
        pub const GOVERNMENT_PAPER_BANKS: usize = 5;
    }
    /// `GEN.real_assets`' places, the real assets measured apart from plant.
    pub mod real {
        pub const INVENTORIES: usize = 0;
        pub const FIRMS_LAND: usize = 1;
        pub const DWELLINGS: usize = 2;
        pub const HOUSEHOLDS_LAND: usize = 3;
        pub const MEASURED: usize = 5;
    }
}
/// A percentage's whole.
pub const PERCENT: f64 = 100.0;
/// The core's ranges of slots settlement nets by, 2^bits slots each.
pub const CORE_RANGE_BITS: u32 = 12;
/// A core firm's record: its product, its region, the tile it is sited on, its productivity, the log of its factor
/// over its way's, in billionths, its posted price of a lot, its output a year in units, its markup over its unit cost
/// and the sales a day it expects in millionths, the units it sold since its last review and that review's day, its
/// management's memory and switching types, the heuristic it relies on, the width of its sales' surprises and its sales as last seen, the return its management requires; the purposes its opening draws and its
/// jobs' dealing are keyed by; and the column of the value added's parts that is labour's.
/// The income-statement lines a party with owners keeps: revenue, cost of sales, goods lost, services used, wages,
/// taxes, interest paid and received, written off and depreciation.
pub const LINES: usize = 10;

/// The loan classes a bank's lending record holds, its words in the bank map.
pub const LOAN_CLASSES: usize = 16;

pub mod bank {
    /// A reserves target's one: a share of deposits holds 32 bits of fraction.
    pub const SHARE_ONE: f64 = 4_294_967_296.0;
}

pub mod household {
    /// The stance's place in a household's states byte: its top three bits, above the tenure's two and the credit
    /// stage's three.
    pub const STANCE_SHIFT: u32 = 5;
    pub const STANCE_BITS: u32 = 3;
    /// The flag of a household whose last decision was to try for a child.
    pub const TRYING: u8 = 1;
}

pub mod firm {
    /// A share, a rate or a firm's sales in millionths, as a firm's words hold them.
    pub const PART_ONE: f64 = 1_000_000.0;
    /// A firm's productivity's log factor in hundred-millionths: a word of 32 bits spans ±21, past the drawn spread.
    pub const PRODUCTIVITY_ONE: f64 = 100_000_000.0;
    /// An output rate's one: a rate a day holds 32 bits of fraction below its units.
    pub const RATE_ONE: f64 = 4_294_967_296.0;
    pub const PURPOSES: u32 = 6;
    pub const PRODUCTIVITY_PURPOSE: u32 = 0;
    pub const SITE_PURPOSE: u32 = 1;
    pub const JOBS_PURPOSE: u32 = 2;
    pub const LOANS_PURPOSE: u32 = 3;
    pub const MANAGEMENT_PURPOSE: u32 = 4;
    pub const OWNERS_PURPOSE: u32 = 5;
    pub const COMPENSATION: usize = 0;
    /// The column of the value added's parts that is its gross operating surplus and mixed income.
    pub const SURPLUS: usize = 1;
    /// The flows' activity the public agencies' staff work in: public administration, after the products, finance and
    /// real estate.
    pub const PUBLIC_ADMINISTRATION: usize = 21;
}
/// The reasons the core's flows are made for, by their code.
pub mod reason {
    pub const PENSION: u8 = 1;
    pub const ESTATE: u8 = 2;
    pub const WAGE: u8 = 3;
    pub const SEVERANCE: u8 = 4;
    pub const SOLD: u8 = 5;
    pub const MADE: u8 = 6;
    pub const USED: u8 = 7;
    pub const DELIVERED: u8 = 8;
    pub const REPAID: u8 = 9;
    pub const TAXED: u8 = 10;
    pub const BENEFIT: u8 = 11;
    pub const LENT: u8 = 12;
    /// A service's capacity no sale took, lost at the day's end.
    pub const PERISHED: u8 = 13;
    /// Goods lost in stock at their product's rate.
    pub const SPOILED: u8 = 14;
    /// A central bank's income paid to its treasury.
    pub const REMITTED: u8 = 15;
    /// A treasury's money to its public agency for what the agency pays that day.
    pub const FUNDED: u8 = 16;
    /// Goods destroyed where a catastrophe struck.
    pub const DESTROYED: u8 = 17;
    /// Plant worn from one condition to the next, or retired from the last.
    pub const WORN: u8 = 18;
    /// Plant entering service: at the opening, or a project complete.
    pub const BUILT: u8 = 19;
    /// Freight paid for a trip.
    pub const CARRIED: u8 = 20;
    /// Goods leaving where they were on a trip, and arriving where they go.
    pub const SHIPPED: u8 = 21;
    pub const ARRIVED: u8 = 22;
    /// The reasons a day's record counts failed flows by: every reason above.
    pub const REASONS: usize = 23;
}
/// The bits a draw's subject gives a region beside its buyer, and a meeting's round beside its seller.
pub mod draws {
    pub const REGION_BITS: u32 = 16;
    pub const ROUND_BITS: u32 = 20;
}
