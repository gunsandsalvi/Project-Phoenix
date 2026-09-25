use phx_market::print::Print;
use phx_val::value::Value;

// What the tape keeps and the indices read.
fn onto_the_tape(_: Print) {}

fn publish(valued: Value) {
    onto_the_tape(Print::from(valued));
}

fn main() {}
