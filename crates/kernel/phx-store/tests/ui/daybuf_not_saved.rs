use phx_store::{DayBuf, Saved};

fn saved<T: Saved>() {}

fn main() {
    saved::<DayBuf<u64>>();
}
