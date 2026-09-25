//! An estate settled: what it holds in money pays what it owes through the waterfall, what it owes beyond is lost by
//! its creditors, what it holds beyond is paid to the destination the law names, and then every row it holds leaves
//! with its counterparts and the estate ends.

use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};

use crate::algebra::Side;
use crate::books::Books;
use crate::dues::{Found, row_leg};
use crate::fails::Fail;
use crate::transfer::MoveAt;
use crate::waterfall::{Claim, Realised, waterfall};
use phx_core::AuditStream;

/// What an estate's settlement moved: paid to its creditors, lost by them, and passed on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    pub paid: i64,
    pub written_off: i64,
    pub passed: i64,
}

/// A debt of the estate: the line it is owed on and the one party holding the claim.
type Debt = (LineId, PartyId);

impl<B: phx_store::Backing> Books<B> {
    /// The one party holding the other side of a line a debtor owes on.
    fn creditor(&self, line: LineId, debtor: PartyId) -> PartyId {
        let holding: Vec<PartyId> = self
            .line_holders_but(line, debtor)
            .into_iter()
            .filter(|p| self.row_on_side(*p, line, Side::Asset).is_some())
            .collect();
        let [creditor] = holding.as_slice() else {
            violation!(clause = "L3", "an estate's debt held by other than one creditor", line = line.get());
        };
        *creditor
    }

    /// An estate settled, all at once or, when a payment fails, up to it: the waterfall over its money and its debts,
    /// one class in the order they are held, the residue of a division falling on the first debt of its currency;
    /// then what is left to `destination`; then each row leaving with its counterparts, and the estate's end.
    ///
    /// # Errors
    /// The first payment that could not settle; what settled before it stands, and a later settlement goes on from
    /// there.
    #[clause("L3", "POP.9", "POP.15", "PTY.9")]
    pub fn settle_estate(
        &mut self,
        estate: PartyId,
        destination: PartyId,
        m: MoveAt,
        draws: &mut phx_rand::Draws,
        audit: &mut dyn AuditStream,
    ) -> Result<Settled, Fail> {
        let (place, slot) = self.parties.row(estate);
        let rows = crate::rows::rows(self.parties.holder(place), slot);
        let (mut realised, mut claims, mut debts): (Vec<Realised>, Vec<Claim>, Vec<Debt>) = Default::default();
        for r in &rows {
            let line = r.row.line;
            let Missing::Present(balance) = r.optional.balance else { continue };
            let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
            match r.side() {
                Side::Asset if self.ledger.lines.is_money(line) => {
                    if balance < 0 {
                        violation!(clause = "MON.7", "an estate overdrawn on its account", line = line.get());
                    }
                    realised.push(Realised { ccy, proceeds: balance, collateral: Missing::Absent });
                }
                Side::Liability if balance < 0 => {
                    let Ok(id) = u32::try_from(claims.len()) else {
                        phx_num::capacity_exceeded!("an estate's debts", u32::MAX, claims.len());
                    };
                    claims.push(Claim { id, ccy, amount: -balance, class: 0, secured_by: Missing::Absent });
                    debts.push((line, self.creditor(line, estate)));
                }
                Side::Asset | Side::Liability => {}
            }
        }
        let mut firsts: Vec<(Ccy, u8, u32)> = Vec::new();
        for c in &claims {
            if !firsts.iter().any(|(ccy, _, _)| *ccy == c.ccy) {
                firsts.push((c.ccy, 0, c.id));
            }
        }
        let fall = waterfall(&realised, &claims, &[0], &firsts);
        let mut found = Found::default();
        let mut out = Settled::default();
        for (paid, (claim, (line, creditor))) in fall.paid.iter().zip(claims.iter().zip(&debts)) {
            let short = claim.amount - paid.paid;
            if paid.paid > 0 {
                let mut legs = self.pay(estate, *creditor, paid.paid, claim.ccy, &mut found);
                legs.push(row_leg(estate, *line, Side::Liability, paid.paid, claim.ccy));
                legs.push(row_leg(*creditor, *line, Side::Asset, -paid.paid, claim.ccy));
                let _ = self.submit(self.dues.payment, legs, m, audit)?;
                out.paid += paid.paid;
            }
            if short > 0 {
                let legs = vec![
                    row_leg(estate, *line, Side::Liability, short, claim.ccy),
                    row_leg(*creditor, *line, Side::Asset, -short, claim.ccy),
                ];
                let _ = self.submit(self.dues.written_off, legs, m, audit)?;
                out.written_off += short;
            }
        }
        for (ccy, left) in fall.left {
            if left > 0 {
                let legs = self.pay(estate, destination, left, ccy, &mut found);
                let _ = self.submit(self.dues.distributed, legs, m, audit)?;
                out.passed += left;
            }
        }
        let (place, slot) = self.parties.row(estate);
        let held: Vec<(LineId, Side, u32)> = crate::rows::rows(self.parties.holder(place), slot)
            .iter()
            .map(|r| (r.row.line, r.side(), r.row.count))
            .collect();
        for (line, side, count) in held {
            let _ = self.members_leave((estate, line, side), count, m, draws, audit)?;
        }
        self.parties.end(estate, m.day);
        Ok(out)
    }
}
