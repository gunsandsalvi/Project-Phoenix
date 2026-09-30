/// Years of business days each country's calendar keeps as a bitset after the current year, from the epoch's year on;
/// days beyond are computed from the rules, and the window grows a year at each year's start. Sixty-four years cover
/// every contract date a run of a few decades reads.
pub const CALENDAR_WINDOW_YEARS: i32 = 64;
/// The business-day conventions ISDA defines: following, modified following, preceding, modified preceding and
/// unadjusted.
pub const CONVENTIONS: usize = 5;

/// A year without 29 February, against which a fixed holiday is checked to exist every year.
pub const COMMON_YEAR: i32 = 2001;
/// A month holds each weekday at least four times, so the fourth exists every year and the fifth may not.
pub const MAX_NTH: i8 = 4;

/// Gregorian computus (Meeus, Jones and Butcher, 1876): the golden-number cycle.
pub const EASTER_GOLDEN: i32 = 19;
/// Years per century in the computus.
pub const EASTER_CENTURY: i32 = 100;
/// The computus' divisor of four, for leap years and the century's quarters.
pub const EASTER_FOUR: i32 = 4;
/// The lunar correction's numerator offset and divisor: (b + 8) / 25.
pub const EASTER_F: [i32; 2] = [8, 25];
/// The solar correction: (b − f + 1) / 3.
pub const EASTER_G_DIV: i32 = 3;
/// The epact: (19a + b − d − g + 15) mod 30.
pub const EASTER_H: [i32; 2] = [15, 30];
/// The weekday correction: (32 + 2e + 2i − h − k) mod 7.
pub const EASTER_L: [i32; 2] = [32, 7];
/// The final correction: (a + 11h + 22l) / 451.
pub const EASTER_M: [i32; 3] = [11, 22, 451];
/// The month and day: month = (h + l − 7m + 114) / 31, day = (h + l − 7m + 114) mod 31 + 1.
pub const EASTER_MONTH: [i32; 3] = [7, 114, 31];

/// Days of the day counts' conventional years.
pub const DAYS_360: i64 = 360;
/// See `DAYS_360`.
pub const DAYS_365: i64 = 365;
/// See `DAYS_360`.
pub const DAYS_366: i64 = 366;
/// A 30/360 convention's month.
pub const DAYS_30: i64 = 30;
/// The day of the month a 30/360 convention counts its month's last day as.
pub const DAY_30: u8 = 30;
/// A 30/360 convention's day of the month that counts as its thirtieth.
pub const DAY_31: u8 = 31;
/// Months in a year.
pub const MONTHS: i32 = 12;
/// Days in a week, for week periods.
pub const DAYS_PER_WEEK: u16 = 7;
/// The shortest month, so a phase within any month period is shorter than every instance of it.
pub const SHORTEST_MONTH_DAYS: u16 = 28;

/// Business days per word of a country's bitset.
pub const CALENDAR_WORD_BITS: i64 = 64;

/// Parts per million, the unit of every share of a whole: a type set's shares sum to exactly this.
pub const PPM: u32 = 1_000_000;
/// Decimal places of a rate's fraction, as `Rate` holds it.
pub const RATE_EXP: u8 = 12;
/// Decimal places of a distribution's parameters in data: twelve keep any parameter written to six significant
/// figures exact across the magnitudes data uses.
pub const PARAM_EXP: u8 = 12;
/// `10^PARAM_EXP`, by which a parameter's integer is read as a number.
pub const PARAM_SCALE_F64: f64 = 1e12;
/// The base of decimal places.
pub const DECIMAL_RADIX_F64: f64 = 10.0;

