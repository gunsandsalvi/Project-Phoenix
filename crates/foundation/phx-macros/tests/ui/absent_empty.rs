use phx_macros::absent_is_zero;

#[absent_is_zero(reason = " ")]
fn sold(of: Option<u32>) -> u32 {
    of.unwrap_or(0)
}

fn main() {}
