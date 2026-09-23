#[derive(Clone, Copy)]
struct Loose(u64);

#[repr(C)]
#[derive(Clone, Copy, phx_macros::Pod)]
struct Holder {
    inner: Loose,
}

fn main() {}
