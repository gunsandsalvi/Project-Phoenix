//! The wire: state changes ONLY by a numbered, two-sided instruction applied by settlement, all
//! legs atomic (delivery-versus-payment, XI-5), settlement is FINAL, and a fail is a recorded state
//! (Money D1, D4, Register C2, A-20).
//!
//! Measured on the full world, the TypeScript settlement costs **42 kernel reads per leg** over
//! **501,044 legs a period**, and one estate hand-over — a single instruction with 5,489 legs — cost
//! 231,250 reads (0g.26). Roughly four fifths of that is repetition inside one atomic instruction:
//! a line's currency read two and three times, a party's home money read once per equity bump. Here
//! those are looked up once per instruction because the instruction is the unit, and the register's
//! row is resolved once and carried.

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::instruments::Instruments;
use crate::journal::{Journal, Value};
use crate::parties::Parties;
use crate::register::Register;

/// Law 5: every flow has two sides, both legs, same pass, same period, same currency. A leg is one
/// side of one move; an instruction is the set of them that stand or fall together.
#[derive(Clone, Copy)]
pub enum Leg {
    /// Money moving between two accounts. `receipt` says WHAT this money is to the party receiving
    /// it, and it is not optional: a writer that cannot say what its money is has not finished
    /// writing the leg (0i.5, Appendix A).
    Money { from: PartyId, to: PartyId, ccy: CurrencyCode, instrument: InstrumentId, amount: f64, receipt: Receipt },
    /// Units of an instrument moving between two holders, at a price if it is a trade.
    Asset { from: PartyId, to: PartyId, instrument: InstrumentId, qty: f64, price_per_unit: Option<f64> },
    /// Goods B: a physical thing coming into existence. ONE side, because nobody is on the other
    /// end of a harvest — and what keeps it honest is the units identity, checked by a family.
    Create { party: PartyId, instrument: InstrumentId, qty: f64, cost_per_unit: f64 },
    /// **Money A1: an issuer creating its own money.** Not `Create`: a money account is a TOTAL and
    /// carries no lots (Money D2), and money created is the issuer's own LIABILITY rather than an
    /// asset it earned — a bank that booked the deposits it prints as income would be the closest
    /// thing to free money this engine could write.
    Mint { issuer: PartyId, ccy: CurrencyCode, money: InstrumentId, amount: f64 },
    /// Goods E4: and a thing leaving it. One side, for the same reason.
    Destroy { party: PartyId, instrument: InstrumentId, qty: f64, why: Gone },
    /// Register C3: a claim over units, which refuses their move rather than adjusting it.
    Pledge { holder: PartyId, instrument: InstrumentId, to: PartyId, qty: f64 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gone {
    Consumed,
    Perished,
    Scrapped,
}

/// 0i.5: what money IS to the party receiving it. Absent is not "unclassified": it is unwritten.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Receipt {
    Wage,
    Sale,
    Interest,
    Dividend,
    Transfer,
    Tax,
    Principal,
}

/// C1.b, Register C2: why the units moved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Trade,
    Payment,
    CorporateAction,
    Production,
    Settlement,
}

/// A-20: a fail is a RECORDED STATE and the module has to read it. Settlement is atomic, so on a
/// fail NOTHING moved — which is why the reason is returned rather than thrown away.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Settled,
    /// The payer had not got it and its bank would not lend (Money B3.a).
    ShortOfMoney,
    /// **The payer had it and its BANK could not settle across** (Money E1, Banks Capital C1.a,
    /// 21.2). It was `ShortOfMoney` too, which made two different failures one word: a customer
    /// that could not pay and a bank that could not deliver its customer's money read the same on
    /// the record, and the row that should have been the BANK's was nobody's.
    BankCouldNotSettle,
    /// The units are there and somebody else has a claim over them (Register C3).
    Encumbered,
    /// The holder has not got the units, and a short needs a borrow (Register C4).
    ShortOfUnits,
}

