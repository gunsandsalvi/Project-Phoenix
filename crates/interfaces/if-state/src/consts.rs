//! The statistics agencies' series and how their values are laid out.

/// The consumer price index, over households' purchases at retail.
pub const CPI: usize = 0;
/// The producer price index, over firms' sales at the goods markets.
pub const PPI: usize = 1;
/// The labour force survey: the employed, the unemployed who search and the adults out of the labour force.
pub const LABOUR_FORCE: usize = 2;
/// The money stock: reserves, and deposits by the kind of their holder.
pub const MONEY: usize = 3;
/// The period life table: for each age class and health that was exposed, deaths and, for the able, onsets of
/// disability, each with its person-days exposed and its rate a year.
pub const LIFE_TABLE: usize = 4;
/// The national accounts: output measured by production, by expenditure and by income, and the discrepancy between
/// expenditure and income.
pub const ACCOUNTS: usize = 5;
/// How many series an agency publishes.
pub const SERIES: usize = 6;
/// Each series' name, in its order.
pub const NAMES: [&str; SERIES] =
    ["STA.cpi", "STA.ppi", "STA.labour_force", "STA.money", "STA.life_table", "STA.accounts"];
/// The decimal places each series' values are published to: the two indices' levels, counts of persons, money in its
/// smallest unit, rates a year, and money again.
pub const PLACES: [u8; SERIES] = [6, 6, 0, 0, 9, 0];
/// A life table's entry: its age class, health and event (nought a death, one an onset), the events, the person-days
/// exposed, and the rate a year at the series' places.
pub const LIFE_ENTRY: usize = 6;
/// The holders' legal forms the money stock's deposits are published by, all others after them in one class.
pub const HOLDER_CLASSES: [&str; 2] = ["household", "company"];
/// The labour force survey's states: employed, unemployed and searching, out of the labour force.
pub const LABOUR_STATES: usize = 3;
