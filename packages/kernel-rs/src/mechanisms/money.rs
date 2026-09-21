//! MONEY, AND WHAT HAPPENS WHEN AN ACCOUNT IS SHORT OF IT.
//!
//! @spec Money B1, Money B3.a, Money B3.b, Money B3.c, Money D2, Central Bank D3, Central Bank E2, Appendix B, Law 5

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::instruments::{outside_its_issuer, Class};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext, ParticipantView};

/// What an issuer answers when the account it issues into is short.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Overdraft {
    /// It lends.
    Lend {
        /// The line the borrower's debt is written on.
        owes: InstrumentId,
        per_annum: f64,
    },
    /// It refuses, and the refusal is the record.
    Refuse,
}

/// The door the kernel asks.
pub trait Issuer {
    /// Whose accounts this issuer answers for.
    fn party_kind(&self) -> u32;

    /// The bank lends only to the room its own capital supports, and refuses past it.
    fn overdraft(
        &self,
        borrower: &ParticipantView<'_>,
        short_by: crate::ledger::Units,
        ccy: CurrencyCode,
    ) -> Overdraft;
}

/// THERE IS NO CENTRAL-BANK OVERDRAFT FOR THE TREASURY.
pub struct NoOverdraftForTheTreasury {
    pub treasury_kind: u32,
}

impl Issuer for NoOverdraftForTheTreasury {
    fn party_kind(&self) -> u32 {
        self.treasury_kind
    }
    fn overdraft(
        &self,
        _b: &ParticipantView<'_>,
        _short: crate::ledger::Units,
        _c: CurrencyCode,
    ) -> Overdraft {
        Overdraft::Refuse
    }
}

/// Every issuer this world has, asked by the kind whose accounts they issue.
#[derive(Default)]
pub struct Issuers {
    by_kind: Vec<(u32, Box<dyn Issuer>)>,
}

impl Issuers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn declare(&mut self, issuer: Box<dyn Issuer>) {
        let kind = issuer.party_kind();
        assert!(
            !self.by_kind.iter().any(|(k, _)| *k == kind),
            "Law 4: two issuers answer for party kind {kind}"
        );
        self.by_kind.push((kind, issuer));
    }

    /// Ask the issuer of this borrower's bank.
    pub fn ask(
        &self,
        bank_kind: u32,
        borrower: &ParticipantView<'_>,
        short_by: crate::ledger::Units,
        ccy: CurrencyCode,
    ) -> Option<Overdraft> {
        self.by_kind
            .iter()
            .find(|(k, _)| *k == bank_kind)
            .map(|(_, i)| i.overdraft(borrower, short_by, ccy))
    }
}

/// What an overdraft BECOMES on the wire.
pub fn as_legs(
    borrower: PartyId,
    bank: PartyId,
    money: InstrumentId,
    short_by: crate::ledger::Units,
    lent: Overdraft,
) -> Option<[crate::ledger::Leg; 2]> {
    match lent {
        Overdraft::Refuse => None,
        Overdraft::Lend { owes, .. } => {
            Some([
                // The money exists because the bank issued it, and it goes to the borrower.
                crate::ledger::Leg::Money {
                    from: bank,
                    to: borrower,
                    instrument: money,
                    amount: short_by,
                    receipt: crate::ledger::Receipt::Principal,
                },
                // And the borrower owes it: the bank holds the claim from the same instant.
                crate::ledger::Leg::Create {
                    party: bank,
                    instrument: owes,
                    qty: short_by,
                    cost_per_unit: 1.0,
                },
            ])
        }
    }
}

// An account short by nothing is not overdrawn, and that is the TYPE now: `ask` and `as_legs` take
// `Units`, so there is no way to ask about a shortfall of zero. A kind nobody answers for answers
// `None`, which is the type again; two issuers declared for one kind panics in `declare`, where
// the second one arrives.
//
// What a bank lends past its room is the ISSUER's decision, and the fixture that checked it
// declared its own issuer, gave it a room and watched it refuse past it — the arrangement proving
// itself. `NoOverdraftForTheTreasury` is the one that matters and it is three lines that can
// return nothing else; nothing calls `ask` yet, so what exercises it is 0r wiring `money` in.

/// WHAT EACH ISSUER OF MONEY OWES THE WORLD — its own liability, read off the register.
pub struct Owed {
    pub kind: u32,
}

impl Mechanism for Owed {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut owed: Vec<(u32, f64)> = Vec::new();
        for i in 0..ctx.instruments().len() {
            let line = InstrumentId::at(i as u32);
            if ctx.instruments().class_of(line) != Class::Money {
                continue;
            }
            let outstanding = outside_its_issuer(line, ctx.register(), ctx.instruments());
            if outstanding > 0.0 {
                owed.push((ctx.instruments().issuer_of(line).0, outstanding));
            }
        }
        for (issuer, amount) in owed {
            ctx.say(self.kind, &[issuer], &[(0, Value::Num(amount))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(of: f64) -> crate::ledger::Units {
        crate::ledger::Units::new(of).expect("a loan moves something")
    }

    #[test]
    fn an_overdraft_is_a_loan_with_two_sides_and_never_a_silent_negative() {
        let (borrower, bank) = (PartyId::at(1), PartyId::at(0));
        let lent = Overdraft::Lend {
            owes: InstrumentId::at(9),
            per_annum: 0.09,
        };
        let legs = as_legs(borrower, bank, InstrumentId::at(0), units(400.0), lent)
            .expect("it lent, so there are legs");
        // The money came from the bank AND the bank holds the claim, in the same pass.
        match legs[0] {
            crate::ledger::Leg::Money {
                from, to, amount, ..
            } => {
                assert_eq!((from, to), (bank, borrower));
                assert_eq!(amount.get(), 400.0);
            }
            ref other => panic!("{:?}", std::mem::discriminant(other)),
        }
        match legs[1] {
            crate::ledger::Leg::Create { party, qty, .. } => {
                assert_eq!(party, bank, "the bank holds what it is owed");
                assert_eq!(qty.get(), 400.0);
            }
            ref other => panic!("{:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn a_refusal_writes_nothing_and_is_never_a_smaller_loan_nobody_asked_for() {
        let legs = as_legs(
            PartyId::at(1),
            PartyId::at(0),
            InstrumentId::at(0),
            units(5_000.0),
            Overdraft::Refuse,
        );
        assert!(legs.is_none());
    }
}
