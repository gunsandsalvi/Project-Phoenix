use phx_macros::sweep;

#[sweep(store = accounts, cycle = 30, reason = "a bank failing")]
fn recount() {}

fn main() {}
