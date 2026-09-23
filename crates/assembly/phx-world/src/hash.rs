use phx_macros::clause;
use phx_store::LogicalHasher;

use crate::consts::HASH_KEY;
use crate::world::World;

/// The world's hash over its logical content: its day, its identities and its records and events. Metrics, findings
/// and derived indexes are outside it.
#[clause("SET.15")]
#[must_use]
pub fn world_hash(world: &World) -> u128 {
    let mut h = LogicalHasher::new(HASH_KEY);
    h.u64(u64::from(world.today.get()));
    world.directory.hash_into(&mut h);
    world.records.hash_into(&mut h);
    world.events.hash_into(&mut h);
    h.finish()
}
