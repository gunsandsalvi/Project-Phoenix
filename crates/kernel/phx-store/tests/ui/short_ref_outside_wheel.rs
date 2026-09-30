use phx_store::{AddressSpace, Column, ShortRef};

struct Contracts;

fn main() {
    let mut space = AddressSpace::empty();
    let _outside: Column<ShortRef<Contracts>> = Column::new(&mut space, 16, 16);
}
