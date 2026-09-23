use phx_id::{LineId, PartyId};

fn main() {
    let _ = PartyId::new(1) == LineId::new(1);
}
