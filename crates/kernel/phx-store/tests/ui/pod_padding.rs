#[repr(C)]
#[derive(Clone, Copy, phx_macros::Pod)]
struct Padded {
    small: u8,
    wide: u32,
}

fn main() {}
