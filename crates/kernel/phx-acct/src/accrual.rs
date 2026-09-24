use std::collections::BTreeMap;

use phx_id::{LineId, PartyId};
use phx_ledger::effects::{DueOutcome, DueRec};
use phx_macros::clause;

use crate::equity::EquityEvent;

/// A claim recognised and not yet paid, keyed by its line, its holder and its counterparty.
type ClaimKey = (LineId, PartyId, PartyId);

/// Receivables and payables, kept as two records — each side's from its own side of each due — so that they agree
/// line by line is a check, not a construction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Claims {
    receivable: BTreeMap<ClaimKey, i64>,
    payable: BTreeMap<ClaimKey, i64>,
    by_party: BTreeMap<PartyId, (i128, i128)>,
}

fn add(map: &mut BTreeMap<ClaimKey, i64>, key: ClaimKey, amount: i64) {
    let v = map.entry(key).or_insert(0);
    *v += amount;
    if *v == 0 {
        map.remove(&key);
    }
}

/// The claims saved as their two records, and each party's totals rebuilt from them on load.
impl phx_store::Saved for Claims {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        self.receivable.save(w);
        self.payable.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Claims, phx_store::LoadError> {
        let receivable: BTreeMap<ClaimKey, i64> = phx_store::Saved::load(r)?;
        let payable: BTreeMap<ClaimKey, i64> = phx_store::Saved::load(r)?;
        let mut by_party: BTreeMap<PartyId, (i128, i128)> = BTreeMap::new();
        for ((_, holder, _), v) in &receivable {
            by_party.entry(*holder).or_insert((0, 0)).0 += i128::from(*v);
        }
        for ((_, holder, _), v) in &payable {
            by_party.entry(*holder).or_insert((0, 0)).1 += i128::from(*v);
        }
        Ok(Claims { receivable, payable, by_party })
    }
}

impl Claims {
    /// A due that fell today: its interest earned by the payee and incurred by the payer, each recognised with a
    /// receivable or payable naming the other, and cleared again where the due was paid; its principal is the
    /// contract's own balance and no accrual. The income events the accrual makes.
    #[clause("ACC.1", "ACC.12", "ACC.13")]
    pub fn post(&mut self, due: &DueRec) -> [EquityEvent; 2] {
        let interest = due.interest.amt();
        let owed = if due.outcome == DueOutcome::Settled { 0 } else { interest };
        add(&mut self.receivable, (due.line, due.payee, due.payer), owed);
        add(&mut self.payable, (due.line, due.payer, due.payee), owed);
        self.by_party.entry(due.payee).or_insert((0, 0)).0 += i128::from(owed);
        self.by_party.entry(due.payer).or_insert((0, 0)).1 += i128::from(owed);
        [EquityEvent::earned(due.payee, interest), EquityEvent::earned(due.payer, -interest)]
    }

    /// A receivable recognised with no payable to meet it, for the audit's injection alone.
    pub(crate) fn receivable_alone(&mut self, line: LineId, holder: PartyId, other: PartyId, amount: i64) {
        add(&mut self.receivable, (line, holder, other), amount);
        self.by_party.entry(holder).or_insert((0, 0)).0 += i128::from(amount);
    }

    /// What a party is owed and owes on its recognised, unpaid claims.
    #[must_use]
    pub fn of(&self, party: PartyId) -> (i128, i128) {
        // A party with no claim recognised is owed nothing and owes nothing: a count, not an unknown.
        self.by_party.get(&party).copied().unwrap_or((0, 0))
    }

    /// Each line where what its holders count receivable differs from what its counterparties count payable.
    #[clause("ACC.12")]
    #[must_use]
    pub fn mismatches(&self) -> Vec<(LineId, i128, i128)> {
        let mut per_line: BTreeMap<LineId, (i128, i128)> = BTreeMap::new();
        for ((line, _, _), v) in &self.receivable {
            per_line.entry(*line).or_insert((0, 0)).0 += i128::from(*v);
        }
        for ((line, _, _), v) in &self.payable {
            per_line.entry(*line).or_insert((0, 0)).1 += i128::from(*v);
        }
        per_line.into_iter().filter(|(_, (r, p))| r != p).map(|(l, (r, p))| (l, r, p)).collect()
    }

    /// Each unpaid receivable and payable into the world's hash.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        for map in [&self.receivable, &self.payable] {
            h.u64(phx_rand::float::len_u64(map.len()));
            for ((line, holder, other), v) in map {
                h.u64(u64::from(line.get()));
                h.u64(holder.get());
                h.u64(other.get());
                h.u64(v.cast_unsigned());
            }
        }
    }

    /// The lines with a recognised, unpaid claim.
    #[must_use]
    pub fn lines(&self) -> usize {
        self.receivable.len() + self.payable.len()
    }

    /// Everything receivable and everything payable across the world.
    #[must_use]
    pub fn totals(&self) -> (i128, i128) {
        let r = self.receivable.values().map(|v| i128::from(*v)).sum();
        let p = self.payable.values().map(|v| i128::from(*v)).sum();
        (r, p)
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{LineId, PartyId};
    use phx_ledger::effects::{DueOutcome, DueRec};
    use phx_num::{Ccy, Money};

    use super::Claims;

    #[test]
    fn a_due_accrues_and_a_payment_clears() {
        let (firm, bank, eur) = (PartyId::new(5), PartyId::new(2), Ccy::new(0));
        let due = |outcome| DueRec {
            line: LineId::new(7),
            payer: firm,
            payee: bank,
            interest: Money::new(30, eur),
            principal: Money::new(0, eur),
            outcome,
        };
        let mut claims = Claims::default();
        let events = claims.post(&due(DueOutcome::Settled));
        assert_eq!((events[0].amount(), events[1].amount()), (30, -30), "earned the day it fell, paid or not");
        assert_eq!(claims.of(bank), (0, 0), "paid, nothing stays receivable");
        let _ = claims.post(&due(DueOutcome::Failed));
        assert_eq!((claims.of(bank).0, claims.of(firm).1), (30, 30), "unpaid, a receivable and a payable");
        assert!(claims.mismatches().is_empty());
        assert_eq!(claims.totals(), (30, 30));
    }

    #[test]
    fn rebuild_equals_live_indexes() {
        use phx_store::Saved as _;
        let (eur, parties) = (Ccy::new(0), [PartyId::new(2), PartyId::new(5), PartyId::new(8)]);
        let mut claims = Claims::default();
        for (i, (payer, payee)) in [(0, 1), (1, 2), (2, 0), (0, 2)].into_iter().enumerate() {
            let due = DueRec {
                line: LineId::new(u32::try_from(i).unwrap()),
                payer: parties[payer],
                payee: parties[payee],
                interest: Money::new(10 + i64::try_from(i).unwrap(), eur),
                principal: Money::new(0, eur),
                outcome: if i == 1 { DueOutcome::Settled } else { DueOutcome::Failed },
            };
            let _ = claims.post(&due);
        }
        let mut bytes = Vec::new();
        let mut w = phx_store::Writer::new(&mut bytes).unwrap();
        claims.save(&mut w);
        w.finish().unwrap();
        let mut src: &[u8] = &bytes;
        let back = Claims::load(&mut phx_store::Reader::new(&mut src).unwrap()).unwrap();
        assert_eq!(back, claims, "each party's totals, rebuilt from the two records, are the ones kept live");
    }
}
