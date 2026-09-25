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
/// implement it with each row's weight, one but for an agent's estate; the population's agent tables with each
/// agent's multiplicity.
#[clause("REP.9", "REP.16")]
pub trait PayerPositions {
    fn weight(&self, holder: Slot) -> u32;
    /// A member's funds in an account: the row's balance less what is pending, with the facility each member has,
    /// shared over the row's members.
    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128;
    /// The holder's funds in an account for every member its row holds: the balance less what is pending, with each
    /// member's facility.
    fn funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128;
    /// The kinks a flow may carry the holder's members across, added to `buf`.
    fn kinks_into(&self, holder: Slot, buf: &mut Vec<Kink>);
    /// The standing rate a row carries per member, where its kind has one.
    fn standing_rate(&self, holder: Slot, row: &RowView) -> Missing<i64>;
}

/// A holder's account: its balance less what is pending, and the members its row holds.
pub fn account_funds(arenas: &dyn crate::holder::HolderArenas, holder: Slot, account: LineId) -> (i128, i128) {
    let Some(view) = rows::find(arenas, holder, account, Side::Asset) else {
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
    if members == 0 {
        violation!(clause = "REP.9", "an account held by no member", line = account.get());
    }
    (i128::from(balance) - pending, members)
}

impl<B: Backing> PayerPositions for KindTable<B> {
    fn weight(&self, holder: Slot) -> u32 {
        self.weight_at(holder)
    }

    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let (available, members) = account_funds(self, holder, account);
        available / members + i128::from(facility_per_member)
    }

    fn funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let (available, members) = account_funds(self, holder, account);
        available + i128::from(facility_per_member) * members
    }

    fn kinks_into(&self, _: Slot, _: &mut Vec<Kink>) {}

    fn standing_rate(&self, _: Slot, _: &RowView) -> Missing<i64> {
        Missing::Absent
    }
}
