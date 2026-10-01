use phx_id::{LineId, PartyRef, Slot};

fn main() {
    let _ = PartyRef::new(0, 0, Slot::new(1)) == LineId::new(1);
}
