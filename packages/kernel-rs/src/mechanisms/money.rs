//! MONEY, AND WHAT HAPPENS WHEN AN ACCOUNT IS SHORT OF IT.
//!
//! @spec Money B1, Money B3.a, Money B3.b, Money B3.c, Money D2, Central Bank D3, Central Bank E2, Appendix B, Law 5

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::instruments::{outside_its_issuer, Class};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext, ParticipantView};
use crate::num::MoneyAmount;
use std::collections::HashMap;

/// One book's settled cash movement in one currency for one week.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Statement {
    pub owner: PartyId,
    pub currency: CurrencyCode,
    pub gross_in: f64,
    pub gross_out: f64,
    pub net: f64,
}

/// Gross and net cash flows, kept per owner and currency and derived from the numbered wire.
pub fn statements(
    wire: &crate::ledger::Settlement,
    parties: &crate::parties::Parties,
    instruments: &crate::instruments::Instruments,
    week: u32,
) -> Vec<Statement> {
    let mut books: HashMap<(u32, u32), (f64, f64)> = HashMap::new();
    let mut add = |owner: PartyId, money: InstrumentId, incoming: f64, outgoing: f64| {
        let ccy = instruments.ccy_of(money).0;
        let flow = books.entry((owner.0, ccy)).or_default();
        flow.0 += incoming;
        flow.1 += outgoing;
    };
    for instruction in wire.in_period(week) {
        if wire.outcome_of(instruction) != crate::ledger::Outcome::Settled {
            continue;
        }
        for leg in wire.legs_of(instruction) {
            match *leg {
                crate::ledger::Leg::Money {
                    from,
                    to,
                    instrument,
                    amount,
                    ..
                } => {
                    add(from, instrument, 0.0, amount.get());
                    if crate::ledger::is_exchange_leg(leg, wire.legs_of(instruction), instruments) {
                        add(to, instrument, amount.get(), 0.0);
                    } else {
                        match crate::ledger::across(parties, instruments, to, instrument) {
                            crate::ledger::Across::Same => add(to, instrument, amount.get(), 0.0),
                            crate::ledger::Across::Banks {
                                payers_bank,
                                payees_bank,
                                payees_money,
                                reserves,
                            } => {
                                add(to, payees_money, amount.get(), 0.0);
                                add(payers_bank, reserves, 0.0, amount.get());
                                add(payees_bank, reserves, amount.get(), 0.0);
                            }
                            crate::ledger::Across::Refused(..) => {
                                unreachable!("settled money leg was pre-checked")
                            }
                        }
                    }
                }
                crate::ledger::Leg::Mint {
                    issuer,
                    money,
                    amount,
                } => {
                    add(issuer, money, amount.get(), 0.0);
                }
                crate::ledger::Leg::Destroy {
                    party,
                    instrument,
                    qty,
                    ..
                } if instruments.class_of(instrument) == Class::Money => {
                    add(party, instrument, 0.0, qty.get());
                }
                _ => {}
            }
        }
    }
    let mut answer: Vec<Statement> = books
        .into_iter()
        .map(|((owner, currency), (gross_in, gross_out))| Statement {
            owner: PartyId::at(owner),
            currency: CurrencyCode::at(currency),
            gross_in,
            gross_out,
            net: gross_in - gross_out,
        })
        .collect();
    answer.sort_by_key(|row| (row.owner.0, row.currency.0));
    answer
}

/// The clearing residual per currency. Transfers cancel; only named issuance or destruction can
/// leave a non-zero change in the stock.
pub fn residual(statements: &[Statement]) -> Vec<MoneyAmount> {
    let mut totals: HashMap<u32, f64> = HashMap::new();
    for row in statements {
        *totals.entry(row.currency.0).or_default() += row.net;
    }
    let mut answer: Vec<MoneyAmount> = totals
        .into_iter()
        .map(|(ccy, amount)| MoneyAmount::new(amount, CurrencyCode::at(ccy)).unwrap())
        .collect();
    answer.sort_by_key(|amount| amount.currency().0);
    answer
}

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
    /// The aggregate liability read, reported once per currency.
    pub stock_kind: u32,
}

/// The money stock, derived from money issuers' liabilities and grouped by denomination.
///
/// Nothing is stored beside the underlying accounts: every call reads the live register.
pub fn stock(
    register: &crate::register::Register,
    instruments: &crate::instruments::Instruments,
) -> Vec<MoneyAmount> {
    stock_from_liabilities((0..instruments.len()).filter_map(|row| {
        let line = InstrumentId::at(row as u32);
        (instruments.class_of(line) == Class::Money).then(|| {
            MoneyAmount::new(
                outside_its_issuer(line, register, instruments),
                instruments.ccy_of(line),
            )
            .expect("register quantities are finite")
        })
    }))
}

fn stock_from_liabilities(liabilities: impl IntoIterator<Item = MoneyAmount>) -> Vec<MoneyAmount> {
    let mut totals: Vec<MoneyAmount> = Vec::new();
    for amount in liabilities {
        match totals
            .iter_mut()
            .find(|total| total.currency() == amount.currency())
        {
            Some(total) => {
                *total = total
                    .checked_add(amount)
                    .expect("the money-stock aggregation key is the currency");
            }
            None => totals.push(amount),
        }
    }
    totals
}

impl Mechanism for Owed {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // One issuer may issue more than one currency. Keep the denomination on the number while
        // accumulating so those liabilities can never become one meaningless total.
        let mut owed: Vec<(u32, MoneyAmount)> = Vec::new();
        for i in 0..ctx.instruments().len() {
            let line = InstrumentId::at(i as u32);
            if ctx.instruments().class_of(line) != Class::Money {
                continue;
            }
            let outstanding = outside_its_issuer(line, ctx.register(), ctx.instruments());
            if outstanding > 0.0 {
                let issuer = ctx.instruments().issuer_of(line).0;
                let amount = MoneyAmount::new(outstanding, ctx.instruments().ccy_of(line))
                    .expect("register quantities are finite");
                match owed
                    .iter_mut()
                    .find(|(who, total)| *who == issuer && total.currency() == amount.currency())
                {
                    Some((_, total)) => {
                        *total = total
                            .checked_add(amount)
                            .expect("the aggregation key is the currency");
                    }
                    None => owed.push((issuer, amount)),
                }
            }
        }
        for (issuer, amount) in owed {
            ctx.say(
                self.kind,
                &[issuer, amount.currency().0],
                &[(0, Value::Num(amount.amount()))],
                true,
            );
        }
        for amount in stock(ctx.register(), ctx.instruments()) {
            ctx.say(
                self.stock_kind,
                &[amount.currency().0],
                &[(0, Value::Num(amount.amount()))],
                true,
            );
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
    fn the_money_stock_sums_issuer_liabilities_per_currency() {
        let usd = CurrencyCode::at(0);
        let eur = CurrencyCode::at(1);
        let stock = stock_from_liabilities([
            MoneyAmount::new(40.0, usd).unwrap(),
            MoneyAmount::new(60.0, usd).unwrap(),
            MoneyAmount::new(25.0, eur).unwrap(),
        ]);
        assert_eq!(
            stock,
            vec![
                MoneyAmount::new(100.0, usd).unwrap(),
                MoneyAmount::new(25.0, eur).unwrap()
            ]
        );
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