/// **HOW THE UNITS AND THE MONEY ARE TIED TOGETHER — and it is DECLARED, not inferred.**
///
/// XI-5, Money C3.a: delivery versus payment is that neither leg happens without the other. That
/// is the protection, and it is not free: it requires the money to be there at the same instant.
///
/// **Not everything settles that way, and this world had no way to say so.** A restructured bond
/// delivered for the old one, a collateral substitution, a distribution in kind, an estate handed
/// to probate: the units move and no money moves against them. In the TypeScript engine those are
/// an instruction with asset legs and no money leg — which on the wire is INDISTINGUISHABLE FROM A
/// MONEY LEG SOMEBODY FORGOT. Law 5 says a one-sided flow is a defect even when nothing fails, and
/// the check could not be written, because a legitimate free delivery and a dropped payment look
/// the same.
///
/// So the instruction says which it is and settlement refuses a mismatch. That is the whole point:
/// it turns "there is no money leg" from an absence into a STATEMENT somebody made and can be held
/// to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delivery {
    /// XI-5: units and money in the same instruction, so neither happens without the other.
    AgainstPayment,
    /// **FREE OF PAYMENT.** The units move and nothing moves against them here. The deliverer
    /// PERFORMS UNCONDITIONALLY: whatever it is owed is owed outside this instruction, by a promise
    /// that can fail, so it carries the other side's performance as an exposure it chose. That is
    /// the economic content of settling this way and the reason it is a different pathway rather
    /// than a convenience — a world that let a module deliver free without saying so would be
    /// hiding a real risk somebody took.
    Free,
    /// Nothing crossed between two parties: a payment, a thing made, a thing that perished. There
    /// is no delivery here to be versus anything.
    Nothing,
}

/// **What settlement is given to work on.**
///
/// The register and the journal are what it writes. The parties and the instruments are what the
/// PAYMENT SYSTEM has to read: who banks where, and which money each bank issues (Money D2). They
/// are part of settling rather than an argument some callers remember to pass, because a payment
/// between two banks' customers is not a payment the wire may decline to understand.
pub struct Settling<'a> {
    pub register: &'a mut Register,
    pub journal: &'a mut Journal,
    pub parties: &'a Parties,
    pub instruments: &'a Instruments,
}

/// **Money D2: the account a party pays out of and is paid into.** Its bank's money — or the money
/// it issues itself when it banks nowhere, which is what a central bank does.
///
/// ONE WRITER of the question (Law 4). Settlement asks it to know where a payment lands; a book asks
/// it to know what a buyer pays with; a participant asks it to know what it has. Three copies of
/// "which money is mine" would drift the day one of them was corrected.
///
/// `Missing` is missing: a party whose bank issues no money has no account, and that is not
/// instrument zero.
pub fn account_of(parties: &Parties, instruments: &Instruments, p: PartyId) -> Option<InstrumentId> {
    let bank = parties.bank_of(p);
    instruments.money_issued_by(if bank.some() { bank } else { p })
}

/// What a payment across two banks needs, once it is known to be one.
struct Across {
    payers_bank: PartyId,
    payees_bank: PartyId,
    payees_money: InstrumentId,
    reserves: InstrumentId,
}

/// **Whether this payment crosses two banks**, and what it takes if it does.
///
/// `None` is the ordinary case and covers three of them: the payee banks at the issuer already, the
/// payee IS the issuer (money coming home extinguishes the deposit), or the payee banks nowhere —
/// which is what a central bank does, and is why no special case names it.
///
/// It THROWS on the one thing that is a contract violation rather than an outcome: a bank that
/// issues no money is not a bank, and a payment to its customer cannot be told where to land.
fn across(
    parties: &Parties,
    instruments: &Instruments,
    to: PartyId,
    money: InstrumentId,
) -> Option<Across> {
    let payers_bank = instruments.issuer_of(money);
    let payees_bank = parties.bank_of(to);
    if !payees_bank.some() || payees_bank == payers_bank || to == payers_bank {
        return None;
    }
    let payees_money = match account_of(parties, instruments, to) {
        Some(m) => m,
        None => panic!(
            "Money D2: party {} banks at {}, which issues no money — a payment to it has nowhere to land",
            to.0, payees_bank.0
        ),
    };
    // The reserve line is **what the payee's bank itself settles in** — the money its own bank
    // issues, or the money it issues itself when it banks nowhere, which is exactly `account_of`.
    // It was `money_issued_by(bank_of(payees_bank))`, which is the same thing for a commercial bank
    // and WRONG for the central bank: a payment to the treasury, which banks AT the central bank,
    // asked what the central bank's own bank issues and there is no such party. Found by running
    // the assembled world at its real size, which is the first thing that ever paid one.
    let reserves = match account_of(parties, instruments, payees_bank) {
        Some(r) => r,
        None => panic!(
            "Money D2, 31 A1: bank {} settles in no money, so two banks have no way to settle between them",
            payees_bank.0
        ),
    };
    Some(Across { payers_bank, payees_bank, payees_money, reserves })
}

