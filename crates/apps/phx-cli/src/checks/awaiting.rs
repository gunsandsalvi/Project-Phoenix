//! The live checks that read the old books, each kept by its identity until its port to the core reads the world
//! again.

use super::{Check, Outcome};
use crate::live_check;

pub const LC_0_01: Check = live_check! {
    id: "LC-0-01",
    title: "Every day has a record for every sub-step that should have run, and none for any that should not",
    from_step: "S0.11",
    retired: "the day's sub-steps and their records were the old kernel's dispatch; the core's day runs its parts in one order, read by LC-0-62",
};

pub const LC_0_03: Check = live_check! {
    id: "LC-0-03",
    title: "read-trace found no undeclared read, no read of a later write and no duplicate stream open",
    from_step: "S0.11",
    retired: "the read trace followed the old handlers' reads, deleted with them (S1.24); the core's rules read what they are handed",
};

pub const LC_0_04: Check = live_check! {
    id: "LC-0-04",
    title: "Every record is dated with the day and sub-step that wrote it",
    from_step: "S0.11",
    retired: "the record store dated by sub-step was the old kernel's, deleted with it (S1.24)",
};

pub const LC_0_05: Check = live_check! {
    id: "LC-0-05",
    title: "The world hash is the same whatever the worker count",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; the world runs once, and determinism is carried by construction",
};

pub const LC_0_06: Check = live_check! {
    id: "LC-0-06",
    title: "The world hash is the same whatever the registration order",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; canonical handler ids carry it by construction",
};

pub const LC_0_07: Check = live_check! {
    id: "LC-0-07",
    title: "Looking at the world changes nothing",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; observers reach the world only through &self",
};

pub const LC_0_08: Check = live_check! {
    id: "LC-0-08",
    title: "Adding a stream changes no other stream's draws",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; a stream's key comes from its own name alone",
};

pub const LC_0_10: Check = live_check! {
    id: "LC-0-10",
    title: "Each family's injection into the day-30 save lights that family alone",
    from_step: "S0.12",
    check: |_| Outcome::NotYet("awaits the core saved and loaded (S1.24 h)"),
};

pub const LC_0_13: Check = live_check! {
    id: "LC-0-13",
    title: "Each region's realised weather is within z = 6.1 of its declared climate for the month, adjusted for persistence",
    from_step: "S0.13",
    check: |_| Outcome::NotYet("awaits the weather's events on the core (S1.24 d)"),
};

pub const LC_0_14: Check = live_check! {
    id: "LC-0-14",
    title: "Catastrophe frequencies per hazard are within z = 6.1 of their declared rates over the run, and their losses cluster in place and time",
    from_step: "S0.13",
    check: |_| Outcome::NotYet("awaits the catastrophes' events on the core (S1.24 d)"),
};

pub const LC_0_15: Check = live_check! {
    id: "LC-0-15",
    title: "For every finite deposit, extracted plus remaining equals its opening quantity",
    from_step: "S0.13",
    check: |_| Outcome::NotYet("awaits extraction from deposits on the core (S1.24 d)"),
};

pub const LC_0_16: Check = live_check! {
    id: "LC-0-16",
    title: "For every issued instrument, holdings sum to the issued amount",
    from_step: "S0.14",
    check: |_| Outcome::NotYet("awaits instruments issued on the core (S1.24 g, the bills)"),
};

pub const LC_0_17: Check = live_check! {
    id: "LC-0-17",
    title: "Every line's two sides hold equal counts, and each side it lists equals its holders' rows",
    from_step: "S0.14",
    retired: "lines and their sides were the old ledger's; the core's contracts are edges with two named ends, read by the contracts family (LC-0-09)",
};

pub const LC_0_19: Check = live_check! {
    id: "LC-0-19",
    title: "Every instruction's paired legs sum to nothing per denomination; transformations and opening writes name their sources",
    from_step: "S0.15",
    retired: "instructions and their paired legs were the old ledger's; the core's flows move money and units whole, read by the money and goods families (LC-0-09)",
};

