use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

fn no_books(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("the world keeps its books from the opening's institutions and their instruments (S0.16)")
}

pub const LC_0_16: Check = live_check! {
    id: "LC-0-16",
    title: "For every issued instrument, holdings sum to the issued amount",
    from_step: "S0.14",
    check: no_books,
};

pub const LC_0_17: Check = live_check! {
    id: "LC-0-17",
    title: "Every line's two sides hold equal counts, and each side it lists equals its holders' rows",
    from_step: "S0.14",
    check: no_books,
};