pub struct Instruction<'a> {
    pub legs: &'a [Leg],
    pub cause: Cause,
    /// What the writer says this is. Settlement checks it against the legs (`Settlement::settle`).
    pub delivery: Delivery,
}

impl<'a> Instruction<'a> {
    /// XI-5: the ordinary way. Units one way, money the other, together or not at all.
    pub fn against_payment(legs: &'a [Leg], cause: Cause) -> Self {
        Self { legs, cause, delivery: Delivery::AgainstPayment }
    }

    /// Free of payment: the deliverer performs and takes the other side on trust.
    pub fn free_of_payment(legs: &'a [Leg], cause: Cause) -> Self {
        Self { legs, cause, delivery: Delivery::Free }
    }

    /// A payment, a thing made, a thing that perished: nothing is delivered against anything.
    pub fn plain(legs: &'a [Leg], cause: Cause) -> Self {
        Self { legs, cause, delivery: Delivery::Nothing }
    }

    /// What the LEGS say this is, so the declaration can be held to them.
    fn shape(&self) -> Delivery {
        let mut delivers = false;
        let mut pays = false;
        for leg in self.legs {
            match leg {
                Leg::Asset { from, to, .. } if from != to => delivers = true,
                Leg::Money { from, to, .. } if from != to => pays = true,
                _ => {}
            }
        }
        match (delivers, pays) {
            (true, true) => Delivery::AgainstPayment,
            (true, false) => Delivery::Free,
            _ => Delivery::Nothing,
        }
    }
}

/// Money D1, D4: EVERY INSTRUCTION EVER APPLIED, numbered, in order, with its legs. **The wire is
/// the history** — nothing else stores what moved, and a reader that wants a total walks this
/// rather than keeping a second tally of it (Law 19).
///
/// The legs are one flat column and an instruction owns a counted slice of it, so walking a
/// period's legs is one contiguous run. The audit families that ask *why did this move* — flows,
/// units, the capital programme — are all walks of this, and in TypeScript each of them walked it
/// separately.
pub struct Settlement {
    outcomes: Vec<Outcome>,
    at_period: Vec<u32>,
    cause: Vec<Cause>,
    leg_at: Vec<u32>,
    leg_len: Vec<u32>,
    legs: Vec<Leg>,
    /// Audit C1: where each period's instructions begin and end, written as they arrive.
    by_period: Vec<(u32, u32, u32)>,
    /// Every free delivery: who performed, who was trusted, and when. It is a READ of what the
    /// wire did and never a second history — the legs are still the record; this is the index a
    /// reader asking *who is exposed and to whom* would otherwise have to walk them to build.
    delivered_free: Vec<(PartyId, PartyId, u32)>,

}

impl Default for Settlement {
    fn default() -> Self {
        Self::new()
    }
}

