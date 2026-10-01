use crate::layout::IntTy::{I32, I64, U8, U16, U32, U64};
use crate::layout::{GroupDecl, KindMap, fits, used, width, word};

/// A person word's bits for its birth date: the calendar's civil day serial, in two's complement, so persons born
/// before the world's epoch are held exactly.
pub const BIRTH_BITS: u32 = 32;
/// A person word's bits for its role, after its birth date: room for eight roles.
pub const ROLE_BITS: u32 = 3;
/// A person word's bits for its kind's person attributes, after its role.
pub const PERSON_ATTR_BITS: u32 = u64::BITS - BIRTH_BITS - ROLE_BITS;
/// A tombstone's packed reference's bits below its day byte: kind, slot and generation.
pub const TOMB_KEY_BITS: u32 = 56;
/// The share of the main tombstone run the recent one may reach before they are merged: a sixteenth, so a day's merge
/// is rare and its cost amortised over the days that fill it.
pub const TOMB_MERGE_SHARE: usize = 16;
/// The main tombstone run's keys a fence entry stands for: a block of a kilobyte, so a lookup is a search of the fence,
/// which stays in cache, then of one block.
pub const TOMB_FENCE: usize = 64;
/// The groups a kind's store holds at most: hot, warm, cold and list headers.
pub const KIND_GROUPS: usize = 4;
/// The indexes a kind keeps on its words at most.
pub const KIND_INDEXES: usize = 4;
/// A kind's group column's chunk, in bytes: a power of two the column's pages commit by.
pub const KIND_CHUNK_BYTES: u32 = 1 << 18;
/// The bytes of each integer type a word may hold: one, two, four and eight.
pub const INT_BYTES: [u16; 4] = [1, 2, 4, 8];

/// A household's hot row, the two lines a visit gathers: its attributes (where it lives, a zone or a building, from
/// which its region and country are read; tenure, credit stage and stance packed in one byte; its preference type; its
/// flags; the day it formed), its persons' and contracts' heads, its positions (the income it received since it last
/// looked, its debt service and what it held after it last spent), its own outlooks (of its income a year, and two
/// ratios in `Fixed`), the month it last looked at its income, counted from the run's first, its age class, and its
/// agenda.
const HOUSEHOLD_HOT: GroupDecl = GroupDecl {
    name: "hot",
    width: 128,
    words: &[
        word("residence", U32, 1, true, "K-32"),
        word("states", U8, 1, false, "K-32"),
        word("preference", U16, 1, true, "K-32"),
        word("flags", U8, 1, false, "K-32"),
        word("formed", U32, 1, true, "K-32"),
        word("persons_head", U32, 1, true, "K-33"),
        word("chain_head", U32, 1, true, "K-53"),
        word("received", I64, 1, true, "K-32"),
        word("debt_service", I64, 1, true, "K-54"),
        word("after", I64, 1, true, "K-32"),
        word("income", I64, 1, true, "K-102"),
        word("outlooks", I32, 2, true, "K-102"),
        word("looked", U16, 1, true, "K-32"),
        word("window", U8, 1, true, "K-32"),
        word("agenda_base", U32, 1, true, "K-43"),
        word("agenda", U16, 27, true, "K-43"),
    ],
};

/// A household's warm row: two named-unit slots its persons or their groups own (owner, unit, count, cost), and its
/// ideal number of children, one more than the number once drawn.
const HOUSEHOLD_WARM: GroupDecl = GroupDecl {
    name: "warm",
    width: 46,
    words: &[
        word("unit_owner", U32, 2, true, "K-60"),
        word("unit_id", U32, 2, true, "K-60"),
        word("unit_count", U32, 2, false, "K-60"),
        word("unit_cost", I64, 2, false, "K-60"),
        word("ideal", U8, 1, true, "K-32"),
    ],
};

/// The household's byte map, 174 bytes.
pub const HOUSEHOLD: KindMap = KindMap { kind: "household", groups: &[HOUSEHOLD_HOT, HOUSEHOLD_WARM] };

