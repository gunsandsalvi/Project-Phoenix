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

pub const LC_0_18: Check = live_check! {
    id: "LC-0-18",
    title: "Per issuer and currency, the balances held equal its money liability; the notes held equal the notes issued",
    from_step: "S0.15",
    check: no_books,
};

pub const LC_0_19: Check = live_check! {
    id: "LC-0-19",
    title: "Every instruction's paired legs sum to nothing per denomination; transformations and opening writes name their sources",
    from_step: "S0.15",
    check: no_books,
};

pub const LC_0_20: Check = live_check! {
    id: "LC-0-20",
    title: "Per holder and asset per day, what it held plus what came in less what went out is what it holds",
    from_step: "S0.15",
    check: no_books,
};

pub const LC_0_21: Check = live_check! {
    id: "LC-0-21",
    title: "Every fail has a cause, and its line kind's contract process recorded it at 2d",
    from_step: "S0.15",
    check: no_books,
};

pub const LC_0_22: Check = live_check! {
    id: "LC-0-22",
    title: "Gross and net settlement and fails by cause are published every day",
    from_step: "S0.15",
    check: no_books,
};
