use phx_acct::valuer::Valuation;
use phx_market::print::Print;

// What the tape keeps and the indices read.
fn onto_the_tape(_: Print) {}

fn publish(valued: Valuation) {
    onto_the_tape(Print::from(valued));
}

fn main() {}