/// A firm's hot row, what a production visit reads: its own and four inputs' unit ids, its stall, its own and its
/// inputs' stocks at cost, its work in progress, its exact output rate, its unit cost and the day it was reckoned, its
/// sales since its review and the expected sales that call one, and its flags.
const FIRM_HOT: GroupDecl = GroupDecl {
    name: "hot",
    width: 192,
    words: &[
        word("units", U32, 5, true, "K-60"),
        word("stall", U32, 1, true, "K-71"),
        word("own_stock", I64, 2, false, "K-60"),
        word("input_stocks", I64, 8, false, "K-60"),
        word("in_progress", I64, 1, false, "K-69"),
        word("rate", I64, 1, true, "K-68"),
        word("rate_anchor", U32, 1, true, "K-68"),
        word("realised", I64, 1, false, "K-68"),
        word("unit_cost", I64, 1, true, "K-35"),
        word("unit_cost_day", U16, 1, true, "K-35"),
        word("sales_since_review", I64, 2, false, "K-74"),
        word("expected_sales", I64, 1, true, "K-74"),
        word("flags", U32, 1, false, "K-32"),
    ],
};

/// A firm's warm row: its agenda, sales outlook (the sales a day it expects and the width of its surprises, in
/// millionths of a unit), wage bill, staff hours and plant capacity, plant occupancy, committed stock, identity
/// (account, zone, legal form, flags, founding preferences, memory and stance types, head's office, industry, founding
/// day), markup (in millionths), productivity (its log factor in hundred-millionths) and the day of its last price
/// review, equity and net assets, and cumulative output.
const FIRM_WARM: GroupDecl = GroupDecl {
    name: "warm",
    width: 192,
    words: &[
        word("agenda_base", U32, 1, true, "K-43"),
        word("agenda", U16, 24, true, "K-43"),
        word("expected", I64, 1, true, "K-102"),
        word("sales_width", I64, 1, true, "K-102"),
        word("wage_bill", I64, 1, false, "K-54"),
        word("staff_hours", U32, 4, false, "K-54"),
        word("plant_capacity", I64, 1, false, "K-67"),
        word("occupancy", U32, 1, false, "K-60"),
        word("overflow_run", U32, 1, true, "K-60"),
        word("committed", I64, 1, false, "K-60"),
        word("account", U32, 1, true, "K-47"),
        word("zone", U16, 1, true, "K-32"),
        word("legal_form", U8, 1, false, "K-32"),
        word("identity_flags", U8, 1, false, "K-32"),
        word("types", U8, 3, true, "K-32"),
        word("head_office", U32, 1, true, "K-34"),
        word("industry", U16, 1, true, "K-32"),
        word("founded", U32, 1, true, "K-32"),
        word("markup", I64, 1, true, "K-32"),
        word("productivity", I32, 1, true, "K-32"),
        word("last_review", U16, 1, true, "K-32"),
        word("equity", I64, 1, false, "K-88"),
        word("net_assets", I64, 1, false, "K-88"),
        word("cumulative_output", U64, 2, false, "K-68"),
    ],
};

/// A firm's cold row: its income-statement lines, tax accrued, trade-credit terms and equity issued.
const FIRM_COLD: GroupDecl = GroupDecl {
    name: "cold",
    width: 112,
    words: &[
        word("income_lines", I64, 10, false, "K-87"),
        word("tax_accrued", I64, 2, false, "K-51"),
        word("credit_terms", U32, 1, true, "K-55"),
        word("equity_issued", I64, 1, false, "K-64"),
    ],
};

/// A firm's list headers: its employer, invoice-seller and invoice-buyer lists (block, length, dead each), and its
/// holder chain's head.
const FIRM_LISTS: GroupDecl = GroupDecl {
    name: "lists",
    width: 32,
    words: &[
        word("list_blocks", U32, 3, true, "K-53"),
        word("list_lengths", U16, 3, false, "K-53"),
        word("list_dead", U16, 3, false, "K-53"),
        word("holder_chain", U32, 1, true, "K-53"),
    ],
};

/// The firm's byte map, 528 bytes.
pub const FIRM: KindMap = KindMap { kind: "firm", groups: &[FIRM_HOT, FIRM_WARM, FIRM_COLD, FIRM_LISTS] };

/// A bank's books: its income-statement lines, its equity and its net assets.
const BANK_BOOKS: GroupDecl = GroupDecl {
    name: "books",
    width: 96,
    words: &[
        word("income_lines", I64, 10, false, "K-87"),
        word("equity", I64, 1, false, "K-88"),
        word("net_assets", I64, 1, false, "K-88"),
    ],
};

/// The loan classes a bank's lending record holds, above the published classes of any country's law.
pub const LOAN_CLASSES: u16 = 16;

