use phx_id::{HoldingRef, OfferRef};

fn offer(_: OfferRef) {}

fn main() {
    offer(HoldingRef::from_word(1));
}
