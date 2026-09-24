//! The representation's own settings, shared by every population kind, and the streams it draws from.

use phx_core::{Declarations, Prim, StreamDef, declare_prim, declare_stream};
use phx_num::{Count, Fixed};

declare_prim! {
    /// The cells the decision gap's estimate samples in a kind each time tolerance control reads it.
    pub GAP_SAMPLE = "REP.gap_sample" { kind: Resolution, value: Count, clause: "REP.39", scope: Shared }
}

declare_prim! {
    /// The share of a kind's cell budget below which its tolerances are narrowed on light days.
    pub NARROW_SHARE = "REP.narrow_share" { kind: Resolution, value: Fixed { exp: 2 }, clause: "REP.28", scope: Shared }
}

declare_prim! {
    /// The day of each month on which every kind's parties are ranked.
    pub RANK_DAY = "REP.rank_day" { kind: Resolution, value: Count, clause: "REP.29", scope: Shared }
}

declare_stream! { pub ToleranceStream = "REP.tolerance" { purpose: Sample, keyed: false, clause: "REP.39" } }
declare_stream! { pub PromotionStream = "REP.promotion" { purpose: Lot, keyed: false, clause: "REP.29" } }
declare_stream! { pub HouseholdsStream = "REP.households" { purpose: Sample, keyed: false, clause: "REP.26" } }

/// The representation's primitives as the world reads them.
#[derive(Debug)]
pub struct RepPrims {
    pub gap_sample: Prim<Count>,
    pub narrow_share: Prim<Fixed<2>>,
    pub rank_day: Prim<Count>,
}

impl RepPrims {
    pub fn declare(d: &mut Declarations) -> RepPrims {
        d.stream(ToleranceStream::DECL);
        d.stream(PromotionStream::DECL);
        d.stream(HouseholdsStream::DECL);
        RepPrims { gap_sample: d.prim(&GAP_SAMPLE), narrow_share: d.prim(&NARROW_SHARE), rank_day: d.prim(&RANK_DAY) }
    }
}