pub const LC_0_21: Check = live_check! {
    id: "LC-0-21",
    title: "Every fail has a cause, and its line kind's contract process recorded it at 2d",
    from_step: "S0.15",
    retired: "the line kinds' contract process at 2d was the old ledger's; the core holds a failed due in its contract's arrears, read by LC-0-55",
};

pub const LC_0_24: Check = live_check! {
    id: "LC-0-24",
    title: "The GEN report lists every opening write with party, amount and identity, and each distribution with its source",
    from_step: "S0.16",
    check: |_| Outcome::NotYet("awaits the core's opening report (S1.24 h)"),
};

pub const LC_0_28: Check = live_check! {
    id: "LC-0-28",
    title: "The settled set is sound and maximal, recomputed on the state at the start of stage 7",
    from_step: "S0.17",
    retired: "the settled set was the old ledger's fixed point; the core's settlement is tested at logic level in phx-core",
};

pub const LC_0_29: Check = live_check! {
    id: "LC-0-29",
    title: "Sampled levy amounts equal per member times count under their conventions",
    from_step: "S0.17",
    check: |_| Outcome::NotYet("awaits the levies sampled on the core (S1.24 g)"),
};

pub const LC_0_30: Check = live_check! {
    id: "LC-0-30",
    title: "The Prices family is clean every close",
    from_step: "S0.18",
    retired: "the Prices family read the old market instances, deleted with them (S1.24)",
};

pub const LC_0_31: Check = live_check! {
    id: "LC-0-31",
    title: "Every print traces to its match set, and every published failure to its meeting",
    from_step: "S0.18",
    retired: "prints and match sets were the old market instances', deleted with them (S1.24)",
};

pub const LC_0_32: Check = live_check! {
    id: "LC-0-32",
    title: "The markets' measures are published per market per day, for the markets that met",
    from_step: "S0.18",
    retired: "the markets' measures were the old market instances', deleted with them (S1.24)",
};

pub const LC_0_33: Check = live_check! {
    id: "LC-0-33",
    title: "Accounts is clean for every party with an equity account",
    from_step: "S0.19",
    check: |_| Outcome::NotYet("awaits the accounts on the core (S1.24 h)"),
};

pub const LC_0_34: Check = live_check! {
    id: "LC-0-34",
    title: "Receivables equal payables across the world",
    from_step: "S0.19",
    check: |_| Outcome::NotYet("awaits the accounts on the core (S1.24 h)"),
};

pub const LC_0_35: Check = live_check! {
    id: "LC-0-35",
    title: "Every save reads back to the world hash of its close",
    from_step: "S0.20",
    check: |_| Outcome::NotYet("awaits the core saved and loaded (S1.24 h)"),
};

pub const LC_0_36: Check = live_check! {
    id: "LC-0-36",
    title: "Save sizes and write times are recorded",
    from_step: "S0.20",
    check: |_| Outcome::NotYet("awaits the core saved and loaded (S1.24 h)"),
};

pub const LC_0_37: Check = live_check! {
    id: "LC-0-37",
    title: "Every agent's rows count its attachments, which name its persons, one a line",
    from_step: "S0.28",
    retired: "agents' rows and attachments were the old population tables', deleted with them (S1.24); every person is held once, read by LC-0-63",
};

pub const LC_0_38: Check = live_check! {
    id: "LC-0-38",
    title: "Every agent's multiplicity is its kind's, but the seated twins of one and their donors of one fewer",
    from_step: "S0.28",
    retired: "every agent is one party; there are no twins (decision 44)",
};

pub const LC_0_40: Check = live_check! {
    id: "LC-0-40",
    title: "Every booking the agenda holds for a day is read that day, and no other",
    from_step: "S0.22",
    retired: "the agenda was the old population's; the core books each household's processes on its wheels",
};

pub const LC_0_42: Check = live_check! {
    id: "LC-0-42",
    title: "Carried needs and notices are decided on the first day their decision point runs",
    from_step: "S0.22",
    retired: "carried needs and notices were the old decision points', deleted with the handlers (S1.24)",
};

pub const LC_0_43: Check = live_check! {
    id: "LC-0-43",
    title: "Agents are each population, their persons the persons counted, and lines' sides are equal",
    from_step: "S0.28",
    retired: "agents and lines' sides were the old kernel's; the core's persons are read by LC-0-63",
};

