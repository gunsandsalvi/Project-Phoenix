use phx_macros::Maintained;

fn record() {}

#[derive(Maintained)]
struct Books {
    #[maintained(writer = record)]
    share: f64,
}

fn main() {}