/// The base of the register's decimal places, as an integer.
pub const DECIMAL_RADIX: i64 = 10;
/// Steps of a series, continued fraction or bisection before it stops: each converges to a double's precision in
/// well under a hundred steps for the parameters data declares, and a bisection halves its interval to below the
/// precision of any double within 200.
pub const NUMERIC_STEPS: u32 = 200;
/// The value a continued fraction's vanishing denominator is held at, far below any term it meets (Lentz's method).
pub const CF_TINY: f64 = 1e-300;
/// Months of a year and of a quarter, as primitives' periods state them.
pub const MONTHS_PER_YEAR: u16 = 12;
/// See `MONTHS_PER_YEAR`.
pub const MONTHS_PER_QUARTER: u16 = 3;
/// The draw address's ordinal of an opening phase: beyond every sub-step of a day, so opening draws share no address
/// with a day's.
pub const OPENING_ORDINAL_BASE: u8 = 64;
/// The day's slots the stage table documents: its thirty-three lettered slots, the six of funding, and the save.
pub const DAY_SLOTS: usize = 40;
/// The opening's first phase after the setup (0) and the map (1): the parties begin.
pub const OPENING_PARTIES: u8 = 2;
/// The parties' plant and stocks.
pub const OPENING_PHYSICAL_STOCK: u8 = 3;
/// Their contracts: lines opened with their terms and rows.
pub const OPENING_CONTRACTS: u8 = 4;
/// The present values the balances are set from.
pub const OPENING_PRESENT_VALUES: u8 = 5;
/// The balancing that closes every party's books.
pub const OPENING_BALANCES: u8 = 6;
/// The books' declarations — line kinds and reasons — which run before the parties begin, draw nothing, and are
/// replayed alone when a save is loaded; an ordinal past the others keeps their draws where they were.
pub const OPENING_DECLARATIONS: u8 = 7;
/// The draw address's ordinal of a keyed stream, beyond every day's and opening's ordinal.
pub const KEYED_ORDINAL: u8 = u8::MAX;

/// A whole, in percent, for the profile's percentages.
pub const PERCENT_F64: f64 = 100.0;

/// The month and day a birthday on the 29th of February falls on in a common year: the 1st of March.
pub const LEAP_BIRTHDAY_IN_COMMON_YEAR: (u8, u8) = (3, 1);

/// Ten, the base a fixed point's places count.
pub const DECIMAL_BASE: f64 = 10.0;

/// The bit of a flow's denomination that marks units, not money: a `u16`'s top bit.
pub const UNITS_BIT: u16 = 15;

/// The key a transformation's flow names on the side a counterparty would stand: nature's kind, the last of the 32 a key
/// holds, at its first slot.
pub const NATURE_WORD: u32 = 0xF800_0000;

/// The flows a word of a bitset marks.
pub const BIT_WORD: usize = 64;

/// Rows a store's chunk holds; a capacity is rounded up to whole chunks, as the stores reserve them.
pub const CHUNK_ROWS: u32 = 4096;
/// The bound on a store's yearly growth that grows with the persons, as a divisor of its rows: a twenty-fifth, 4 % a
/// year, above the fastest population growth any country in the sources shows, so two years' growth never meets the
/// ceiling.
pub const GROWTH_DIVISOR: u32 = 25;
/// Years of growth a capacity holds beyond the design point.
pub const GROWTH_YEARS: u32 = 2;
/// The width a row is reserved at until its base lays it out: a cache line, the widest a hot row takes.
pub const ROW_BYTES_UNLAID: u32 = 64;
/// Words an event's subjects and details take at most in the events' arena.
pub const EVENT_WORDS: u32 = 8;
/// Words a household's list of persons may take in its chunk's arena, at two a person: 512 persons, beyond any
/// household, since a list that outgrows its room stops the run. Address space only, committed as it is written.
pub const HOUSEHOLD_WORDS: u32 = 1024;
/// Days the due wheel files ahead: past a quarter, the longest period a bill or an anchor waits.
pub const WHEEL_DAYS: u32 = 128;
/// The longest quarter, July to September or October to December.
pub const LONGEST_QUARTER_DAYS: u32 = 92;
/// A chain link's and a wheel entry's family code, and its slot, in bits: a contract link's.
pub const FAMILY_BITS: u32 = phx_id::consts::FAMILY_BITS;
/// See `FAMILY_BITS`; a short reference's link is this same link.
pub const SLOT_BITS: u32 = phx_id::consts::FAMILY_SLOT_BITS;
/// The family code reserved for holdings, the highest the code holds.
pub const HOLDINGS_CODE: u8 = phx_id::consts::HOLDINGS_FAMILY;

