//! The families' codes: one row a family, contract or dated reason, each holding a fixed 8-bit code a save names, so a
//! code is a permanent identifier, kept when its family retires and never reissued.

use serde::Deserialize;

use crate::consts::HOLDINGS_CODE;

/// Whether a row's family is a family of contracts or of dated reasons the wheel files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyKind {
    Contract,
    DatedReason,
}

/// Whether a row's family is declared by the world today, planned by a later step, or retired with its code kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyStatus {
    Declared,
    Planned,
    Retired,
}

/// A family's row: its code, name, kind, the step that settles it, and its status.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyCode {
    pub code: u8,
    pub name: String,
    pub kind: FamilyKind,
    pub step: String,
    pub status: FamilyStatus,
}

/// Every family's row, by code.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FamilyCodes {
    rows: Vec<FamilyCode>,
}

impl FamilyCodes {
    /// The rows, refusing a code twice — a retired family's code reissued among them — a name twice, and holdings'
    /// code.
    ///
    /// # Errors
    /// Every refusal, listed.
    pub fn new(mut rows: Vec<FamilyCode>) -> Result<FamilyCodes, Vec<String>> {
        let mut refused = Vec::new();
        rows.sort_by_key(|r| r.code);
        for pair in rows.windows(2) {
            if let [a, b] = pair
                && a.code == b.code
            {
                let both = [a, b];
                match both.iter().find(|r| r.status == FamilyStatus::Retired) {
                    Some(r) => refused.push(format!("family code {} of retired `{}` reissued", a.code, r.name)),
                    None => refused.push(format!("family code {} held by `{}` and `{}`", a.code, a.name, b.name)),
                }
            }
        }
        for (i, r) in rows.iter().enumerate() {
            if rows.iter().take(i).any(|s| s.name == r.name) {
                refused.push(format!("family `{}` given two rows", r.name));
            }
            if r.code == HOLDINGS_CODE {
                refused.push(format!("family `{}` at code {}, holdings'", r.name, r.code));
            }
        }
        if refused.is_empty() { Ok(FamilyCodes { rows }) } else { Err(refused) }
    }

    /// The rows in code order.
    #[must_use]
    pub fn rows(&self) -> &[FamilyCode] {
        &self.rows
    }
}
