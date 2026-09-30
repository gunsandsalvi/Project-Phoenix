use phx_store::GenRef;

struct Firms;
struct Households;

fn firm(_: GenRef<Firms>) {}

fn main() {
    let household: Option<GenRef<Households>> = None;
    if let Some(h) = household {
        firm(h);
    }
    firm(phx_id::Slot::new(0));
}