/// The persons' store, which a constructor reads by name.
pub const PERSONS: (&str, u32, u32, crate::capacity::Growth) =
    ("persons", 6_000_000, 66, crate::capacity::Growth::Persons);
/// The institutions' store, which a constructor reads by name.
pub const INSTITUTIONS: (&str, u32, u32, crate::capacity::Growth) =
    ("institutions", 16_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons);
/// The events' store, which a constructor reads by name.
pub const EVENTS: (&str, u32, u32, crate::capacity::Growth) =
    ("events", 584_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons);
/// The instruments' store, which a constructor reads by name.
pub const INSTRUMENTS: (&str, u32, u32, crate::capacity::Growth) =
    ("instruments", 61_600, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons);

/// Events a day the event log records at the design point while it keeps every event of the run: the hazards'
/// hits measured at the committed resolution, about 690 a day at 750 000 persons, scaled to the design point's
/// persons, and the regions' weather.
pub const EVENTS_A_DAY: u32 = 5_600;
/// Days of the longest run the build makes while the event log keeps every event: a gate's, a year of settling and two
/// years run.
pub const EVENT_LOG_DAYS: u32 = 1_096;
/// The event log as it stands, every event of the run resident, until the log keeps occurrences on storage within a
/// horizon and this store goes.
pub const EVENTS_UNPRUNED: (&str, u32, u32, crate::capacity::Growth) =
    ("events_unpruned", EVENTS_A_DAY * EVENT_LOG_DAYS, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons);

/// Every store the finished world holds, as the design point names it: its rows at the design point, a row's bytes
/// (as its base lays it out, or reserved at a cache line until it does) and how it grows. The counts are the design
/// point's (`perf/design.toml [store]`); a capacity changes no outcome, since it is address space committed only as
/// rows are written.
pub const STORES: &[(&str, u32, u32, crate::capacity::Growth)] = &[
    PERSONS,
    ("households", 1_768_000, 174, crate::capacity::Growth::Persons),
    ("firms", 921_600, 520, crate::capacity::Growth::Persons),
    ("banks", 35, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    INSTITUTIONS,
    ("offices", 2_000_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("estates", 72_000, 256, crate::capacity::Growth::Persons),
    ("directory_slots", 8_800_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("tombstones", 672_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("accounts", 5_920_000, 20, crate::capacity::Growth::Persons),
    ("side_rows", 7_496_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("deposits", 50_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("holdings", 3_960_000, 33, crate::capacity::Growth::Persons),
    ("lots", 880_000, 20, crate::capacity::Growth::Persons),
    INSTRUMENTS,
    ("unit_rows", 3_600_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("physical_rows", 800_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("plant_cells", 9_400_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("named_units", 160_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("buildings", 1_280_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("processes", 800_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("liens", 960_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("terms", 640_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("ways", 50_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("stalls", 1_360_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("goods_with_stalls", 60_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("vacancies", 160_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("standing_orders", 160_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("dealer_quotes", 40_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("listings", 36_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("posted_rates", 5_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("market_instances", 46_400, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("capacity_resources", 921_600, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("wheel_rows", 25_200_000, 4, crate::capacity::Growth::Persons),
    ("wheel_far", 960_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("agenda_entries", 2_680_000, 8, crate::capacity::Growth::Persons),
    ("messages_live", 400_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("thresholds", 480_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("bureau_borrowers", 400_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("bureau_events", 560_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    ("filed_statements", 736_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
    EVENTS,
    EVENTS_UNPRUNED,
    ("segments", 500_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Fixed),
    ("segment_tiles", 1_000_000, 4, crate::capacity::Growth::Fixed),
    ("parcels", 800_000, 24, crate::capacity::Growth::Persons),
    ("zones", 1_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Fixed),
    ("regions", 25, ROW_BYTES_UNLAID, crate::capacity::Growth::Fixed),
    ("tiles", 40_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Fixed),
    ("products", 250, ROW_BYTES_UNLAID, crate::capacity::Growth::Fixed),
    ("unit_ids", 110_000, ROW_BYTES_UNLAID, crate::capacity::Growth::Persons),
];

/// A flow's grouping by its payer's range, declared at a flow's cost: about 25 ns of phone core time.
pub const FLOW_GROUP_COST: u64 = 25;
