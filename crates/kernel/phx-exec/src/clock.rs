/// Wall time, which the applications provide; it reaches counters and the player, never the world.
pub trait Clock: Sync {
    fn now_ns(&self) -> u64;
}
