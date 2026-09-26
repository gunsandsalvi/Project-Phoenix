//! An estate settled: the contracts that hold no claim — its staff's jobs, its tenants' tenancies — end at once with
//! their counterparts; what it holds in money pays what it owes through the waterfall; while it still holds units,
//! which only a sale can turn into money, what it owes beyond stays owed and it waits; once it holds none, what it
//! owes beyond is lost by its creditors, what it holds beyond passes to the destination the law names, its last rows
//! leave and it ends.

use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};

use crate::algebra::Side;
use crate::books::Books;
use crate::dues::row_leg;
use crate::fails::Fail;
use crate::transfer::MoveAt;
use crate::waterfall::{Claim, Realised, waterfall};
use phx_core::AuditStream;

/// What an estate's settlement moved: paid to its creditors, lost by them, and passed on; who left with it; and
/// whether it ended, waits to sell the units it holds, or stopped at a payment that failed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    pub paid: i64,
    pub written_off: i64,
    pub passed: i64,
    /// The holders drawn to leave with the estate's members, each with its line, its side and how many left.
    pub left: Vec<(PartyId, LineId, Side, u32)>,
    pub ended: bool,
    /// Units still held, which the estate must sell before its creditors' loss is known.
    pub unsold: bool,
    /// The payment that failed, where one did; what settled before it stands.
    pub fail: Option<Fail>,
    /// Each debt written off: its creditor, its line and the amount lost.
    pub lost: Vec<(PartyId, LineId, i64)>,
}

/// A debt of the estate: the line it is owed on and the one party holding the claim.
type Debt = (LineId, PartyId);

impl<B: phx_store::Backing> Books<B> {
    /// Every holding of one party passed whole to another, each as its units at its basis, one instruction a holding:
    /// what an ended firm held to its estate.
    /// Returns the bases passed.
    ///
    /// # Errors
    /// The first move that could not settle.
    #[clause("PTY.9", "L3")]
    pub fn pass_holdings(
        &mut self,
        (from, to): (PartyId, PartyId),
        reason: crate::instruction::ReasonId,
        m: MoveAt,
        audit: &mut dyn AuditStream,
    ) -> Result<i64, Fail> {
        let (place, slot) = self.parties.row(from);
        let held: Vec<(phx_id::InstrumentId, i64, i64)> = {
            let table = self.parties.holder(place);
            crate::holding::bases(table, slot)
                .into_iter()
                .map(|(instrument, basis)| {
                    let Missing::Present(h) = crate::holding::holding(table, slot, instrument) else {
                        violation!(clause = "REG.4", "a basis of a holding its holder does not hold");
                    };
                    (instrument, h.quantity.raw(), basis)
                })
                .collect()
        };
        let mut passed = 0_i64;
        for (instrument, units, basis) in held {
            let unit = self.ledger.instruments.get(instrument).unit;
            let leg = |party, qty, cost| crate::instruction::LegRec {
                party,
                account: crate::instruction::AccountRef::Instrument(instrument),
                qty,
                denom: crate::instruction::Denom::Unit(unit),
                kind: crate::instruction::LegKind::Units { cost },
            };
            let _ = self.submit(reason, vec![leg(from, -units, -basis), leg(to, units, basis)], m, audit)?;
            passed += basis;
        }
        Ok(passed)
    }

