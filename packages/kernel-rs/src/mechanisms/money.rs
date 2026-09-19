//! MONEY, AND WHAT HAPPENS WHEN AN ACCOUNT IS SHORT OF IT.
//!
//! @spec Money B1, Money B3.a, Money B3.b, Money B3.c, Money D2, Central Bank D3, Central Bank E2, Appendix B, Law 5

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::module::ParticipantView;

/// What an issuer answers when the account it issues into is short.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Overdraft {
    /// It lends. The money is ISSUED and the borrower owes it back on `owes`, at `per_annum` — two
    /// sides in the same pass, so nothing is created that nobody holds.
    Lend {
        /// The line the borrower's debt is written on. The issuer's module owns it; the kernel is
        /// told which, never asked to invent one.
        owes: InstrumentId,
        per_annum: f64,
    },
    /// It refuses, and the refusal is the record. The payment does not settle, which is an outcome
    /// the payer reads and does something about — not a crash and not a hole.
    Refuse,
}

/// The door the kernel asks. One per party KIND, so a bank and a central bank answer differently
/// because they are different kinds, and no mechanism branches on which.
pub trait Issuer {
    /// Whose accounts this issuer answers for.
    fn party_kind(&self) -> u32;

    /// The bank lends only to the room its own capital supports, and refuses past it. The view is
    /// the BORROWER's, because what is being judged is the borrower — and the issuer's own room is a
    /// fact about the issuer that its module holds.
    fn overdraft(
        &self,
        borrower: &ParticipantView<'_>,
        short_by: f64,
        ccy: CurrencyCode,
    ) -> Overdraft;
}

/// THERE IS NO CENTRAL-BANK OVERDRAFT FOR THE TREASURY. A treasury that has not funded itself cannot
/// spend, and an issuer that answered otherwise would reverse the causation the whole of Part XII
/// turns on.
pub struct NoOverdraftForTheTreasury {
    pub treasury_kind: u32,
}

impl Issuer for NoOverdraftForTheTreasury {
    fn party_kind(&self) -> u32 {
        self.treasury_kind
    }
    fn overdraft(&self, _b: &ParticipantView<'_>, _short: f64, _c: CurrencyCode) -> Overdraft {
        Overdraft::Refuse
    }
}

/// Every issuer this world has, asked by the kind whose accounts they issue. A payer whose bank is
/// of a kind nobody answers for is not refused quietly — it is a world that was assembled wrong, and
/// the read says so.
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

    /// Ask the issuer of this borrower's bank. Missing is missing — a kind nobody answers for has no
    /// answer, and the caller may not read that as a refusal.
    pub fn ask(
        &self,
        bank_kind: u32,
        borrower: &ParticipantView<'_>,
        short_by: f64,
        ccy: CurrencyCode,
    ) -> Option<Overdraft> {
        assert!(short_by > 0.0, "Money B3.a: an account short by {short_by} is not overdrawn");
        self.by_kind
            .iter()
            .find(|(k, _)| *k == bank_kind)
            .map(|(_, i)| i.overdraft(borrower, short_by, ccy))
    }
}