impl Settlement {
    pub fn new() -> Self {
        Self {
            outcomes: Vec::new(),
            at_period: Vec::new(),
            cause: Vec::new(),
            leg_at: Vec::new(),
            leg_len: Vec::new(),
            legs: Vec::new(),
            by_period: Vec::new(),
            delivered_free: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }

    pub fn outcome_of(&self, n: usize) -> Outcome {
        self.outcomes[n]
    }

    pub fn cause_of(&self, n: usize) -> Cause {
        self.cause[n]
    }

    pub fn period_of(&self, n: usize) -> u32 {
        self.at_period[n]
    }

    /// The legs of one instruction, as a contiguous slice. Nothing is copied to read them.
    pub fn legs_of(&self, n: usize) -> &[Leg] {
        let at = self.leg_at[n] as usize;
        let len = self.leg_len[n] as usize;
        &self.legs[at..at + len]
    }

    /// Who delivered free of payment, to whom, and when — the exposures this world is carrying
    /// because somebody performed before being paid.
    pub fn delivered_free(&self) -> &[(PartyId, PartyId, u32)] {
        &self.delivered_free
    }

    /// Audit C1: this period's instructions, without walking the history.
    pub fn in_period(&self, period: u32) -> std::ops::Range<usize> {
        for &(p, from, to) in &self.by_period {
            if p == period {
                return from as usize..to as usize;
            }
        }
        0..0
    }


    /// XI-5: ALL LEGS OR NONE. Every leg is checked before any is applied, so a refusal leaves the
    /// world exactly as it was — which is what makes delivery-versus-payment true rather than
    /// hoped for, and what lets a caller read the reason and do something else.
    pub fn settle(
        &mut self,
        ins: &Instruction<'_>,
        period: u32,
        on: &mut Settling<'_>,
        settled_kind: u32,
        failed_kind: u32,
    ) -> Outcome {
        let Settling { register: reg, journal, parties, instruments } = on;
        let (parties, instruments) = (*parties, *instruments);
        // The declaration, against the legs. A writer that says one thing and sends another has
        // made a mistake rather than met an outcome, so this THROWS where a short balance is
        // returned: it is a contract violation at the site (`docs/ARCHITECTURE.md` error
        // discipline), and it is what makes "free of payment" a statement instead of an absence.
        let shape = ins.shape();
        assert!(
            shape == ins.delivery,
            "XI-5: this instruction is declared {:?} and its legs are {shape:?} — \
             a delivery with no money against it is either FREE OF PAYMENT or a payment somebody \
             forgot, and the wire cannot tell which unless the writer says",
            ins.delivery
        );

        // The pre-check. Law 6: nothing is clamped here — a leg that cannot happen is refused.

        for leg in ins.legs {
            match *leg {
                Leg::Money { from, to, instrument, amount, .. } => {
                    let row = reg.row(from, instrument);
                    if reg.quantity(row) < amount {
                        return self.record(Outcome::ShortOfMoney, from, ins, period, journal, failed_kind);
                    }
                    // Money D2: and the payer's BANK needs the reserves to settle it across. A bank
                    // that cannot is a bank whose customers' payments do not go through — which is
                    // what a liquidity problem IS, and it is refused rather than overdrawn (Appendix
                    // B: no silent overdraft; the lender of last resort is a mechanism, not a default).
                    if let Some(a) = across(parties, instruments, to, instrument) {
                        let at = reg.row(a.payers_bank, a.reserves);
                        if reg.quantity(at) < amount {
                            return self.record(Outcome::BankCouldNotSettle, a.payers_bank, ins, period, journal, failed_kind);
                        }
                    }
                }
                Leg::Asset { from, instrument, qty, .. } => {
                    let row = reg.row(from, instrument);
                    if !row.some() {
                        return self.record(Outcome::ShortOfUnits, from, ins, period, journal, failed_kind);
                    }
                    if reg.quantity(row) < qty {
                        return self.record(Outcome::ShortOfUnits, from, ins, period, journal, failed_kind);
                    }
                    if reg.free(row) < qty {
                        return self.record(Outcome::Encumbered, from, ins, period, journal, failed_kind);
                    }
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    if reg.free(row) < qty {
                        return self.record(Outcome::ShortOfUnits, party, ins, period, journal, failed_kind);
                    }
                }
                Leg::Create { .. } | Leg::Mint { .. } | Leg::Pledge { .. } => {}
            }
        }
        // The application. Nothing here can fail: the pre-check is what made that true.

        for leg in ins.legs {
            match *leg {
                Leg::Money { from, to, instrument, amount, .. } => {
                    // **THE INTERBANK LEG** (Money D2, XI-9, worklist 1). A deposit is a claim on the
                    // bank that ISSUED it. When the payee banks somewhere else, the payer's bank's
                    // deposit is extinguished, the payee's bank's deposit is created, and RESERVES
                    // move between the two at the central bank. Without it, a payee simply came to
                    // hold the payer's bank's money — which settles, and means no bank ever loses
                    // reserves to another, so no bank's liquidity is ever tested by its customers'
                    // payments and the money market has nothing to meet about.
                    reg.money_delta(from, instrument, -amount);
                    match across(parties, instruments, to, instrument) {
                        None => {
                            reg.money_delta(to, instrument, amount);
                        }
                        Some(Across { payers_bank, payees_bank, payees_money, reserves }) => {
                            reg.money_delta(to, payees_money, amount);
                            reg.money_delta(payers_bank, reserves, -amount);
                            reg.money_delta(payees_bank, reserves, amount);
                        }
                    }
                }
                Leg::Asset { from, to, instrument, qty, price_per_unit } => {
                    let row = reg.row(from, instrument);
                    let drawn = reg.debit(row, qty);
                    // Register D2: what the units cost goes with them where it is a transfer, and
                    // the price is the basis where a market struck one (C2.a).
                    match price_per_unit {
                        Some(price) => {
                            reg.credit(to, instrument, qty, price, period);
                        }
                        None => {
                            for d in &drawn {
                                reg.credit(to, instrument, d.qty, d.basis_per_unit, period);
                            }
                        }
                    }
                }
                Leg::Create { party, instrument, qty, cost_per_unit } => {
                    reg.credit(party, instrument, qty, cost_per_unit, period);
                }
                // Money A1, D2: the issuer's own money, as a TOTAL, and NO equity — what it created
                // is what it owes, and `Instruments::owed_by` reads that from issued against held.
                Leg::Mint { issuer, money, amount, .. } => {
                    reg.money_delta(issuer, money, amount);
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    let drawn = reg.debit(row, qty);
                    // Goods E4: what perished cost something, and the loss is an EVENT on the
                    // account rather than a number that quietly stops existing (XI-1).
                    let _ = drawn;
                }
                Leg::Pledge { holder, instrument, to, qty } => {
                    reg.pledge(holder, instrument, to, qty);
                }
            }
        }
        // A free delivery is a risk somebody took, so the record NAMES who took it. Without this
        // the world would carry an exposure nobody could see, which is the thing Law 1 is about.
        if ins.delivery == Delivery::Free {
            for leg in ins.legs {
                if let Leg::Asset { from, to, .. } = *leg {
                    if from != to {
                        self.delivered_free.push((from, to, period));
                    }
                }
            }
        }
        self.record(Outcome::Settled, PartyId::NONE, ins, period, journal, settled_kind)

    }

    fn record(
        &mut self,
        outcome: Outcome,
        // A-20, Money E1: WHOSE failure it is. It was nobody's — `say` was called with no subjects
        // at all — so a fail was a recorded state nobody could find by looking for their own.
        on: PartyId,
        ins: &Instruction<'_>,
        period: u32,
        journal: &mut Journal,
        kind: u32,
    ) -> Outcome {
        let n = self.outcomes.len() as u32;
        self.outcomes.push(outcome);
        self.at_period.push(period);
        self.cause.push(ins.cause);
        self.leg_at.push(self.legs.len() as u32);
        self.leg_len.push(ins.legs.len() as u32);
        self.legs.extend_from_slice(ins.legs);
        match self.by_period.last_mut() {
            Some(last) if last.0 == period => last.2 = n + 1,
            _ => self.by_period.push((period, n, n + 1)),
        }
        let subjects: &[u32] = if on.some() { &[on.0] } else { &[] };
        journal.say(period, 0, kind, subjects, &[(0, Value::Num(ins.legs.len() as f64))], true);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
    use crate::instruments::Class;
    use crate::parties::Representation;

    /// A small world these tests settle in. **`party(9)` is the bank**, it issues instrument 0, and
    /// everybody banks at it — so no payment below crosses two banks. The one that does is its own
    /// test (`a_payment_across_two_banks_moves_reserves_between_them`).
    fn world() -> (Register, Journal, Parties, Instruments, Settlement, u32, u32) {
        let mut j = Journal::new();
        let ok = j.kinds.declare("instruction.settled");
        let no = j.kinds.declare("instruction.failed");
        let bank = PartyId::at(9);
        let mut p = Parties::new();
        for _ in 0..16 {
            p.add(0, RegionId::at(0), bank, Representation::Named, 1, 0);
        }
        let mut i = Instruments::new();
        i.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        for _ in 1..16 {
            i.issue(PartyId::at(1), CurrencyCode::at(0), Class::Good, UnitId::at(0), None, None);
        }
        (Register::new(), j, p, i, Settlement::new(), ok, no)
    }

    /// 22b.7a: what a party is worth, READ from what it holds against what it owes. There is no pot.
    fn worth(reg: &Register, ins: &Instruments, p: PartyId) -> f64 {
        // 21.110: settlement moves instruments, and no instruction in these cases makes a claim on an
        // estate — so an empty book of them is what this world has, not a corner being avoided.
        crate::instruments::equity(p, reg, ins, &crate::stores::Claims::new())
    }

    /// The four stores settlement works on, gathered for a call.
    fn on<'a>(
        register: &'a mut Register,
        journal: &'a mut Journal,
        parties: &'a Parties,
        instruments: &'a Instruments,
    ) -> Settling<'a> {
        Settling { register, journal, parties, instruments }
    }

