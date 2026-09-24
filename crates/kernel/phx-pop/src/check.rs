use phx_id::{Day, LineId, Slot};
use phx_ledger::algebra::Side;
use phx_ledger::apply::Ledger;
use phx_ledger::rows;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::key::{KeyId, KeyInterner};
use crate::kind::PopKindDecl;
use crate::part::Part;
use crate::steps::Step;
use crate::table::{Attention, CellTable, steps_of};

/// Why a part may not join a cell: another key, signature or step vector, rates, attention or review exposure that
/// could not add, or a row of a line both hold whose record differs or whose members lie across one of the line's
/// kinks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    Key,
    Signature,
    Steps,
    Rates,
    Attention,
    Exposure,
    Record(LineId),
    Kink(LineId),
}

/// The points of a line where one of its terms or constraints changes, on a row's balance per member: a credit limit,
/// the insured limit. The lines' owners declare them; the check reads them.
pub trait LineKinks {
    fn points(&self, line: LineId, side: Side) -> Vec<i64>;
}

/// A row as the check reads it: its line side, its members, their payment record and price point, its balance and
/// since when it has been in arrears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowFacts {
    pub line: LineId,
    pub side: Side,
    pub count: u32,
    pub record: u32,
    pub point: u16,
    pub balance: Missing<i64>,
    pub since: Missing<Day>,
}

/// What the check compares of a cell or a part, each read from its own totals and scales.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub key: Missing<KeyId>,
    pub sig: Vec<u64>,
    pub steps: Vec<Step>,
    pub rates: Vec<Missing<i64>>,
    pub attention: Vec<Missing<Attention>>,
    pub exposed: Vec<bool>,
    pub rows: Vec<RowFacts>,
}

impl View {
    /// A cell as it stands, its rows read from its arena in their order.
    #[must_use]
    pub fn of_cell<B: Backing, L: Backing>(
        table: &CellTable<B>,
        slot: Slot,
        ledger: &Ledger<L>,
        kind: &PopKindDecl,
        levels: &[u8],
    ) -> View {
        let party = table.party(slot);
        let rows = rows::iter(table, slot)
            .map(|r| RowFacts {
                line: r.row.line,
                side: r.side(),
                count: r.row.count,
                record: r.row.record,
                point: r.row.point,
                balance: r.optional.balance,
                since: match ledger.arrears().of(r.row.line, r.side(), party) {
                    Some(d) => Missing::Present(d),
                    None => Missing::Absent,
                },
            })
            .collect();
        View {
            key: Missing::Present(table.hot(slot).key_id),
            sig: table.sig(slot),
            steps: table.steps(slot, kind, levels),
            rates: (0..table.rate_kinds()).map(|r| table.rate(slot, r)).collect(),
            attention: (0..table.review_kinds()).map(|j| table.attention(slot, j)).collect(),
            exposed: (0..table.review_kinds()).map(|j| table.exposure(slot, j) != Missing::Absent).collect(),
            rows,
        }
    }

    /// A part as it travels, its key found among the keys cells hold, if any holds it.
    #[must_use]
    pub fn of_part(part: &Part, keys: &KeyInterner, kind: &PopKindDecl, levels: &[u8]) -> View {
        let rows = part
            .rows
            .iter()
            .map(|d| RowFacts {
                line: d.line(),
                side: d.side(),
                count: d.row.count,
                record: d.row.record,
                point: d.row.point,
                balance: d.optional.balance,
                since: d.arrears_since,
            })
            .collect();
        View {
            key: keys.id(&part.key),
            sig: part.sig.clone(),
            steps: steps_of(kind, levels, part.weight, &part.positions, &part.rates),
            rates: part.rates.clone(),
            attention: part.attention.clone(),
            exposed: part.exposures.iter().map(|e| *e != Missing::Absent).collect(),
            rows,
        }
    }
}

/// Which side of a point a row's members lie on: at or above it per member, compared exactly over the row's count.
fn at_or_above(balance: i64, count: u32, point: i64) -> bool {
    i128::from(balance) >= i128::from(point) * i128::from(count)
}

/// The landing check: the same key identity, signature and full step vector, compared exactly, since a landing key
/// only proposes a candidate; the same rates and attention, which a landing never averages, and review exposures
/// kept on both or neither; then, for every line side both hold, the same payment record, arrears and price point, and
/// members on the same side of every point of the line's kinks. It draws nothing and changes nothing.
///
/// # Errors
/// The first difference that keeps the part from the cell.
#[clause("REP.8", "REP.16", "REP.36")]
pub fn check(part: &View, cell: &View, kinks: &dyn LineKinks) -> Result<(), Refused> {
    if part.key == Missing::Absent || part.key != cell.key {
        return Err(Refused::Key);
    }
    if part.sig != cell.sig {
        return Err(Refused::Signature);
    }
    if part.steps != cell.steps {
        return Err(Refused::Steps);
    }
    if part.rates != cell.rates {
        return Err(Refused::Rates);
    }
    if part.attention != cell.attention {
        return Err(Refused::Attention);
    }
    if part.exposed != cell.exposed {
        return Err(Refused::Exposure);
    }
    for mine in &part.rows {
        let Some(theirs) = cell.rows.iter().find(|r| (r.line, r.side) == (mine.line, mine.side)) else {
            continue;
        };
        if (mine.record, mine.point, mine.since) != (theirs.record, theirs.point, theirs.since) {
            return Err(Refused::Record(mine.line));
        }
        if let (Missing::Present(a), Missing::Present(b)) = (mine.balance, theirs.balance) {
            for p in kinks.points(mine.line, mine.side) {
                if at_or_above(a, mine.count, p) != at_or_above(b, theirs.count, p) {
                    return Err(Refused::Kink(mine.line));
                }
            }
        }
    }
    Ok(())
}
