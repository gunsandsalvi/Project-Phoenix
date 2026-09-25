use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::algebra::Side;
use crate::instruction::{AccountRef, Denom, LegKind, LegRec, RowOp};

/// Why an instruction did not settle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub enum FailCause {
    /// A payer had not the money, within any facility its deposit's terms grant.
    Funds,
    /// A giver had not the free units: not held, pledged, or covering an offer.
    FreeUnits,
    /// A named unit its giver does not hold.
    NotHeld,
    /// A party that ended with no successor to answer for it.
    Ended,
    /// A payer or payee holding no money in the payment's currency: no account, and none it issues.
    NoMoney,
    /// A payer whose bank could not cover its net after intraday credit, so its customers' legs were removed.
    BankShort,
}

/// What a leg draws on before the instruction: what is there now, and how far it may fall; a position with no floor,
/// an issuer's liability, may fall without end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub now: i64,
    pub floor: Missing<i64>,
    pub short: FailCause,
}

/// Whether every position the legs move stays at or above its floor after all of them: each leg names the position
/// it moves and by how much, and a position moved by several legs is judged on their sum. The first failing position's
/// cause and the first leg that moved it are returned.
///
/// # Errors
/// The cause and leg of the first position that would fall below its floor.
#[clause("SET.4", "MON.3", "MON.12")]
pub fn check_legs(positions: &[Position], moves: &[(usize, i64)]) -> Result<(), (FailCause, usize)> {
    // Each position's net and its first move, in one pass over the moves.
    let mut net: Vec<(i128, Option<usize>)> = vec![(0, None); positions.len()];
    for (m, (at, q)) in moves.iter().enumerate() {
        let Some((sum, first)) = net.get_mut(*at) else {
            violation!(clause = "SET.4", "a move of no position", position = *at);
        };
        *sum += i128::from(*q);
        first.get_or_insert(m);
    }
    for (at, (p, (delta, first))) in positions.iter().zip(net).enumerate() {
        let Missing::Present(floor) = p.floor else { continue };
        if delta < 0 && i128::from(p.now) + delta < i128::from(floor) {
            let Some(first) = first else {
                violation!(clause = "SET.4", "a position moved by no leg", position = at);
            };
            return Err((p.short, first));
        }
    }
    Ok(())
}

/// What a leg counts toward its instruction's balance: its quantity; a row opened or retired counts its members, on
/// the asset side against the liability side, so a line's two sides open and close together.
fn signed(leg: &LegRec) -> i128 {
    i128::from(flow(leg))
}

/// What a leg counts toward its instruction's balance, as the audit keeps it.
pub(crate) fn flow(leg: &LegRec) -> i64 {
    match (leg.kind, leg.account) {
        (
            LegKind::Row(RowOp::Open(_) | RowOp::Close | RowOp::Count),
            AccountRef::Line { side: Side::Liability, .. },
        ) => -leg.qty,
        _ => leg.qty,
    }
}

/// The first denomination whose paired legs do not sum to nothing, if any: every instruction has both its sides.
#[clause("SET.9", "SET.11")]
#[must_use]
pub fn unbalanced(legs: &[LegRec]) -> Option<Denom> {
    let mut sums: Vec<(Denom, i128)> = Vec::new();
    for leg in legs.iter().filter(|l| l.paired()) {
        match sums.iter_mut().find(|(d, _)| *d == leg.denom) {
            Some((_, s)) => *s += signed(leg),
            None => sums.push((leg.denom, signed(leg))),
        }
    }
    sums.into_iter().find(|(_, s)| *s != 0).map(|(d, _)| d)
}
