use phx_core::kind_tables::KindTable;
use phx_id::{LineId, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::rows::{self, RowView};

/// What the settlement stream reads of a payer, as its table keeps it: what it may draw on in an account, and the
/// standing rate a row carries.
#[clause("REP.9", "REP.16")]
pub trait PayerPositions {
    /// The holder's funds in an account: the balance less what is pending, with its facility.
    fn funds(&self, holder: Slot, account: LineId, facility: i64) -> i128;
    /// The standing rate a row carries, where its kind has one.
    fn standing_rate(&self, holder: Slot, row: &RowView) -> Missing<i64>;
}

/// A holder's account: its balance less what is pending.
pub fn account_funds(arenas: &dyn crate::holder::HolderArenas, holder: Slot, account: LineId) -> i128 {
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
    i128::from(balance) - pending
}

impl<B: Backing> PayerPositions for KindTable<B> {
    fn funds(&self, holder: Slot, account: LineId, facility: i64) -> i128 {
        account_funds(self, holder, account) + i128::from(facility)
    }

    fn standing_rate(&self, _: Slot, _: &RowView) -> Missing<i64> {
        Missing::Absent
    }
}
