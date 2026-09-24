use phx_core::IntentDef;
use phx_id::{LineId, PartyId};
use phx_macros::clause;

use crate::algebra::Side;

/// Why some of a row's members must split off: a flow failed for them, or carried them across a kink of the named
/// index in the kink registry's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitCause {
    Failed,
    Kink(u16),
}

/// A request, from the pooled-flow rule, that `count` members of a holder's row split off with their own outcome;
/// the population turns it into parts, since the ledger names no population type. A flow paid for them was applied to
/// the holder's totals: `per_member` is what it took from the funds row `account` per member (paid positive, received
/// negative) and `moves` how it moved the position the kink lies on, per member, which the part takes whole. A failed
/// flow moved nothing.
#[clause("REP.8", "REP.16")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitRequest {
    pub holder: PartyId,
    pub line: LineId,
    pub side: Side,
    pub count: u32,
    pub cause: SplitCause,
    pub account: LineId,
    pub per_member: i64,
    pub moves: i64,
}

const FAILED: u64 = u64::MAX;

impl IntentDef for SplitRequest {
    const NAME: &'static str = "ledger.split_request";

    fn encode(&self, out: &mut Vec<u64>) {
        let side = match self.side {
            Side::Asset => 0,
            Side::Liability => 1,
        };
        let cause = match self.cause {
            SplitCause::Failed => FAILED,
            SplitCause::Kink(k) => u64::from(k),
        };
        out.extend([
            self.holder.get(),
            u64::from(self.line.get()),
            side,
            u64::from(self.count),
            cause,
            u64::from(self.account.get()),
            self.per_member.cast_unsigned(),
            self.moves.cast_unsigned(),
        ]);
    }
}

impl SplitRequest {
    /// A request from its words, as `encode` wrote them; none from words it did not write.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<SplitRequest> {
        let [holder, line, side, count, cause, account, per_member, moves] = *words else { return None };
        let side = match side {
            0 => Side::Asset,
            1 => Side::Liability,
            _ => return None,
        };
        let cause = if cause == FAILED { SplitCause::Failed } else { SplitCause::Kink(u16::try_from(cause).ok()?) };
        Some(SplitRequest {
            holder: PartyId::new(holder),
            line: LineId::new(u32::try_from(line).ok()?),
            side,
            count: u32::try_from(count).ok()?,
            cause,
            account: LineId::new(u32::try_from(account).ok()?),
            per_member: per_member.cast_signed(),
            moves: moves.cast_signed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use phx_core::IntentDef;
    use phx_id::{LineId, PartyId};

    use super::{SplitCause, SplitRequest};
    use crate::algebra::Side;

    #[test]
    fn a_split_request_round_trips() {
        for cause in [SplitCause::Failed, SplitCause::Kink(7)] {
            let r = SplitRequest {
                holder: PartyId::new(41),
                line: LineId::new(3),
                side: Side::Liability,
                count: 2,
                cause,
                account: LineId::new(9),
                per_member: -1_250,
                moves: 1_250,
            };
            let mut words = Vec::new();
            r.encode(&mut words);
            assert_eq!(SplitRequest::decode(&words), Some(r));
        }
        assert_eq!(SplitRequest::decode(&[1, 2, 3]), None);
    }
}
