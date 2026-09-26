use if_base::Way;

// A recipe as a share of the output's value: a way has no such field, so its draws never move with prices.
fn value_share(way: &Way) -> i64 {
    way.value_shares[0].1.raw()
}

fn main() {}
