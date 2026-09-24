use phx_core::kind_tables::ListKind;
use phx_id::{LineId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Missing, violation};

use crate::algebra::Side;
use crate::holder::HolderArenas;
use crate::words::{from_words, to_words, words_of};

/// A holder's relationship to a line, 16 bytes with no padding, followed in the holder's arena by the optional words
/// its flags name: `line`, the members sharing it (`count`), the payment record (days in arrears and missed
/// payments, sixteen bits each), the line's price point, the role (side and role within the party) and the flags.
#[clause("REP.3", "REG.8")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct RelRow {
    pub line: LineId,
    pub count: u32,
    pub record: u32,
    pub point: u16,
    pub role: u8,
    pub flags: u8,
}

/// The row carries a running balance word.
pub const BALANCE: u8 = 1;
/// The row carries a word of amounts awaiting settlement.
pub const PENDING: u8 = 1 << 1;
/// The row carries its amount, for a line kind with no point table.
pub const AMOUNT: u8 = 1 << 2;

const ROW: usize = words_of::<RelRow>();

/// The optional words of a row, each present only when its line kind declares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Optional {
    pub balance: Missing<i64>,
    pub pending: Missing<i64>,
    pub amount: Missing<i64>,
}

impl Optional {
    pub const NONE: Optional = Optional { balance: Missing::Absent, pending: Missing::Absent, amount: Missing::Absent };

    pub(crate) fn flags(self) -> u8 {
        let has = |m: Missing<i64>, f: u8| if matches!(m, Missing::Present(_)) { f } else { 0 };
        has(self.balance, BALANCE) | has(self.pending, PENDING) | has(self.amount, AMOUNT)
    }

    fn words(self) -> Vec<u64> {
        [self.balance, self.pending, self.amount]
            .into_iter()
            .filter_map(|m| match m {
                Missing::Present(v) => Some(v.cast_unsigned()),
                Missing::Absent => None,
            })
            .collect()
    }
}

/// A row's role: its side, and its role within the party.
#[must_use]
pub fn role(side: Side, within: u8) -> u8 {
    let s = match side {
        Side::Asset => 0,
        Side::Liability => 1,
    };
    (within << 1) | s
}

/// The side a role is on.
#[must_use]
pub fn side_of(role: u8) -> Side {
    if role & 1 == 0 { Side::Asset } else { Side::Liability }
}

/// A row as read from its holder's arena: the row, its optional words, and where in the holder's run it lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowView {
    pub row: RelRow,
    pub optional: Optional,
    pub at: usize,
}

impl RowView {
    #[must_use]
    pub fn side(&self) -> Side {
        side_of(self.row.role)
    }

    /// Days in arrears: the record's low half.
    #[must_use]
    pub fn arrears_days(&self) -> u16 {
        let [a, b, _, _] = self.row.record.to_le_bytes();
        u16::from_le_bytes([a, b])
    }

    /// Payments missed: the record's high half.
    #[must_use]
    pub fn missed(&self) -> u16 {
        let [_, _, c, d] = self.row.record.to_le_bytes();
        u16::from_le_bytes([c, d])
    }
}

fn width(flags: u8) -> usize {
    let optional = (flags & (BALANCE | PENDING | AMOUNT)).count_ones();
    let Ok(n) = usize::try_from(optional) else {
        violation!(clause = "REG.14", "a row's optional words beyond a count", flags = flags);
    };
    ROW + n
}

/// A holder's rows, in its arena's order, each read with the optional words its flags name.
#[must_use]
pub fn rows(arenas: &dyn HolderArenas, holder: Slot) -> Vec<RowView> {
    let words = arenas.read(holder, ListKind::RelationshipRows);
    let mut out = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let Some(head) = words.get(at..at + ROW) else {
            violation!(clause = "REG.14", "a holder's rows end inside a row", at = at);
        };
        let row: RelRow = from_words(head);
        let mut rest = words.get(at + ROW..at + width(row.flags)).map_or_else(Vec::new, <[u64]>::to_vec).into_iter();
        let mut next = |flag: u8| {
            if row.flags & flag == 0 {
                Missing::Absent
            } else {
                let Some(w) = rest.next() else {
                    violation!(clause = "REG.14", "a row's flagged word missing from its holder's run", at = at);
                };
                Missing::Present(w.cast_signed())
            }
        };
        let optional = Optional { balance: next(BALANCE), pending: next(PENDING), amount: next(AMOUNT) };
        out.push(RowView { row, optional, at });
        at += width(row.flags);
    }
    out
}

/// A row appended to its holder's run.
pub(crate) fn append(arenas: &mut dyn HolderArenas, holder: Slot, mut row: RelRow, optional: Optional) {
    row.flags = (row.flags & !(BALANCE | PENDING | AMOUNT)) | optional.flags();
    let mut words = to_words(&row);
    words.extend(optional.words());
    arenas.append(holder, ListKind::RelationshipRows, &words);
}

/// A row written back where it lies; its optional words keep their places.
pub(crate) fn rewrite(arenas: &mut dyn HolderArenas, holder: Slot, view: &RowView, optional: Optional) {
    if optional.flags() != view.row.flags & (BALANCE | PENDING | AMOUNT) {
        violation!(clause = "REG.14", "a row's optional words changed in place", line = view.row.line.get());
    }
    let mut words = to_words(&view.row);
    words.extend(optional.words());
    arenas.overwrite(holder, ListKind::RelationshipRows, view.at, &words);
}

/// A row removed from its holder's run.
pub(crate) fn remove(arenas: &mut dyn HolderArenas, holder: Slot, view: &RowView) {
    arenas.remove(holder, ListKind::RelationshipRows, view.at, width(view.row.flags));
}
