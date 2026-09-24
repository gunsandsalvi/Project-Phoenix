use std::collections::BTreeMap;

use phx_id::PartyId;
use phx_ledger::effects::EffectRec;
use phx_ledger::instruction::Effect;
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};

/// What moves an equity account: income recognised (an expense negative), capital paid in or returned, a
/// distribution to owners, a revaluation the basis sends straight to equity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquityKind {
    Income,
    Capital,
    Distribution,
    Revaluation,
}

/// An event that moves an equity account. Only this crate makes one, from an accounting effect a reason declares or
/// from an accrual, so no account moves without a declared cause.
#[clause("ACC.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquityEvent {
    party: PartyId,
    kind: EquityKind,
    amount: i64,
}

impl EquityEvent {
    /// Income a party earned (or, negative, an expense it incurred) by an accrual.
    pub(crate) fn earned(party: PartyId, amount: i64) -> EquityEvent {
        EquityEvent { party, kind: EquityKind::Income, amount }
    }

    /// The event a settled leg's declared effect makes, where it makes one: revenue and expense are income, an
    /// equity effect is capital; an asset or liability moved is no event.
    pub fn from_effect(e: &EffectRec) -> Missing<EquityEvent> {
        let amt = e.amount.amt();
        let (kind, amount) = match e.effect {
            Effect::Revenue => (EquityKind::Income, amt),
            Effect::Expense => (EquityKind::Income, -amt),
            Effect::Equity => (EquityKind::Capital, amt),
            Effect::Asset | Effect::Liability => return Missing::Absent,
        };
        Missing::Present(EquityEvent { party: e.party, kind, amount })
    }

    pub fn party(&self) -> PartyId {
        self.party
    }

    #[must_use]
    pub fn kind(&self) -> EquityKind {
        self.kind
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

/// A party's equity account: its balance, in its own money.
#[clause("ACC.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct EquityAccount {
    balance: i64,
    ccy: Ccy,
}

impl EquityAccount {
    #[must_use]
    pub fn balance(&self) -> i64 {
        self.balance
    }

    pub fn ccy(&self) -> Ccy {
        self.ccy
    }
}

/// Every equity account, by its party: opened once at the party's opening equity, and moved after that only by
/// events, never set from assets less liabilities.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct EquityAccounts {
    accounts: BTreeMap<PartyId, EquityAccount>,
}

impl EquityAccounts {
    /// A party's account opened at its opening equity.
    pub(crate) fn open(&mut self, party: PartyId, ccy: Ccy, balance: i64) {
        if self.accounts.insert(party, EquityAccount { balance, ccy }).is_some() {
            violation!(clause = "ACC.4", "an equity account opened twice", party = party.get());
        }
    }

    /// An event moves its party's account; a party with no account has only a net worth, which no event moves.
    #[clause("ACC.4", "ACC.10")]
    pub(crate) fn post(&mut self, event: &EquityEvent) {
        if let Some(a) = self.accounts.get_mut(&event.party) {
            let Some(b) = a.balance.checked_add(event.amount) else {
                phx_num::capacity_exceeded!("an equity account", i64::MAX, event.amount);
            };
            a.balance = b;
        }
    }

    pub fn of(&self, party: PartyId) -> Missing<EquityAccount> {
        match self.accounts.get(&party) {
            Some(a) => Missing::Present(*a),
            None => Missing::Absent,
        }
    }

    pub fn parties(&self) -> impl Iterator<Item = PartyId> + '_ {
        self.accounts.keys().copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, PartyId};
    use phx_ledger::effects::EffectRec;
    use phx_ledger::instruction::{Effect, InstructionId};
    use phx_num::{Ccy, Missing, Money};

    use super::{EquityAccounts, EquityEvent};

    #[test]
    fn equity_moves_only_by_events() {
        let (bank, eur) = (PartyId::new(3), Ccy::new(0));
        let mut accounts = EquityAccounts::default();
        accounts.open(bank, eur, 1_000);
        let leg = |effect, amt| EffectRec {
            instruction: InstructionId::new(Day::new(1), 0),
            party: bank,
            effect,
            amount: Money::new(amt, eur),
        };
        for rec in
            [leg(Effect::Revenue, 40), leg(Effect::Expense, 15), leg(Effect::Asset, 500), leg(Effect::Liability, 70)]
        {
            if let Missing::Present(e) = EquityEvent::from_effect(&rec) {
                accounts.post(&e);
            }
        }
        assert!(
            matches!(accounts.of(bank), Missing::Present(a) if a.balance() == 1_025),
            "income moves it; money moved does not"
        );
        accounts.post(&EquityEvent::earned(PartyId::new(9), 5));
        assert_eq!(accounts.len(), 1, "a party without owners keeps no account");
    }
}
