use phx_core::kind_tables::KindTable;
use phx_id::{LineId, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::pooled::Kink;
use crate::rows::{self, RowView};

/// What the pooled-flow rule reads of a payer, as its table keeps it: how many members its row stands for, what a
/// member may draw on in an account, the kinks on its positions, and the standing rate a row carries. The kind tables
/// implement it with a weight of one; the population's cells implement it for their members.
#[clause("REP.8", "REP.16")]
pub trait PayerPositions {
    fn weight(&self, holder: Slot) -> u32;
    /// A member's funds in an account: the row's balance less what is pending, with the facility each member has,
    /// shared over the row's members.
    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128;
    /// The kinks a flow may carry the holder's members across, added to `buf`.
    fn kinks_into(&self, holder: Slot, buf: &mut Vec<Kink>);
    /// The standing rate a row carries per member, where its kind has one.
    fn standing_rate(&self, holder: Slot, row: &RowView) -> Missing<i64>;
}

impl<B: Backing> PayerPositions for KindTable<B> {
    fn weight(&self, _: Slot) -> u32 {
        1
    }

    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let Some(view) = rows::iter(self, holder).find(|r| r.row.line == account && r.side() == Side::Asset) else {
            violation!(clause = "MON.5", "funds read on an account its holder does not hold", line = account.get());
        };
        let Missing::Present(balance) = view.optional.balance else {
            violation!(clause = "MON.5", "an account with no balance", line = account.get());
        };
        let pending = match view.optional.pending {
            Missing::Present(p) => i128::from(p),
            Missing::Absent => 0,
        };
        let members = i128::from(view.row.count);
        (i128::from(balance) - pending) / members + i128::from(facility_per_member)
    }

    fn kinks_into(&self, _: Slot, _: &mut Vec<Kink>) {}

    fn standing_rate(&self, _: Slot, _: &RowView) -> Missing<i64> {
        Missing::Absent
    }
}