pub const LC_0_44: Check = live_check! {
    id: "LC-0-44",
    title: "No landing crossed a kink: sampled landings re-checked against both sides' kinks",
    from_step: "S0.23",
    retired: "agents are never joined, so nothing lands (S0.28)",
};

pub const LC_0_45: Check = live_check! {
    id: "LC-0-45",
    title: "Sampled landings leave every straight rule's total unchanged to a smallest unit per member",
    from_step: "S0.23",
    retired: "agents are never joined, so nothing lands (S0.28)",
};

pub const LC_0_46: Check = live_check! {
    id: "LC-0-46",
    title: "The representation's measures are reported per day: agents, persons, hits and bookings",
    from_step: "S0.28",
    retired: "the representation's agents and bookings were the old kernel's; one person is one person on the core",
};

pub const LC_0_47: Check = live_check! {
    id: "LC-0-47",
    title: "The cells carried never exceed the cell budget at any close",
    from_step: "S0.24",
    retired: "there are no cells and no cell budget; the factor is the representation's one valve (S0.28)",
};

pub const LC_0_48: Check = live_check! {
    id: "LC-0-48",
    title: "No party with a public instrument is an agent",
    from_step: "S0.24",
    retired: "agents were the old kernel's; every party on the core is a party",
};

pub const LC_0_49: Check = live_check! {
    id: "LC-0-49",
    title: "The representation and its factor are reported every day, with its agents and persons",
    from_step: "S0.28",
    retired: "the representation's factor was the old kernel's; one person is one person on the core",
};

pub const LC_0_50: Check = live_check! {
    id: "LC-0-50",
    title: "The identity hash is equal immediately before and after each renumbering slice",
    from_step: "S0.24",
    retired: "agents are never renumbered (S0.28)",
};

pub const LC_0_53: Check = live_check! {
    id: "LC-0-53",
    title: "Every death has a cause and a destination for everything held and owed; every estate settles or waits, named",
    from_step: "S0.25",
    check: |_| Outcome::NotYet("awaits estates' destinations named on the core (S1.24 f)"),
};

pub const LC_0_57: Check = live_check! {
    id: "LC-0-57",
    title: "the player's queued intents are decided on the first day their point runs; with none, the rule decides",
    from_step: "S0.26",
    check: |_| Outcome::NotYet("awaits the player's party on the core (S1.24 h)"),
};

pub const LC_0_58: Check = live_check! {
    id: "LC-0-58",
    title: "every public event was made public by the declared rule, with a date and subjects",
    from_step: "S0.26",
    check: |_| Outcome::NotYet("awaits the public events on the core (S1.24 h)"),
};

pub const LC_0_61: Check = live_check! {
    id: "LC-0-61",
    title: "On sampled holders, every row due was read in the holder's run, and no head was later than its segment's earliest due day",
    from_step: "S0.17",
    retired: "the ledger's run of rows due was the old kernel's; the core's dues come off its wheels, read by LC-0-62",
};

pub const LC_1_06: Check = live_check! {
    id: "LC-1-06",
    title: "the family of the firms' revenue (FRM.17) is clean; the claims' (FRM.18) joins with the invoices (S2.02)",
    from_step: "S1.03",
    check: |_| Outcome::NotYet("awaits S1.24 h, the revenue family"),
};

pub const LC_1_10: Check = live_check! {
    id: "LC-1-10",
    title: "per owner and kind, plant next day is plant today plus completions less retirements plus transfers: \
            the family of the plant's stock (CAP.8) is clean and plant wears",
    from_step: "S1.04",
    check: |_| Outcome::NotYet("awaits S1.24 d, plant"),
};

pub const LC_1_11: Check = live_check! {
    id: "LC-1-11",
    title: "no output exceeds the capacity of the plant that made it (CAP.9)",
    from_step: "S1.04",
    check: |_| Outcome::NotYet("awaits S1.24 d, plant"),
};

