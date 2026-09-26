//! A carrier's side of freight: the mode its vehicles run on.

use phx_core::declare_fact;

declare_fact! {
    /// The mode a carrier's vehicles run on, by the network's modes' places.
    pub Mode = "FRT.mode" {
        value: Count, kinds: ["firm", "small_firm"], writer: "FRT", audience: Public, repr: Position, clause: "FRT.1",
    }
}