/// What an overdraft BECOMES on the wire. The money is issued to the borrower and the borrower's
/// debt is written on the issuer's book — two legs, same pass, same period, so the money that paid
/// has a named creditor from the instant it exists.
pub fn as_legs(
    borrower: PartyId,
    bank: PartyId,
    money: InstrumentId,
    short_by: f64,
    lent: Overdraft,
) -> Option<[crate::ledger::Leg; 2]> {
    match lent {
        Overdraft::Refuse => None,
        Overdraft::Lend { owes, .. } => Some([
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
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::params::Params;
    use crate::prices::Prints;
    use crate::register::Register;

    const BANK: u32 = 3;
    const TREASURY: u32 = 4;

    /// A bank that lends to the room its capital supports and refuses past it.
    struct ABankWithRoom {
        room: f64,
        owes: InstrumentId,
    }
    impl Issuer for ABankWithRoom {
        fn party_kind(&self) -> u32 {
            BANK
        }
        fn overdraft(&self, _b: &ParticipantView<'_>, short_by: f64, _c: CurrencyCode) -> Overdraft {
            if short_by > self.room {
                // This is not a cap on the loan. It is a refusal of it — the bank does not lend a
                // smaller amount nobody asked for, it says no and the payment fails.
                return Overdraft::Refuse;
            }
            Overdraft::Lend { owes: self.owes, per_annum: 0.09 }
        }
    }

    fn view_of<'a>(
        who: PartyId,
        r: &'a Register,
        p: &'a Prints,
        j: &'a Journal,
        m: &'a Params,
    ) -> ParticipantView<'a> {
        ParticipantView::of(who, r, p, j, m, 1, None)
    }

    #[test]
    fn an_overdraft_is_a_loan_with_two_sides_and_never_a_silent_negative() {
        let (r, p, j, m) = (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0));
        let mut issuers = Issuers::new();
        issuers.declare(Box::new(ABankWithRoom { room: 1_000.0, owes: InstrumentId::at(9) }));
        let borrower = PartyId::at(1);
        let bank = PartyId::at(0);
        let view = view_of(borrower, &r, &p, &j, &m);

        let answer = issuers.ask(BANK, &view, 400.0, CurrencyCode::at(0)).expect("a bank answers");
        let legs = as_legs(borrower, bank, InstrumentId::at(0), 400.0, answer)
            .expect("it lent, so there are legs");
        // The money came from the bank AND the bank holds the claim, in the same pass.
        match legs[0] {
            crate::ledger::Leg::Money { from, to, amount, .. } => {
                assert_eq!(from, bank);
                assert_eq!(to, borrower);
                assert_eq!(amount, 400.0);
            }
            ref other => panic!("{:?}", std::mem::discriminant(other)),
        }
        match legs[1] {
            crate::ledger::Leg::Create { party, qty, .. } => {
                assert_eq!(party, bank, "the bank holds what it is owed");
                assert_eq!(qty, 400.0);
            }
            ref other => panic!("{:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn a_bank_refuses_past_its_room_and_the_refusal_is_the_record() {
        let (r, p, j, m) = (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0));
        let mut issuers = Issuers::new();
        issuers.declare(Box::new(ABankWithRoom { room: 1_000.0, owes: InstrumentId::at(9) }));
        let view = view_of(PartyId::at(1), &r, &p, &j, &m);
        let answer = issuers.ask(BANK, &view, 5_000.0, CurrencyCode::at(0)).unwrap();
        assert_eq!(answer, Overdraft::Refuse);
        // Refused means nothing is written — not a smaller loan nobody asked for.
        assert!(as_legs(PartyId::at(1), PartyId::at(0), InstrumentId::at(0), 5_000.0, answer).is_none());
    }

    #[test]
    fn the_treasury_has_no_central_bank_overdraft_and_it_is_a_refusal_in_the_type() {
        let (r, p, j, m) = (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0));
        let mut issuers = Issuers::new();
        issuers.declare(Box::new(NoOverdraftForTheTreasury { treasury_kind: TREASURY }));
        let view = view_of(PartyId::at(1), &r, &p, &j, &m);
        // However small the shortfall, there is no advance.
        assert_eq!(issuers.ask(TREASURY, &view, 1.0, CurrencyCode::at(0)).unwrap(), Overdraft::Refuse);
        assert_eq!(
            issuers.ask(TREASURY, &view, 1e12, CurrencyCode::at(0)).unwrap(),
            Overdraft::Refuse
        );
    }

    #[test]
    fn a_kind_nobody_answers_for_has_no_answer_and_is_not_a_refusal() {
        let (r, p, j, m) = (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0));
        let issuers = Issuers::new();
        let view = view_of(PartyId::at(1), &r, &p, &j, &m);
        // Missing is missing. A world assembled without an issuer for a kind has a hole in it, and
        // reading that as "refused" would hide the hole.
        assert!(issuers.ask(BANK, &view, 10.0, CurrencyCode::at(0)).is_none());
    }

    #[test]
    #[should_panic(expected = "two issuers answer for party kind")]
    fn one_kind_has_one_issuer() {
        let mut issuers = Issuers::new();
        issuers.declare(Box::new(ABankWithRoom { room: 1.0, owes: InstrumentId::at(9) }));
        issuers.declare(Box::new(ABankWithRoom { room: 2.0, owes: InstrumentId::at(9) }));
    }

    #[test]
    #[should_panic(expected = "is not overdrawn")]
    fn an_account_that_is_not_short_is_not_asked_about() {
        let (r, p, j, m) = (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0));
        let issuers = Issuers::new();
        let view = view_of(PartyId::at(1), &r, &p, &j, &m);
        issuers.ask(BANK, &view, 0.0, CurrencyCode::at(0));
    }
}