    /// The one party holding the other side of a line a debtor owes on.
    fn creditor(&self, line: LineId, debtor: PartyId) -> PartyId {
        // A listed side of one holder names it at once, without reading the line's other holders.
        if let Some(key) = self.ledger.lines.sole_holder(line, Side::Asset) {
            let creditor = self.party_of_key(key);
            if creditor != debtor {
                return creditor;
            }
        }
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

    /// The sides that keep no holder list its rows' members would leave against: what must be read before the party
    /// leaves its lines.
    #[must_use]
    pub fn unlisted_against(&self, party: PartyId) -> Vec<(LineId, Side)> {
        let (place, slot) = self.parties.row(party);
        crate::rows::rows(self.parties.holder(place), slot)
            .iter()
            .map(|r| {
                let other = match r.side() {
                    Side::Asset => Side::Liability,
                    Side::Liability => Side::Asset,
                };
                (r.row.line, other)
            })
            .filter(|(l, s)| !self.ledger.lines.listed_side(*l, *s))
            .collect()
    }

    /// Whether an estate holds units, which must be sold before its creditors' loss is known.
    #[must_use]
    pub fn holds_units(&self, estate: PartyId) -> bool {
        let (place, slot) = self.parties.row(estate);
        !crate::holding::bases(self.parties.holder(place), slot).is_empty()
    }

    /// An estate settled as far as it can: its contracts that hold no claim end with their counterparts; its money pays
    /// its debts through the waterfall, one class in the order they are held, the residue of a division falling on the
    /// first debt of its currency; then, if it holds no units, what it owes beyond is lost, what is left passes to
    /// `destination`, its last rows leave and it ends. One that holds units waits, its debts beyond its money still
    /// owed, until they are sold. A payment that fails stops it where it stands, and a later settlement goes on from
    /// there.
    #[clause("L3", "POP.9", "POP.15", "PTY.9")]
    pub fn settle_estate(
        &mut self,
        estate: PartyId,
        destination: PartyId,
        m: MoveAt,
        draws: &mut phx_rand::Draws,
        audit: &mut dyn AuditStream,
    ) -> Settled
    where
        B: Sync,
    {
        let mut out = Settled::default();
        if let Err(f) = self.release(estate, m, draws, audit, &mut out) {
            out.fail = Some(f);
            return out;
        }
        out.unsold = self.holds_units(estate);
        if let Err(f) = self.wind_up(estate, destination, m, draws, audit, &mut out) {
            out.fail = Some(f);
        }
        out
    }

    /// The estate's contracts that hold no claim, of no balance, leaving with their counterparts.
    fn release(
        &mut self,
        estate: PartyId,
        m: MoveAt,
        draws: &mut phx_rand::Draws,
        audit: &mut dyn AuditStream,
        out: &mut Settled,
    ) -> Result<(), Fail>
    where
        B: Sync,
    {
        let (place, slot) = self.parties.row(estate);
        let free: Vec<(LineId, Side, u32)> = crate::rows::rows(self.parties.holder(place), slot)
            .iter()
            .filter(|r| matches!(r.optional.balance, Missing::Absent | Missing::Present(0)))
            .filter(|r| !self.ledger.lines.is_money(r.row.line))
            .map(|r| (r.row.line, r.side(), r.row.count))
            .collect();
        self.leave_all(estate, &free, (m, draws, audit), out)
    }

    /// Rows leaving whole with their counterparts, each counterpart drawn noted.
    fn leave_all(
        &mut self,
        estate: PartyId,
        held: &[(LineId, Side, u32)],
        (m, draws, audit): (MoveAt, &mut phx_rand::Draws, &mut dyn AuditStream),
        out: &mut Settled,
    ) -> Result<(), Fail>
    where
        B: Sync,
    {
        for (line, side, count) in held {
            let other = match side {
                Side::Asset => Side::Liability,
                Side::Liability => Side::Asset,
            };
            let taken = self.members_leave((estate, *line, *side), *count, m, draws, audit)?;
            out.left.extend(taken.into_iter().map(|(p, k)| (p, *line, other, k)));
        }
        Ok(())
    }

    /// The waterfall over the estate's money; and, once it holds no units, the loss, the residue and its end.
    fn wind_up(
        &mut self,
        estate: PartyId,
        destination: PartyId,
        m: MoveAt,
        draws: &mut phx_rand::Draws,
        audit: &mut dyn AuditStream,
        out: &mut Settled,
    ) -> Result<(), Fail>
    where
        B: Sync,
    {
        let (place, slot) = self.parties.row(estate);
        let rows = crate::rows::rows(self.parties.holder(place), slot);
        // The waterfall runs on one twin's estate, and each of its amounts is paid for every twin alike.
        let unit = i64::from(self.parties.unit(estate));
        let (mut realised, mut claims, mut debts): (Vec<Realised>, Vec<Claim>, Vec<Debt>) = Default::default();
        for r in &rows {
            let line = r.row.line;
            let Missing::Present(balance) = r.optional.balance else { continue };
            if balance % unit != 0 {
                violation!(clause = "REP.9", "an estate's balance not a whole share for each twin", line = line.get());
            }
            let balance = balance / unit;
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
        let every = |x: i64| {
            let Some(all) = x.checked_mul(unit) else {
                phx_num::capacity_exceeded!("an estate's amount for its twins", i64::MAX, x);
            };
            all
        };
        for (paid, (claim, (line, creditor))) in fall.paid.iter().zip(claims.iter().zip(&debts)) {
            let (paid, short) = (every(paid.paid), every(claim.amount - paid.paid));
            if paid > 0 {
                let mut legs = self.pay(estate, *creditor, paid, claim.ccy);
                legs.push(row_leg(estate, *line, Side::Liability, paid, claim.ccy));
                legs.push(row_leg(*creditor, *line, Side::Asset, -paid, claim.ccy));
                let _ = self.submit(self.dues.payment, legs, m, audit)?;
                out.paid += paid;
            }
            // What the units still held will fetch is owed to the debts beyond the money, so none is lost yet.
            if short > 0 && !out.unsold {
                let legs = vec![
                    row_leg(estate, *line, Side::Liability, short, claim.ccy),
                    row_leg(*creditor, *line, Side::Asset, -short, claim.ccy),
                ];
                let _ = self.submit(self.dues.written_off, legs, m, audit)?;
                out.written_off += short;
                out.lost.push((*creditor, *line, short));
            }
        }
        if out.unsold {
            return Ok(());
        }
        for (ccy, left) in fall.left {
            let left = every(left);
            if left > 0 {
                let legs = self.pay(estate, destination, left, ccy);
                let _ = self.submit(self.dues.distributed, legs, m, audit)?;
                out.passed += left;
            }
        }
        let (place, slot) = self.parties.row(estate);
        let held: Vec<(LineId, Side, u32)> = crate::rows::rows(self.parties.holder(place), slot)
            .iter()
            .map(|r| (r.row.line, r.side(), r.row.count))
            .collect();
        self.leave_all(estate, &held, (m, draws, audit), out)?;
        self.parties.end(estate, m.day);
        out.ended = true;
        Ok(())
    }
}
