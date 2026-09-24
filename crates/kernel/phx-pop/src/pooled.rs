use phx_core::KinkOn;
use phx_core::KinkRegistry;
use phx_id::LineId;
use phx_ledger::algebra::Side;
use phx_ledger::part::RowShare;
use phx_ledger::split_request::{SplitCause, SplitRequest};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};

use crate::kind::PopKindDecl;
use crate::split::SplitSpec;

/// The split a pooled flow's request makes of its holder: the members the flow reached leave its row, and a flow paid
/// for them leaves with them whole what it moved — its amount on their funds row and its move of the position its kink
/// lies on — the rest of every total shared as a split shares it. A failed flow moved nothing.
#[clause("REP.8", "REP.23")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PooledSplit {
    pub count: u32,
    pub rows: Vec<((LineId, Side), RowShare)>,
    pub own: Vec<(usize, i64)>,
}

impl PooledSplit {
    /// The request as a split: the reached members' row and funds given, their other groups and rows drawn.
    #[must_use]
    pub fn spec(&self, rounding: Round) -> SplitSpec<'_> {
        SplitSpec {
            count: self.count,
            given: &[],
            rows: &self.rows,
            own: &self.own,
            reviewed: Missing::Absent,
            rounding,
        }
    }
}

fn times(per_member: i64, count: u32) -> i64 {
    let Some(t) = per_member.checked_mul(i64::from(count)) else {
        violation!(clause = "Law 7", "a flow for its reached members overflows", per_member = per_member);
    };
    t
}

/// A request read against the kind: its kink's position found by name, one instance per role taken for the role
/// `within` the reached row names.
#[clause("REP.8", "REP.16")]
#[must_use]
pub fn pooled_split(req: &SplitRequest, within: u8, kinks: &KinkRegistry, kind: &PopKindDecl) -> PooledSplit {
    let reached = RowShare { count: req.count, own_balance: 0 };
    let SplitCause::Kink(k) = req.cause else {
        return PooledSplit { count: req.count, rows: vec![((req.line, req.side), reached)], own: Vec::new() };
    };
    let paid = -times(req.per_member, req.count);
    let rows = if req.account == req.line {
        vec![((req.line, req.side), RowShare { count: req.count, own_balance: paid })]
    } else {
        let funds = RowShare { count: req.count, own_balance: paid };
        vec![((req.line, req.side), reached), ((req.account, Side::Asset), funds)]
    };
    let Some(decl) = kinks.get(usize::from(k)) else {
        violation!(clause = "REP.16", "a split request naming a kink never registered", kink = k);
    };
    let own = match decl.on {
        KinkOn::Position(name) => {
            let role = usize::from(within);
            let found = kind
                .positions
                .iter()
                .position(|p| p.name == name && (p.role == Missing::Absent || p.role == Missing::Present(role)));
            let Some(i) = found else {
                violation!(clause = "REP.16", "a kink on a position the kind does not hold", kink = k);
            };
            vec![(i, times(req.moves, req.count))]
        }
        KinkOn::PerMember(_) => Vec::new(),
    };
    PooledSplit { count: req.count, rows, own }
}

#[cfg(test)]
mod tests {
    use phx_core::{KinkDecl, KinkOn, KinkRegistry, KinkSource};
    use phx_id::{LineId, PartyId};
    use phx_ledger::algebra::Side;
    use phx_ledger::part::RowShare;
    use phx_ledger::pooled::{Kink, PooledRow, RowOutcome, pooled};
    use phx_ledger::split_request::{SplitCause, SplitRequest};

    use super::pooled_split;
    use crate::fixture::kind;

    fn request(cause: SplitCause, count: u32, per_member: i64) -> SplitRequest {
        SplitRequest {
            holder: PartyId::new(7),
            line: LineId::new(3),
            side: Side::Asset,
            count,
            cause,
            account: LineId::new(0),
            per_member,
            moves: -per_member,
        }
    }

    #[test]
    fn pooled_rule_cases() {
        // The fourth-round reviewer's case: a cell of ten with 100 in its account owes 12 a member on a row reaching
        // all ten; its share is 10, so the row fails, and every row after it with it.
        let rows = [
            PooledRow { per_member: 12, reached: 10, position: 0, moves: 0 },
            PooledRow { per_member: 1, reached: 10, position: 0, moves: 0 },
        ];
        assert_eq!(pooled(100, 10, &rows, &[]), [RowOutcome::Fails, RowOutcome::Fails]);
        // A small due within every member's share is pooled and splits nobody.
        assert_eq!(
            pooled(100, 10, &[PooledRow { per_member: 3, reached: 10, position: 0, moves: 0 }], &[]),
            [RowOutcome::Pooled]
        );
        // A wage reaching four of ten carries them across a tax band: they split, taking what the wage moved whole.
        let band = Kink { at: 1_000, fails: false };
        let wage = PooledRow { per_member: -300, reached: 4, position: 800, moves: 300 };
        assert_eq!(pooled(0, 10, &[wage], &[band]), [RowOutcome::Splits { reached: 4 }]);
        let mut kinks = KinkRegistry::default();
        kinks
            .register(KinkDecl {
                name: "TAX.band",
                on: KinkOn::Position("HH.income"),
                source: KinkSource::Rule("TAX.income_tax"),
                points: 2,
                owner: "TAX",
                clause: "TAX.2",
            })
            .unwrap();
        let split = pooled_split(&request(SplitCause::Kink(0), 4, -300), 0, &kinks, &kind());
        assert_eq!(split.count, 4);
        assert_eq!(
            split.rows,
            [
                ((LineId::new(3), Side::Asset), RowShare { count: 4, own_balance: 0 }),
                ((LineId::new(0), Side::Asset), RowShare { count: 4, own_balance: 1_200 }),
            ],
            "the reached members leave the wage's row, with the 1 200 it paid them on their account"
        );
        assert_eq!(split.own, [(0, 1_200)], "and the income it moved for them");
        let failed = pooled_split(&request(SplitCause::Failed, 4, 12), 0, &kinks, &kind());
        assert_eq!((failed.rows.len(), failed.own.len()), (1, 0), "a failed flow moved nothing");
    }
}
