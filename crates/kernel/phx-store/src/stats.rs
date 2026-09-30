//! What a store reports of itself to the counters: its rows live now and ever, and its bytes. The rows a walk visits
//! are counted by the traversal that walks it, which alone may share a counter across workers.

/// A store's own statistics, read by the counters, never by the world.
pub trait StoreStats {
    fn rows_live(&self) -> u64;
    fn rows_ever(&self) -> u64;
    fn bytes(&self) -> u64;
}
