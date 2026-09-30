use phx_macros::Saved;

#[derive(Saved)]
struct Kept {
    #[saved(keep)]
    index: Vec<u32>,
}

fn main() {}
