use phx_id::{LineId, PartyId};

fn main() {
    let _ = LineId::from(PartyId::new(1));
}
