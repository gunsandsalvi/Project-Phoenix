use phx_core::{Bindings, EventStore, KernelTable, PlayerQueue, RecordStore};
use phx_id::Day;
use phx_macros::clause;
use phx_store::LogicalHasher;

use crate::consts::HASH_KEY;
use crate::world::World;

/// The world store's content: the day, the fails waiting for the contract process, the player's queue, the bindings
/// and the closed fact.
pub(crate) fn hash_world_store(
    h: &mut LogicalHasher,
    today: Day,
    unprocessed: &Vec<phx_ledger::fails::Fail>,
    queue: &PlayerQueue,
    bindings: &Bindings,
    closed: &phx_ledger::pending::Closed,
) {
    h.u64(u64::from(today.get()));
    phx_store::hash_saved(unprocessed, h);
    phx_store::hash_saved(queue, h);
    phx_store::hash_saved(bindings, h);
    phx_store::hash_saved(closed, h);
}

/// The books store's content: the directory and the books.
pub(crate) fn hash_books(h: &mut LogicalHasher, books: &phx_ledger::books::Books) {
    books.parties.directory().hash_into(h);
    books.hash_into(h);
}

pub(crate) fn hash_records(h: &mut LogicalHasher, records: &RecordStore) {
    records.hash_into(h);
}

pub(crate) fn hash_events(h: &mut LogicalHasher, events: &EventStore) {
    events.hash_into(h);
}

/// The map store's content: the map as generated and the kernel tables' columns.
pub(crate) fn hash_geo(h: &mut LogicalHasher, geo: &phx_geo::GeoState, tables: &[KernelTable]) {
    phx_store::hash_saved(geo, h);
    for t in tables {
        h.bytes(t.name.as_bytes());
        t.columns.hash_into(h);
    }
}

/// The world's hash over its logical content, store by store in the order a save writes them: the day and what it
/// carries over, the directory and books, the markets, the accounts, the records, the events, and the map with the
/// kernel's tables. Metrics, findings, layout and derived indexes are outside it.
#[clause("SET.15")]
#[must_use]
pub fn world_hash(world: &World) -> u128 {
    let mut h = LogicalHasher::new(HASH_KEY);
    hash_world_store(&mut h, world.today, &world.unprocessed, &world.queue, &world.bindings, &world.closed);
    hash_books(&mut h, &world.books);
    world.markets.hash_into(&mut h);
    world.accounts.hash_into(&mut h);
    hash_records(&mut h, &world.records);
    hash_events(&mut h, &world.events);
    hash_geo(&mut h, world.geo(), &world.tables);
    h.finish()
}
