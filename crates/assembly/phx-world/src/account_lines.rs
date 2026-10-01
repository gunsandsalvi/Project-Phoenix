//! The books a party with owners keeps on its kind's store: its income-statement lines since the accounts opened, each
//! an i64 word of its row, and its equity at the opening, from which its equity account is its equity and the income
//! its lines make. A line past an i64 stops the run.

use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_pop::kinds::{Attr, AttrW, KindStore};
use phx_pop::layout::Layout;
use phx_store::SystemBacking;

use crate::consts::LINES;

/// A line of income an event moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Line {
    Revenue,
    CostOfSales,
    GoodsLost,
    ServicesUsed,
    Wages,
    Taxes,
    InterestPaid,
    InterestReceived,
    WrittenOff,
    Depreciation,
}

impl Line {
    /// Every line, in the order a party's words hold them.
    pub const ALL: [Line; LINES] = [
        Line::Revenue,
        Line::CostOfSales,
        Line::GoodsLost,
        Line::ServicesUsed,
        Line::Wages,
        Line::Taxes,
        Line::InterestPaid,
        Line::InterestReceived,
        Line::WrittenOff,
        Line::Depreciation,
    ];

    /// The line's place among a party's words.
    #[must_use]
    pub fn at(self) -> usize {
        match Line::ALL.iter().position(|l| *l == self) {
            Some(i) => i,
            None => violation!(clause = "ACC.13", "a line its parties hold no word for"),
        }
    }

    /// Whether the line adds to income; every other takes from it.
    #[must_use]
    pub fn is_income(self) -> bool {
        matches!(self, Line::Revenue | Line::InterestReceived)
    }
}

/// A party's lines as read, by their order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines(pub [i64; LINES]);

impl Lines {
    #[must_use]
    pub fn get(&self, line: Line) -> i64 {
        self.0.get(line.at()).copied().unwrap_or_else(|| violation!(clause = "ACC.13", "a line past the party's"))
    }

    /// The income the lines make.
    #[clause("ACC.13")]
    #[must_use]
    pub fn net(&self) -> i128 {
        Line::ALL.iter().map(|l| if l.is_income() { i128::from(self.get(*l)) } else { -i128::from(self.get(*l)) }).sum()
    }
}

/// A line's word moved by an amount; past an i64 stops the run.
#[clause("ACC.13")]
#[must_use]
pub fn added(held: i64, amount: i64) -> i64 {
    match held.checked_add(amount) {
        Some(v) => v,
        None => capacity_exceeded!("an income-statement line", i64::MAX, i128::from(held) + i128::from(amount)),
    }
}

/// The write handles of the words a kind keeps its books in.
#[derive(Clone, Copy, Debug)]
pub struct BookWords {
    lines: [AttrW<i64>; LINES],
    equity: AttrW<i64>,
}

impl BookWords {
    /// The handles of a layout's book words, `income_lines` and `equity`, handed to the books once.
    #[must_use]
    pub fn bind(layout: &mut Layout) -> BookWords {
        let mut handle = |name: &str, i: u16, base: &str| match layout.writer::<i64>(name, i, base) {
            Ok(a) => a,
            Err(_) => violation!(clause = "REP.1", "a kind's book word refused"),
        };
        let lines = core::array::from_fn(|i| match u16::try_from(i) {
            Ok(i) => handle("income_lines", i, "K-87"),
            Err(_) => violation!(clause = "REP.1", "a line past a word's count"),
        });
        BookWords { lines, equity: handle("equity", 0, "K-88") }
    }

    fn line(&self, line: Line) -> AttrW<i64> {
        match self.lines.get(line.at()) {
            Some(a) => *a,
            None => violation!(clause = "ACC.13", "a line past the kind's words"),
        }
    }

    /// An amount entered on a party's line.
    pub fn add(&self, store: &mut KindStore<SystemBacking>, slot: phx_id::Slot, line: Line, amount: i64) {
        let a = self.line(line);
        let Missing::Present(held) = read_at(store, slot, a.read()) else {
            violation!(clause = "ACC.13", "a line read for no party", slot = slot.get());
        };
        store.set_at(slot, a, Missing::Present(added(held, amount)));
    }

    /// A party's equity at the opening written.
    pub fn open(&self, store: &mut KindStore<SystemBacking>, slot: phx_id::Slot, equity: i64) {
        store.set_at(slot, self.equity, Missing::Present(equity));
    }

    /// A party's lines, read from its row gathered once: a kind's lines are one word's values, in one group.
    #[must_use]
    pub fn lines(&self, store: &KindStore<SystemBacking>, slot: phx_id::Slot) -> Option<Lines> {
        let (group, _) = self.line(Line::Revenue).read().place();
        let row = store.gather_at(slot, group)?;
        let mut out = Lines::default();
        for (v, a) in out.0.iter_mut().zip(&self.lines) {
            *v = match row.get(a.read()) {
                Missing::Present(x) => x,
                Missing::Absent => return None,
            };
        }
        Some(out)
    }

    /// A party's equity account: its equity at the opening and the income its lines make.
    #[must_use]
    pub fn equity(&self, store: &KindStore<SystemBacking>, slot: phx_id::Slot) -> Option<i128> {
        let Missing::Present(opening) = read_at(store, slot, self.equity.read()) else { return None };
        Some(i128::from(opening) + self.lines(store, slot)?.net())
    }
}

/// A word of the party at a slot, read from its group's row.
fn read_at(store: &KindStore<SystemBacking>, slot: phx_id::Slot, a: Attr<i64>) -> Missing<i64> {
    let (group, _) = a.place();
    match store.gather_at(slot, group) {
        Some(row) => row.get(a),
        None => Missing::Absent,
    }
}

#[cfg(test)]
#[path = "account_lines_tests.rs"]
mod tests;
