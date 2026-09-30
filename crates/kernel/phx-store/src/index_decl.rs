//! A keyed index's declaration: the data a system names an instance by, read at assembly.

use phx_macros::opening;
use phx_num::Missing;

/// How an instance keeps a key's members: a lazy list, a lazy list with its members counted, a sum-tree of weights,
/// or a count alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Lazy,
    LazyCounted,
    Weighted,
    Count,
}

impl Mode {
    #[must_use]
    pub fn lists(self) -> bool {
        matches!(self, Mode::Lazy | Mode::LazyCounted)
    }

    #[must_use]
    pub fn counts(self) -> bool {
        matches!(self, Mode::LazyCounted | Mode::Count)
    }

    #[must_use]
    pub fn weighs(self) -> bool {
        matches!(self, Mode::Weighted)
    }
}

/// An instance's key space, below which every key lies: dense where its keys hold many members each (zones,
/// (zone, kind)), a list a key found by the key's place; sparse where they hold few among many keys (a party among
/// millions), its entries held as (key, member) pairs found by binary search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keys {
    Dense(u32),
    Sparse(u32),
}

impl Keys {
    #[must_use]
    pub fn count(self) -> u32 {
        match self {
            Keys::Dense(n) | Keys::Sparse(n) => n,
        }
    }
}

/// An instance: its name, the table its members are rows of, the declared columns its key and its membership read,
/// its mode, its key space, the most entries it holds at once, and the clause it serves. A walk reads a member as one
/// exactly when the member's own columns put it at the key and satisfy the predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexDecl {
    pub name: &'static str,
    pub member_table: &'static str,
    pub key: &'static str,
    pub predicate: Missing<&'static str>,
    pub mode: Mode,
    pub keys: Keys,
    pub entries: u32,
    pub clause: &'static str,
}

impl IndexDecl {
    /// # Errors
    /// An instance with no name, no member table, no key column, no key space or no room for an entry.
    #[opening]
    pub fn check(&self) -> Result<(), String> {
        let missing = [
            (self.name.is_empty(), "a name"),
            (self.member_table.is_empty(), "a member table"),
            (self.key.is_empty(), "a key column"),
            (self.keys.count() == 0, "a key space"),
            (self.entries == 0 && !matches!(self.mode, Mode::Count), "room for an entry"),
            (
                matches!(self.keys, Keys::Sparse(_)) && !matches!(self.mode, Mode::Lazy),
                "dense keys for its weights or counts",
            ),
        ];
        match missing.iter().find(|(lacks, _)| *lacks) {
            Some((_, what)) => Err(format!("the index `{}` declares no {what}", self.name)),
            None => Ok(()),
        }
    }
}
