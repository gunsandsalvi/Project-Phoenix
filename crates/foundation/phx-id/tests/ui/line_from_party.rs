use phx_id::{LineId, PartyRef, Slot};

fn main() {
    let _ = LineId::from(PartyRef::new(0, 0, Slot::new(1)));
}
