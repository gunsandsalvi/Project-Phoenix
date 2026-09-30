use phx_macros::Maintained;

#[derive(Maintained)]
struct Books {
    #[maintained]
    sold: u64,
}

fn main() {}
