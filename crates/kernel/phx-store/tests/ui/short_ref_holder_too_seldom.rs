use phx_store::{AddressSpace, Recheck, ShortRefs};

struct Contracts;

struct Yearly;

impl Recheck for Yearly {
    const DAYS: u8 = 365;
}

fn main() {
    let mut space = AddressSpace::empty();
    let _seldom: ShortRefs<Contracts, Yearly> = ShortRefs::new(&mut space, 16, 16);
}
