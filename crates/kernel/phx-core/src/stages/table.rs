//! The day's stage table as data, slot by slot in the day's order: what each runs, reads and writes.

use super::AsOf::{Through, Today, Yesterday};
use super::Pool::{Apply, Chunked, Serial};
use super::{Read, SlotDecl};
use crate::consts::DAY_SLOTS;
use crate::slots::DaySlot as S;

const fn read(store: &'static str, as_of: super::AsOf) -> Read {
    Read { store, as_of }
}

/// A slot that runs on every day the table lists it for, followed by a barrier.
const fn slot(
    slot: S,
    (business_only, on_non_business): (bool, bool),
    runs: &'static [&'static str],
    reads: &'static [Read],
    writes: &'static [&'static str],
    pool: super::Pool,
) -> SlotDecl {
    SlotDecl { slot, business_only, on_non_business, runs, reads, writes, barrier_after: true, pool }
}

/// Runs every day.
const DAILY: (bool, bool) = (false, true);
/// Runs only for countries whose business day it is, never on a non-business day.
const BUSINESS: (bool, bool) = (true, false);

/// The day: its ten stages as slots, a non-business day's slots those that wait for no market or settlement, and the
/// save.
pub const DAY_TABLE: [SlotDecl; DAY_SLOTS] = [
    // 1 Open: today's buckets taken; lapses and the player's intents.
    slot(
        S::S1a,
        DAILY,
        &["K-42", "K-43", "K-44", "K-19", "K-20", "K-55"],
        &[read("agenda", Yesterday)],
        &["due", "facts", "policy", "terms"],
        Chunked,
    ),
    slot(
        S::S1b,
        DAILY,
        &["K-70", "K-45", "K-100"],
        &[read("offers", Yesterday), read("messages", Yesterday)],
        &["offers", "messages", "wakes"],
        Chunked,
    ),
    // 2 Resolve: accruals every day; the rest on business days.
    slot(S::S2a, DAILY, &["K-55", "K-87"], &[read("terms", Today), read("facts", Today)], &["accruals"], Chunked),
    slot(
        S::S2b,
        BUSINESS,
        &["K-51", "K-47", "K-12"],
        &[read("due", Today), read("accruals", Today)],
        &["pending"],
        Apply,
    ),
    slot(
        S::S2c,
        BUSINESS,
        &["K-99", "K-45", "K-48", "K-49"],
        &[read("due", Today), read("messages", Yesterday), read("resolutions", Yesterday)],
        &["pending", "resolutions"],
        Chunked,
    ),
    slot(S::S2d, BUSINESS, &["K-56"], &[read("fails", Yesterday)], &["status"], Chunked),
    slot(S::S2e, BUSINESS, &["K-65"], &[read("due", Today)], &["holder_events", "holdings"], Chunked),
    slot(
        S::S2f,
        BUSINESS,
        &["K-97", "K-96", "K-57", "K-98"],
        &[read("status", Today)],
        &["endings", "holdings"],
        Chunked,
    ),
    // 3 Nature and population.
    slot(
        S::S3a,
        DAILY,
        &["K-30", "K-27", "K-05", "K-60", "K-26"],
        &[read("facts", Today)],
        &["weather", "hazard_hits"],
        Chunked,
    ),
    slot(
        S::S3b,
        DAILY,
        &["K-44", "K-33", "K-31", "K-69"],
        &[read("hazard_hits", Today)],
        &["population", "holdings"],
        Chunked,
    ),
    // 4 Real work: production, then every trip placed, then each trip read at all the day's loads.
    slot(
        S::S4a,
        DAILY,
        &["K-68", "K-67", "K-69", "K-53"],
        &[read("due", Today), read("population", Today)],
        &["production", "holdings"],
        Chunked,
    ),
    slot(S::S4b, DAILY, &[], &[], &["loads"], Chunked),
    slot(S::S4c, DAILY, &[], &[read("loads", Today)], &["trip_costs"], Chunked),
    // 5 Decide: on yesterday's prints and publications.
    slot(
        S::S5a,
        DAILY,
        &["K-92"],
        &[read("prints", Yesterday), read("publications", Yesterday)],
        &["outlooks"],
        Chunked,
    ),
    slot(
        S::S5b,
        DAILY,
        &["K-43", "K-100", "K-35", "K-03"],
        &[read("outlooks", Today), read("holdings", Through(S::S4c))],
        &["decisions", "wants"],
        Chunked,
    ),
    slot(
        S::S5c,
        DAILY,
        &["K-101", "K-45", "K-80"],
        &[read("outlooks", Today), read("messages", Yesterday), read("holdings", Through(S::S4c))],
        &["decisions"],
        Chunked,
    ),
    slot(
        S::S5d,
        DAILY,
        &["K-70", "K-71", "K-72", "K-43", "K-46"],
        &[read("decisions", Today)],
        &["offers", "agenda", "triggers"],
        Apply,
    ),
    // 6 Form prices.
    slot(
        S::S6a,
        DAILY,
        &["K-73", "K-75", "K-82", "K-76", "K-78", "K-79", "K-81", "K-84", "K-83", "K-77", "K-85"],
        &[read("offers", Today), read("wants", Today)],
        &["matches"],
        Chunked,
    ),
    slot(
        S::S6b,
        DAILY,
        &["K-74", "K-87", "K-51", "K-39", "K-68", "K-60"],
        &[read("matches", Today)],
        &["sales", "holdings"],
        Chunked,
    ),
    slot(S::S6c, DAILY, &["K-86", "K-91"], &[read("sales", Today)], &["prints"], Chunked),
    slot(
        S::S6d,
        DAILY,
        &["K-48", "K-47", "K-50", "K-71"],
        &[read("matches", Today)],
        &["pending", "commitments"],
        Apply,
    ),
    // 7 Settle.
    slot(
        S::S7a,
        BUSINESS,
        &["K-47", "K-48", "K-50"],
        &[read("pending", Today), read("commitments", Today)],
        &["pending", "settled"],
        Apply,
    ),
    slot(S::S7b, BUSINESS, &["K-49", "K-74"], &[read("pending", Today)], &["settled"], Chunked),
    slot(S::S7c, BUSINESS, &["K-53", "K-54", "K-36"], &[read("settled", Today)], &["fails", "holdings"], Chunked),
    // 8 Fund, over the reserves settlement left.
    slot(S::S8a, BUSINESS, &["K-77"], &[read("settled", Today)], &["orders"], Chunked),
    slot(S::S8b, BUSINESS, &["K-76"], &[read("orders", Today)], &["funding"], Serial),
    slot(S::S8c, BUSINESS, &["K-49"], &[read("funding", Today)], &["funding_settled"], Serial),
    slot(S::S8d, BUSINESS, &["K-81", "K-93"], &[read("settled", Today)], &["facilities"], Chunked),
    slot(S::S8e, BUSINESS, &["K-47"], &[read("facilities", Today)], &["funding_settled"], Serial),
    slot(S::S8f, BUSINESS, &["K-53"], &[read("funding_settled", Today)], &["shortfalls"], Serial),
    // 9 Value and judge, on today's prints.
    slot(
        S::S9a,
        BUSINESS,
        &["K-91", "K-95", "K-89", "K-94", "K-14", "K-35", "K-92"],
        &[read("holdings", Today), read("prints", Today)],
        &["valuations"],
        Chunked,
    ),
    slot(
        S::S9b,
        BUSINESS,
        &["K-88", "K-46", "K-93", "K-87", "K-90", "K-65", "K-41"],
        &[read("valuations", Today)],
        &["books"],
        Chunked,
    ),
    slot(S::S9c, BUSINESS, &["K-45", "K-99"], &[read("books", Today)], &["tests", "messages", "resolutions"], Chunked),
    slot(
        S::S9d,
        BUSINESS,
        &["K-37", "K-39", "K-46", "K-43"],
        &[read("books", Today), read("tests", Today)],
        &["publications", "triggers"],
        Chunked,
    ),
    // 10 Close.
    slot(
        S::S10a,
        DAILY,
        &["K-36", "K-41"],
        &[read("hazard_hits", Today), read("population", Today)],
        &["public_events"],
        Chunked,
    ),
    slot(S::S10b, DAILY, &["K-14"], &[], &["sweeps"], Chunked),
    slot(S::S10c, DAILY, &["K-103"], &[read("pending", Today), read("holdings", Today)], &["audit"], Chunked),
    slot(
        S::S10d,
        DAILY,
        &["K-38", "K-39", "K-106", "K-40", "K-08", "K-15"],
        &[read("settled", Today), read("prints", Today), read("audit", Today)],
        &["ledger"],
        Chunked,
    ),
    slot(S::S10e, DAILY, &["K-02", "K-31", "K-03", "K-35"], &[read("ledger", Today)], &["day_buffers"], Chunked),
    // The save, taken at its declared moments, after the day and outside its stages.
    SlotDecl {
        slot: S::Save,
        business_only: false,
        on_non_business: true,
        runs: &["K-104"],
        reads: &[read("ledger", Today)],
        writes: &[],
        barrier_after: false,
        pool: Serial,
    },
];