/// A bank's lending record: its standard (the worst class it admits), the applications it read, declined and quoted,
/// the loans it made, what its write-offs lost since its last review, and by class the loan-days it has held and the
/// defaults it has seen.
const BANK_LENDING: GroupDecl = GroupDecl {
    name: "lending",
    width: 240,
    words: &[
        word("standard", U32, 1, true, "BNK"),
        word("applications", U64, 1, false, "BNK"),
        word("declined", U64, 1, false, "BNK"),
        word("quoted", U64, 1, false, "BNK"),
        word("lent", U64, 1, false, "BNK"),
        word("written", I64, 1, false, "BNK"),
        word("loan_days", U64, LOAN_CLASSES, false, "BNK"),
        word("defaults", U32, LOAN_CLASSES, false, "BNK"),
    ],
};

/// A bank's reserves target: the share of its deposits its central bank's fund stage holds it to, in 2⁻³² parts.
const BANK_RESERVES: GroupDecl =
    GroupDecl { name: "reserves", width: 8, words: &[word("reserve_target", I64, 1, true, "CB")] };

/// The bank's byte map: its books, its lending record and its reserves target; its site is on its kind's place store.
pub const BANK: KindMap = KindMap { kind: "bank", groups: &[BANK_BOOKS, BANK_LENDING, BANK_RESERVES] };

/// The regions an agency's staffing record holds, more than any world's.
pub const AGENCY_REGIONS: u16 = 64;
/// The occupations an agency's staffing record holds: the labour law's.
pub const AGENCY_OCCUPATIONS: u16 = 11;

/// An agency's staffing: the staff it keeps by region and occupation, region-major, and its appropriation for wages
/// a month.
const AGENCY_STAFFING: GroupDecl = GroupDecl {
    name: "staffing",
    width: 2824,
    words: &[
        word("targets", U32, AGENCY_REGIONS * AGENCY_OCCUPATIONS, true, "SOC"),
        word("budget", I64, 1, false, "SOC"),
    ],
};

/// The agency's byte map: its staffing record; its site is on its kind's place store.
pub const AGENCY: KindMap = KindMap { kind: "agency", groups: &[AGENCY_STAFFING] };

/// A party placed by its site: the tile it stands on.
const SITE_PLACE: GroupDecl = GroupDecl { name: "place", width: 4, words: &[word("site", U32, 1, true, "K-32")] };
/// A party placed by its region.
const REGION_PLACE: GroupDecl = GroupDecl { name: "place", width: 4, words: &[word("region", U32, 1, true, "K-32")] };
/// A party placed by its country.
const COUNTRY_PLACE: GroupDecl = GroupDecl { name: "place", width: 1, words: &[word("country", U8, 1, true, "K-32")] };

/// The places of the kinds placed by a site, a region or a country: each such kind's store holds its parties' place.
pub const SITED: KindMap = KindMap { kind: "sited", groups: &[SITE_PLACE] };
pub const REGIONED: KindMap = KindMap { kind: "regioned", groups: &[REGION_PLACE] };
pub const COUNTRIED: KindMap = KindMap { kind: "countried", groups: &[COUNTRY_PLACE] };

/// Every group of the maps within its width, and each map within a store.
const _: () = assert!(
    fits(&HOUSEHOLD)
        && fits(&FIRM)
        && fits(&BANK)
        && fits(&AGENCY)
        && fits(&SITED)
        && fits(&REGIONED)
        && fits(&COUNTRIED)
);
/// The household's 174 bytes, 128 of them hot: two cache lines a visit gathers.
const _: () = assert!(width(&HOUSEHOLD) == 174 && HOUSEHOLD_HOT.width == 128);
/// The firm's 528 bytes, 192 of them hot and 192 warm: three cache lines each.
const _: () = assert!(width(&FIRM) == 528 && FIRM_HOT.width == 192 && FIRM_WARM.width == 192);
/// The reserves later steps fill: the household's 12 bytes.
const _: () = assert!(HOUSEHOLD_HOT.width - used(&HOUSEHOLD_HOT) + HOUSEHOLD_WARM.width - used(&HOUSEHOLD_WARM) == 12);
/// The firm's reserve, 39 bytes.
const _: () = assert!(
    FIRM_HOT.width - used(&FIRM_HOT) + FIRM_WARM.width - used(&FIRM_WARM) + FIRM_COLD.width - used(&FIRM_COLD)
        + FIRM_LISTS.width
        - used(&FIRM_LISTS)
        == 39
);
