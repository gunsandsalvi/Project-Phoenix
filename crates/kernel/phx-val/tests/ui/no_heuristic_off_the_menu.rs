use phx_val::heuristic::{Heuristic, Params, Seen};

// A forecast handed in from outside the menu.
#[derive(Debug)]
struct Oracle;

impl Heuristic for Oracle {
    fn name(&self) -> &'static str {
        "oracle"
    }
    fn outlook(&self, seen: &Seen, _: &Params) -> f64 {
        seen.last
    }
}

fn main() {}