    #[test]
    fn a_refused_instruction_moves_nothing_at_all() {
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 3.0, 1);
        // Delivery-versus-payment: the money is there, the shares are not enough.
        let legs = [
            Leg::Money { from: a, to: b, ccy: CurrencyCode::at(0), instrument: cash, amount: 500.0, receipt: Receipt::Sale },
            Leg::Asset { from: b, to: a, instrument: share, qty: 99.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        assert_eq!(out, Outcome::ShortOfUnits);
        // NOTHING moved: the money is where it was and so are the shares.
        assert_eq!(reg.quantity(reg.row(a, cash)), 500.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 10.0);
        // And the fail is a RECORDED state, not a silence.
        assert_eq!(j.in_period(1).len(), 1);
    }

    #[test]
    fn both_legs_of_a_trade_move_in_the_same_pass() {
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 3.0, 1);
        let legs = [
            Leg::Money { from: a, to: b, ccy: CurrencyCode::at(0), instrument: cash, amount: 50.0, receipt: Receipt::Sale },
            Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(a, cash)), 450.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 50.0);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 0.0);
        // C2.a: the buyer's lot carries what the market struck, not what the seller paid.
        assert_eq!(reg.lots(reg.row(a, share))[0].basis_per_unit, 5.0);
    }

    #[test]
    fn encumbered_units_refuse_the_whole_instruction() {
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let lender = PartyId::at(2);
        let share = InstrumentId::at(1);
        reg.credit(a, share, 10.0, 3.0, 1);
        reg.pledge(a, share, lender, 8.0);
        let legs = [Leg::Asset { from: a, to: b, instrument: share, qty: 5.0, price_per_unit: None }];
        let out = s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
    }

    #[test]
    fn free_of_payment_delivers_and_records_who_was_trusted() {
        // A restructured bond handed over for the old one: the units move and nothing moves
        // against them. The deliverer performs FIRST and carries the other side's performance.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let issuer = PartyId::at(0);
        let holder = PartyId::at(1);
        let new_bond = InstrumentId::at(2);
        reg.credit(issuer, new_bond, 1_000.0, 1.0, 1);
        let legs = [Leg::Asset {
            from: issuer,
            to: holder,
            instrument: new_bond,
            qty: 1_000.0,
            price_per_unit: None,
        }];
        let out = s.settle(
            &Instruction::free_of_payment(&legs, Cause::CorporateAction),
            3,
            &mut on(&mut reg, &mut j, &ps, &ins),
            ok,
            no,
        );
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(holder, new_bond)), 1_000.0);
        assert_eq!(reg.quantity(reg.row(issuer, new_bond)), 0.0);
        // And the exposure is VISIBLE: who performed, who was trusted, and when.
        assert_eq!(s.delivered_free(), &[(issuer, holder, 3)]);
    }

    #[test]
    #[should_panic(expected = "a payment somebody forgot")]
    fn a_delivery_with_money_against_it_is_not_free_of_payment() {
        // Declaring `Free` while sending a money leg is the writer contradicting itself, and it is
        // refused at the site rather than settled and reported later.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 1.0, 1);
        let legs = [
            Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
            Leg::Money { from: a, to: b, ccy: CurrencyCode::at(0), instrument: cash, amount: 50.0, receipt: Receipt::Sale },
        ];
        s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
    }

    #[test]
    #[should_panic(expected = "a payment somebody forgot")]
    fn a_trade_that_lost_its_money_leg_is_caught_instead_of_settling_free() {
        // **The defect this pathway exists to make findable.** Before it, an asset-only instruction
        // was indistinguishable from one whose money leg was dropped: both settled, and Law 5's
        // "a one-sided flow is a defect even when nothing fails" could not be checked.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let share = InstrumentId::at(1);
        reg.credit(b, share, 10.0, 1.0, 1);
        let legs = [Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) }];
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
    }

    #[test]
    fn a_free_delivery_is_still_all_legs_or_none() {
        // XI-5 holds for a basket handed over free: one line short and NOTHING moves, because a
        // half-delivered restructuring is not a restructuring.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let issuer = PartyId::at(0);
        let holder = PartyId::at(1);
        let one = InstrumentId::at(2);
        let two = InstrumentId::at(3);
        reg.credit(issuer, one, 100.0, 1.0, 1);
        reg.credit(issuer, two, 5.0, 1.0, 1);
        let legs = [
            Leg::Asset { from: issuer, to: holder, instrument: one, qty: 100.0, price_per_unit: None },
            Leg::Asset { from: issuer, to: holder, instrument: two, qty: 50.0, price_per_unit: None },
        ];
        let out = s.settle(
            &Instruction::free_of_payment(&legs, Cause::CorporateAction),
            3,
            &mut on(&mut reg, &mut j, &ps, &ins),
            ok,
            no,
        );
        assert_eq!(out, Outcome::ShortOfUnits);
        assert_eq!(reg.quantity(reg.row(issuer, one)), 100.0, "the first line did not move either");
        assert!(s.delivered_free().is_empty(), "nothing was delivered, so nobody was trusted");
    }

    #[test]
    fn a_payment_moves_money_and_makes_nobody_richer() {

        // Law 5: every flow has two sides, so what one account loses another gains and the world's
        // equity is unchanged. A payment that moved the total would be money appearing from nowhere.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        reg.money_delta(a, cash, 500.0);
        let before = worth(&reg, &ins, a) + worth(&reg, &ins, b);
        let bank_before = worth(&reg, &ins, PartyId::at(9));
        let legs = [Leg::Money {
            from: a,
            to: b,
            ccy: CurrencyCode::at(0),
            instrument: cash,
            amount: 120.0,
            receipt: Receipt::Wage,
        }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        // A LEVEL, not a running delta: what each is worth is read from its book every time.
        assert_eq!(worth(&reg, &ins, a), 500.0 - 120.0);
        assert_eq!(worth(&reg, &ins, b), 120.0);
        assert_eq!(worth(&reg, &ins, a) + worth(&reg, &ins, b), before);
        // And the BANK is no better or worse off for having moved it: it owes 120 less to one
        // depositor and 120 more to the other, which nets to nothing. That its LEVEL is negative is
        // this fixture being a fixture — the deposit was put on the register rather than minted
        // against anything, so the bank owes money it was never paid for. The read says so, which is
        // the side the old pot never saw (22b.7a).
        assert_eq!(worth(&reg, &ins, PartyId::at(9)), bank_before);
    }

    #[test]
    fn a_sale_books_the_gain_and_never_plugs_it() {
        // Register D2: the seller gives up what the units cost it and takes in what it was paid.
        // The difference is a GAIN and it is on the account, not a residual with no holder.
        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(b, cash, 1_000.0);
        reg.credit(a, share, 10.0, 3.0, 1);
        let seller_before = worth(&reg, &ins, a);
        let buyer_before = worth(&reg, &ins, b);
        let legs = [
            Leg::Asset { from: a, to: b, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
            Leg::Money { from: b, to: a, ccy: CurrencyCode::at(0), instrument: cash, amount: 50.0, receipt: Receipt::Sale },
        ];
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        // It gave up 30 of book and took in 50: it is 20 better off, and nothing was invented.
        assert_eq!(seller_before, 30.0, "ten shares that cost three");
        assert_eq!(worth(&reg, &ins, a), 50.0);
        // The buyer paid 50 and holds 50 of stock: unchanged, which is what a purchase is. Its own
        // share line is NOT a liability to itself — a share is the residual, not a promise (Law 8).
        assert_eq!(worth(&reg, &ins, b), buyer_before);
    }

    #[test]
    fn a_transfer_carries_the_basis_and_a_trade_does_not() {

        let (mut reg, mut j, ps, ins, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let good = InstrumentId::at(3);
        reg.credit(a, good, 10.0, 7.0, 1);
        let legs = [Leg::Asset { from: a, to: b, instrument: good, qty: 10.0, price_per_unit: None }];
        s.settle(&Instruction::free_of_payment(&legs, Cause::CorporateAction), 2, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        // Register D2: what it cost went with it.
        assert_eq!(reg.lots(reg.row(b, good))[0].basis_per_unit, 7.0);
    }

    /// Two banks, and a payment between their customers. **This is the leg the wire did not have.**
    fn two_banks() -> (Register, Journal, Parties, Instruments, Settlement, u32, u32, [PartyId; 5], [InstrumentId; 3]) {
        let mut j = Journal::new();
        let ok = j.kinds.declare("instruction.settled");
        let no = j.kinds.declare("instruction.failed");
        let mut p = Parties::new();
        let region = RegionId::at(0);
        // 31 A1: the central bank banks nowhere, and both banks bank at it.
        let cb = p.add(0, region, PartyId::NONE, Representation::Named, 1, 0);
        let one = p.add(0, region, cb, Representation::Named, 1, 0);
        let two = p.add(0, region, cb, Representation::Named, 1, 0);
        let payer = p.add(0, region, one, Representation::Named, 1, 0);
        let payee = p.add(0, region, two, Representation::Named, 1, 0);
        let mut i = Instruments::new();
        let unit = UnitId::at(0);
        let ccy = CurrencyCode::at(0);
        let reserves = i.issue(cb, ccy, Class::Money, unit, None, None);
        let ones = i.issue(one, ccy, Class::Money, unit, None, None);
        let twos = i.issue(two, ccy, Class::Money, unit, None, None);
        (Register::new(), j, p, i, Settlement::new(), ok, no, [cb, one, two, payer, payee], [reserves, ones, twos])
    }

    #[test]
    fn a_payment_across_two_banks_moves_reserves_between_them() {
        // Money D2, worklist 1: a deposit is a claim on the bank that ISSUED it, so a payee banking
        // elsewhere cannot simply come to hold the payer's bank's money. The payer's bank's deposit is
        // extinguished, the payee's bank's is created, and RESERVES move between the two.
        let (mut reg, mut j, ps, ins, mut s, ok, no, who, lines) = two_banks();
        let (one, two, payer, payee) = (who[1], who[2], who[3], who[4]);
        let (reserves, ones, twos) = (lines[0], lines[1], lines[2]);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);

        let legs = [Leg::Money { from: payer, to: payee, ccy: CurrencyCode::at(0), instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        assert_eq!(out, Outcome::Settled);

        // The payer's deposit at its own bank fell; the payee holds ITS OWN bank's money, not the
        // payer's — which is the whole point, because a deposit names who owes it.
        assert_eq!(reg.quantity(reg.row(payer, ones)), 200.0);
        assert_eq!(reg.quantity(reg.row(payee, twos)), 300.0);
        assert_eq!(reg.quantity(reg.row(payee, ones)), 0.0);
        // And the reserves moved: bank one is 300 poorer at the central bank and bank two 300 richer.
        assert_eq!(reg.quantity(reg.row(one, reserves)), 500.0);
        assert_eq!(reg.quantity(reg.row(two, reserves)), 300.0);
    }

    #[test]
    fn a_bank_without_the_reserves_cannot_settle_its_customer_out() {
        // **What the leg makes possible: a bank's liquidity is tested by its customers' payments.**
        // Without it no bank ever lost a reserve to another and this could not happen at all. It is
        // REFUSED, not overdrawn (Appendix B: no silent overdraft — the lender of last resort is a
        // mechanism somebody builds, never a default the wire helps itself to).
        let (mut reg, mut j, ps, ins, mut s, ok, no, who, lines) = two_banks();
        let (payer, payee) = (who[3], who[4]);
        let ones = lines[1];
        reg.money_delta(payer, ones, 500.0);
        // Its bank has none: the deposit is there and the settlement asset is not.
        let legs = [Leg::Money { from: payer, to: payee, ccy: CurrencyCode::at(0), instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        // **21.2, Money E1: it is the BANK's failure and the record says so.** It was `ShortOfMoney`
        // — the same word as a customer that had not got it — so a bank that could not deliver its
        // customer's money and a customer that could not pay read identically, and the row that
        // should have been the bank's was nobody's.
        assert_eq!(out, Outcome::BankCouldNotSettle);
        assert_ne!(out, Outcome::ShortOfMoney, "the payer HAD it; its bank could not settle it out");
        // A-20: a fail is a recorded state, and it is recorded ON somebody. The payer's bank is
        // `one`, and a reader looking for its own failures finds this one.
        let failed: Vec<u32> = j.in_period(1).filter(|r| j.kind_of(*r) == no).collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(j.subjects_of(failed[0]), &[who[1].0]);
        // XI-5: and nothing moved.
        assert_eq!(reg.quantity(reg.row(payer, ones)), 500.0);
    }

    #[test]
    fn a_payment_within_one_bank_never_touches_reserves() {
        // The ordinary case, and it has to stay ordinary: two customers of one bank settle on that
        // bank's books and the central bank never hears about it.
        let (mut reg, mut j, mut ps, ins, mut s, ok, no, who, lines) = two_banks();
        let one = who[1];
        let (reserves, ones) = (lines[0], lines[1]);
        let payer = who[3];
        let alongside = ps.add(0, RegionId::at(0), one, Representation::Named, 1, 0);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);
        let legs = [Leg::Money { from: payer, to: alongside, ccy: CurrencyCode::at(0), instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins), ok, no);
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(alongside, ones)), 300.0);
        assert_eq!(reg.quantity(reg.row(one, reserves)), 800.0, "nothing left the bank");
    }
}
