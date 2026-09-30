use phx_core::{ObserverDraws, WorldStreams};

/// A world rule draws only from the world's streams.
fn rule(_: &WorldStreams) {}

fn hand(observer: &ObserverDraws<'_>) {
    rule(observer);
}

fn main() {}
