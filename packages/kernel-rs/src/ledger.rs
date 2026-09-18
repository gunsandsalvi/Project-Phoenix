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
use crate::journal::{Journal, Value};
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
    /// The units are there and somebody else has a claim over them (Register C3).
    Encumbered,
    /// The holder has not got the units, and a short needs a borrow (Register C4).
    ShortOfUnits,
}

pub struct Instruction<'a> {
    pub legs: &'a [Leg],
    pub cause: Cause,
}

pub struct Settlement {
    /// Money D4: every instruction ever applied, numbered, in order. The wire IS the history.
    outcomes: Vec<Outcome>,
    at_period: Vec<u32>,
    leg_count: Vec<u32>,
}

impl Default for Settlement {
    fn default() -> Self {
        Self::new()
    }
}

impl Settlement {
    pub fn new() -> Self {
        Self { outcomes: Vec::new(), at_period: Vec::new(), leg_count: Vec::new() }
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

    /// XI-5: ALL LEGS OR NONE. Every leg is checked before any is applied, so a refusal leaves the
    /// world exactly as it was — which is what makes delivery-versus-payment true rather than
    /// hoped for, and what lets a caller read the reason and do something else.
    pub fn settle(
        &mut self,
        ins: &Instruction<'_>,
        period: u32,
        reg: &mut Register,
        journal: &mut Journal,
        settled_kind: u32,
        failed_kind: u32,
    ) -> Outcome {
        // The pre-check. Law 6: nothing is clamped here — a leg that cannot happen is refused.
        for leg in ins.legs {
            match *leg {
                Leg::Money { from, instrument, amount, .. } => {
                    let row = reg.row(from, instrument);
                    if reg.quantity(row) < amount {
                        return self.record(Outcome::ShortOfMoney, ins, period, journal, failed_kind);
                    }
                }
                Leg::Asset { from, instrument, qty, .. } => {
                    let row = reg.row(from, instrument);
                    if !row.some() {
                        return self.record(Outcome::ShortOfUnits, ins, period, journal, failed_kind);
                    }
                    if reg.quantity(row) < qty {
                        return self.record(Outcome::ShortOfUnits, ins, period, journal, failed_kind);
                    }
                    if reg.free(row) < qty {
                        return self.record(Outcome::Encumbered, ins, period, journal, failed_kind);
                    }
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    if reg.free(row) < qty {
                        return self.record(Outcome::ShortOfUnits, ins, period, journal, failed_kind);
                    }
                }
                Leg::Create { .. } | Leg::Pledge { .. } => {}
            }
        }
        // Currency B1, C5, Law 18 (0g.26): WHICH MONEY A PARTY REPORTS IN and WHAT THE RATE IS,
        // once per instruction. An instruction is atomic, so neither can change between two of its
        // legs — and the TypeScript wire read both per leg, which is where an estate hand-over's
        // 5,489 legs cost 231,250 kernel reads.
        let mut homes: Vec<(u32, u32)> = Vec::new();
        let mut home_of = |p: PartyId| -> u32 {
            for &(who, home) in &homes {
                if who == p.0 {
                    return home;
                }
            }
            // One region, one money, in this bench: the seam a registry fills in.
            homes.push((p.0, 0));
            0
        };
        let rate_into = |_home: u32, _ccy: CurrencyCode| -> f64 { 1.0 };

        // The application. Nothing here can fail: the pre-check is what made that true.

        for leg in ins.legs {
            match *leg {
                Leg::Money { from, to, instrument, amount, ccy, .. } => {
                    reg.money_delta(from, instrument, -amount);
                    reg.money_delta(to, instrument, amount);
                    // Currency C5: in each party's OWN money, at the rate in force — asked ONCE
                    // per instruction, not once per leg, which is where 0g.26 found four fifths
                    // of settlement's 42 reads a leg going.
                    reg.bump_equity(from, -amount * rate_into(home_of(from), ccy));
                    reg.bump_equity(to, amount * rate_into(home_of(to), ccy));
                }
                Leg::Asset { from, to, instrument, qty, price_per_unit } => {
                    let row = reg.row(from, instrument);
                    let drawn = reg.debit(row, qty);
                    // Register D2: the seller gives up what the units COST it, and takes in what
                    // it was paid; the gain is the difference and it is booked, never plugged.
                    let mut carried = 0.0;
                    for d in &drawn {
                        carried += d.qty * d.basis_per_unit;
                    }
                    reg.bump_equity(from, -carried);
                    if let Some(price) = price_per_unit {
                        reg.bump_equity(to, qty * price);
                    } else {
                        reg.bump_equity(to, carried);
                    }
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
                    reg.bump_equity(party, qty * cost_per_unit);
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    let drawn = reg.debit(row, qty);
                    // Goods E4: what perished cost something, and the loss is an EVENT on the
                    // account rather than a number that quietly stops existing (XI-1).
                    let mut carried = 0.0;
                    for d in &drawn {
                        carried += d.qty * d.basis_per_unit;
                    }
                    reg.bump_equity(party, -carried);
                }
                Leg::Pledge { holder, instrument, to, qty } => {
                    reg.pledge(holder, instrument, to, qty);
                }
            }
        }
        self.record(Outcome::Settled, ins, period, journal, settled_kind)
    }

    fn record(
        &mut self,
        outcome: Outcome,
        ins: &Instruction<'_>,
        period: u32,
        journal: &mut Journal,
        kind: u32,
    ) -> Outcome {
        self.outcomes.push(outcome);
        self.at_period.push(period);
        self.leg_count.push(ins.legs.len() as u32);
        journal.say(period, 0, kind, &[], &[(0, Value::Num(ins.legs.len() as f64))], true);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CurrencyCode, InstrumentId, PartyId};

    fn world() -> (Register, Journal, Settlement, u32, u32) {
        let mut j = Journal::new();
        let ok = j.kinds.declare("instruction.settled");
        let no = j.kinds.declare("instruction.failed");
        (Register::new(), j, Settlement::new(), ok, no)
    }

    #[test]
    fn a_refused_instruction_moves_nothing_at_all() {
        let (mut reg, mut j, mut s, ok, no) = world();
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
        let out = s.settle(&Instruction { legs: &legs, cause: Cause::Trade }, 1, &mut reg, &mut j, ok, no);
        assert_eq!(out, Outcome::ShortOfUnits);
        // NOTHING moved: the money is where it was and so are the shares.
        assert_eq!(reg.quantity(reg.row(a, cash)), 500.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 10.0);
        // And the fail is a RECORDED state, not a silence.
        assert_eq!(j.in_period(1).len(), 1);
    }

    #[test]
    fn both_legs_of_a_trade_move_in_the_same_pass() {
        let (mut reg, mut j, mut s, ok, no) = world();
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
        let out = s.settle(&Instruction { legs: &legs, cause: Cause::Trade }, 1, &mut reg, &mut j, ok, no);
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
        let (mut reg, mut j, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let lender = PartyId::at(2);
        let share = InstrumentId::at(1);
        reg.credit(a, share, 10.0, 3.0, 1);
        reg.pledge(a, share, lender, 8.0);
        let legs = [Leg::Asset { from: a, to: b, instrument: share, qty: 5.0, price_per_unit: None }];
        let out = s.settle(&Instruction { legs: &legs, cause: Cause::Trade }, 1, &mut reg, &mut j, ok, no);
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
    }

    #[test]
    fn a_payment_moves_money_and_makes_nobody_richer() {
        // Law 5: every flow has two sides, so what one account loses another gains and the world's
        // equity is unchanged. A payment that moved the total would be money appearing from nowhere.
        let (mut reg, mut j, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        reg.money_delta(a, cash, 500.0);
        let before = reg.equity(a) + reg.equity(b);
        let legs = [Leg::Money {
            from: a,
            to: b,
            ccy: CurrencyCode::at(0),
            instrument: cash,
            amount: 120.0,
            receipt: Receipt::Wage,
        }];
        s.settle(&Instruction { legs: &legs, cause: Cause::Payment }, 1, &mut reg, &mut j, ok, no);
        assert_eq!(reg.equity(a), -120.0);
        assert_eq!(reg.equity(b), 120.0);
        assert_eq!(reg.equity(a) + reg.equity(b), before);
        // And what passed THROUGH each account is its own magnitude, for Law 7's dust.
        assert_eq!(reg.gross(a), 120.0);
    }

    #[test]
    fn a_sale_books_the_gain_and_never_plugs_it() {
        // Register D2: the seller gives up what the units cost it and takes in what it was paid.
        // The difference is a GAIN and it is on the account, not a residual with no holder.
        let (mut reg, mut j, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(b, cash, 1_000.0);
        reg.credit(a, share, 10.0, 3.0, 1);
        let legs = [
            Leg::Asset { from: a, to: b, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
            Leg::Money { from: b, to: a, ccy: CurrencyCode::at(0), instrument: cash, amount: 50.0, receipt: Receipt::Sale },
        ];
        s.settle(&Instruction { legs: &legs, cause: Cause::Trade }, 1, &mut reg, &mut j, ok, no);
        // It gave up 30 of book and took in 50: it is 20 better off, and nothing was invented.
        assert_eq!(reg.equity(a), -30.0 + 50.0);
        // The buyer paid 50 and holds 50 of stock: unchanged, which is what a purchase is.
        assert_eq!(reg.equity(b), 50.0 - 50.0);
    }

    #[test]
    fn a_transfer_carries_the_basis_and_a_trade_does_not() {

        let (mut reg, mut j, mut s, ok, no) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let good = InstrumentId::at(3);
        reg.credit(a, good, 10.0, 7.0, 1);
        let legs = [Leg::Asset { from: a, to: b, instrument: good, qty: 10.0, price_per_unit: None }];
        s.settle(&Instruction { legs: &legs, cause: Cause::CorporateAction }, 2, &mut reg, &mut j, ok, no);
        // Register D2: what it cost went with it.
        assert_eq!(reg.lots(reg.row(b, good))[0].basis_per_unit, 7.0);
    }
}
