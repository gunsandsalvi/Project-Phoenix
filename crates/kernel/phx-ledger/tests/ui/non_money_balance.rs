use phx_ledger::line::{InUnit, LineKind};
use phx_ledger::rows::RowView;
use phx_num::{Ccy, Missing, Money};

fn as_money(kind: &LineKind<InUnit>, row: &RowView) -> Missing<Money> {
    kind.balance(row, Ccy::new(0))
}

fn main() {}