pub const LC_1_12: Check = live_check! {
    id: "LC-1-12",
    title: "every investment is a purchase from a named producer, a commitment until delivery; investment's share, \
            volatility and responses and the plant's age are reported (CAP.10)",
    from_step: "S1.04",
    check: |_| Outcome::NotYet("awaits S1.24 d, plant"),
};

pub const LC_1_14: Check = live_check! {
    id: "LC-1-14",
    title: "for every finite deposit, extracted plus remaining equals its opening quantity: the family of deposits \
            (GEO.12) is clean",
    from_step: "S1.05",
    check: |_| Outcome::NotYet("awaits S1.24 d, extraction"),
};

pub const LC_1_15: Check = live_check! {
    id: "LC-1-15",
    title: "the reads of GDS.11 are reported: volatility against stocks, the basis between places against freight, \
            and producer prices moving before consumer prices",
    from_step: "S1.05",
    check: |_| Outcome::NotYet("awaits S1.24 h, the goods' reads"),
};

pub const LC_1_19: Check = live_check! {
    id: "LC-1-19",
    title: "FRT.9 and GEO.13: every shipment has one owner, one carrier and its goods pledged to it; no carrier books \
            beyond its vehicles' room and no segment beyond its capacity, which the carriage meeting refuses",
    from_step: "S1.07",
    check: |_| Outcome::NotYet("awaits S1.24 d, shipments"),
};

pub const LC_1_20: Check = live_check! {
    id: "LC-1-20",
    title: "FRT.10: freight rates and price gaps between places are reported, and gaps track freight",
    from_step: "S1.07",
    check: |_| Outcome::NotYet("awaits S1.24 d, shipments"),
};

pub const LC_1_25: Check = live_check! {
    id: "LC-1-25",
    title: "Declined applications are visible and counted per bank",
    from_step: "S1.09",
    check: |_| Outcome::NotYet("awaits S1.24 f, the banks' books"),
};

pub const LC_1_27: Check = live_check! {
    id: "LC-1-27",
    title: "MON.7 and MON.9 clean with the central bank's facilities in use; facility quantities are reported daily",
    from_step: "S1.10",
    check: |_| Outcome::NotYet("awaits S1.24 f, the central bank"),
};

pub const LC_1_28: Check = live_check! {
    id: "LC-1-28",
    title: "TAX.5: tax received equals tax remitted by named collectors; every tax payment has a named payer and base",
    from_step: "S1.11",
    check: |_| Outcome::NotYet("awaits S1.24 g, the taxes' collectors"),
};

pub const LC_1_29: Check = live_check! {
    id: "LC-1-29",
    title: "TRS.6: debt outstanding equals issuance minus redemptions, read from the register",
    from_step: "S1.11",
    check: |_| Outcome::NotYet("awaits S1.24 g, the bills"),
};

pub const LC_1_31: Check = live_check! {
    id: "LC-1-31",
    title: "Auction results (cover, tail, failures) are published",
    from_step: "S1.11",
    check: |_| Outcome::NotYet("awaits S1.24 g, the bills"),
};

pub const LC_1_33: Check = live_check! {
    id: "LC-1-33",
    title: "Households going without their needs are recorded as events and counted",
    from_step: "S1.12",
    check: |_| Outcome::NotYet("awaits needs by quantity, with the households' finances (S2.05)"),
};

pub const LC_1_39: Check = live_check! {
    id: "LC-1-39",
    title: "IDX.5: an index's return equals the weighted return of its constituents",
    from_step: "S1.14",
    retired: "market indices, whose returns it read, are built with S3.09",
};

pub const LC_1_46: Check = live_check! {
    id: "LC-1-46",
    title: "when a drought strikes one place, the price there rises before prices elsewhere (GDS.9)",
    from_step: "S1.05",
    check: |_| Outcome::NotYet("awaits S1.24 d, the weather's shocks"),
};

pub const LC_1_47: Check = live_check! {
    id: "LC-1-47",
    title: "every lien of goods in transit is released on arrival (FRT.6, FRT.8): no shipment stays in transit past \
            its day",
    from_step: "S1.07",
    check: |_| Outcome::NotYet("awaits S1.24 d, shipments"),
};
