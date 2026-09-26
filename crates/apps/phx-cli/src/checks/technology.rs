//! Technology: every unit made came by a way its maker knew, from the inputs the way states.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

fn not_yet(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("firms produce from their plant once they decide what to make (S1.03)")
}

pub const LC_1_05: super::Check = live_check! {
    id: "LC-1-05",
    title: "every production names a way its producer knew, with the inputs it consumed as the way states",
    from_step: "S1.02",
    check: not_yet,
};
