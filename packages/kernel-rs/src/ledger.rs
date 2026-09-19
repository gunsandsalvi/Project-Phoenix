//! The wire: state changes ONLY by a numbered, two-sided instruction applied by settlement, all legs
//! atomic (delivery-versus-payment, XI-5), settlement is FINAL, and a fail is a recorded state.

use crate::calendar::{Calendar, Day, Period};
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Instruments;
use crate::journal::{Journal, Value};
use crate::parties::Parties;
use crate::register::Register;

/// Every flow has two sides, both legs, same pass, same period, same currency.
#[derive(Clone, Copy)]
pub enum Leg {
    /// Money moving between two accounts.
    Money { from: PartyId, to: PartyId, instrument: InstrumentId, amount: f64, receipt: Receipt },
    /// Units of an instrument moving between two holders, at a price if it is a trade.
    Asset { from: PartyId, to: PartyId, instrument: InstrumentId, qty: f64, price_per_unit: Option<f64> },
    /// Goods B: a physical thing coming into existence.
    Create { party: PartyId, instrument: InstrumentId, qty: f64, cost_per_unit: f64 },
    /// An issuer creating its own money.
    Mint { issuer: PartyId, money: InstrumentId, amount: f64 },
    /// And a thing leaving it.
    Destroy { party: PartyId, instrument: InstrumentId, qty: f64, why: Gone },
    /// A claim over units, which refuses their move rather than adjusting it.
    Pledge { holder: PartyId, instrument: InstrumentId, to: PartyId, qty: f64 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gone {
    Consumed,
    Perished,
    Scrapped,
}

/// What money IS to the party receiving it.
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

/// Why the units moved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Trade,
    Payment,
    CorporateAction,
    Production,
    Settlement,
}

/// A-20: a fail is a RECORDED STATE and the module has to read it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Settled,
    /// The payer had not got it YET.
    Queued,
    /// The payer had not got it and its bank would not lend.
    ShortOfMoney,
    /// The payer had it and its BANK could not settle across.
    BankCouldNotSettle,
    /// The payee has no account in the money this was sent in.
    NoAccountInThatMoney,
    /// The units are there and somebody else has a claim over them.
    Encumbered,
    /// The holder has not got the units, and a short needs a borrow.
    ShortOfUnits,
}

/// HOW THE UNITS AND THE MONEY ARE TIED TOGETHER — and it is DECLARED, not inferred.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delivery {
    /// Units and money in the same instruction, so neither happens without the other.
    AgainstPayment,
    Free,
    /// Nothing crossed between two parties: a payment, a thing made, a thing that perished.
    Nothing,
}

/// What settlement is given to work on.
pub struct Settling<'a> {
    pub register: &'a mut Register,
    pub journal: &'a mut Journal,
    pub parties: &'a Parties,
    /// Settlement WRITES this now, because it is where units come into and go out of existence —
    /// `Create` and `Mint` make them, `Destroy` unmakes them — and Register B1 says the issued
    /// amount moves only by a named event.
    pub instruments: &'a mut Instruments,
    /// The one calendar, because a queued payment is late on a DATE and never after a count of
    /// periods.
    pub calendar: &'a Calendar,
    /// What an outcome is SAID under.
    pub says: Outcomes,
}

/// The journal kinds an instruction's outcome is said under, named once.
#[derive(Clone, Copy, Debug)]
pub struct Outcomes {
    pub settled: u32,
    pub failed: u32,
    /// It is waiting, which is neither of the other two.
    pub queued: u32,
    /// And the kind a process ending is said under.
    pub closed: u32,
    /// The kind what a disposal realised is said under.
    pub realised: u32,
}

impl Outcomes {
    /// The four kinds, declared once on a journal.
    pub fn declared(journal: &mut Journal) -> Self {
        Self {
            settled: journal.kinds.declare("instruction.settled"),
            failed: journal.kinds.declare("instruction.failed"),
            queued: journal.kinds.declare("instruction.queued"),
            closed: journal.kinds.declare("process.closed"),
            realised: journal.kinds.declare("disposal.realised"),
        }
    }
}

/// The account a party pays out of and is paid into.
pub fn account_of(parties: &Parties, instruments: &Instruments, p: PartyId) -> Option<InstrumentId> {
    let bank = parties.bank_of(p);
    instruments.money_issued_by(if bank.some() { bank } else { p })
}

/// Where a payment lands.
#[derive(Clone, Copy)]
enum Across {
    /// Nothing crosses.
    Same,
    /// Two banks, one money, and the reserve line they both settle in.
    Banks {
        payers_bank: PartyId,
        payees_bank: PartyId,
        payees_money: InstrumentId,
        reserves: InstrumentId,
    },
    /// It cannot land, and the outcome says whose failure it is.
    Refused(Outcome, PartyId),
}

/// Whether this payment crosses two banks, and what it takes if it does.
fn across(
    parties: &Parties,
    instruments: &Instruments,
    to: PartyId,
    money: InstrumentId,
) -> Across {
    let payers_bank = instruments.issuer_of(money);
    let payees_bank = parties.bank_of(to);
    if !payees_bank.some() || payees_bank == payers_bank || to == payers_bank {
        return Across::Same;
    }
    let payees_money = match account_of(parties, instruments, to) {
        Some(m) => m,
        None => panic!(
            "Money D2: party {} banks at {}, which issues no money — a payment to it has nowhere to land",
            to.0, payees_bank.0
        ),
    };
    // A PAYMENT IN ONE MONEY LANDS AS THAT MONEY.
    if instruments.ccy_of(money) != instruments.ccy_of(payees_money) {
        return Across::Refused(Outcome::NoAccountInThatMoney, to);
    }
    // The reserve line is the one BOTH banks settle in, so it is read off both: taking it from
    // one side alone debits that bank in reserves it need never have held.
    let settles_in = |b: PartyId| match account_of(parties, instruments, b) {
        Some(r) => r,
        None => panic!(
            "Money D2, 31 A1: bank {} settles in no money, so two banks have no way to settle between them",
            b.0
        ),
    };
    let (mine, theirs) = (settles_in(payers_bank), settles_in(payees_bank));
    if mine != theirs {
        return Across::Refused(Outcome::BankCouldNotSettle, payers_bank);
    }
    Across::Banks { payers_bank, payees_bank, payees_money, reserves: mine }
}

pub struct Instruction<'a> {
    pub legs: &'a [Leg],
    pub cause: Cause,
    /// What the writer says this is.
    pub delivery: Delivery,
}

impl<'a> Instruction<'a> {
    /// The ordinary way.
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
        let mut deliveries: Vec<(PartyId, PartyId)> = Vec::new();
        let mut money: Vec<(PartyId, PartyId)> = Vec::new();
        for leg in self.legs {
            match *leg {
                Leg::Asset { from, to, .. } if from != to => deliveries.push((from, to)),
                Leg::Money { from, to, .. } if from != to => money.push((from, to)),
                _ => {}
            }
        }
        if deliveries.is_empty() {
            return Delivery::Nothing;
        }
        // Against: the payer received units here, or the payee delivered them.
        let against = money
            .iter()
            .any(|(mf, mt)| deliveries.iter().any(|(df, dt)| mf == dt || mt == df));
        if against {
            Delivery::AgainstPayment
        } else {
            Delivery::Free
        }
    }
}

/// How an instruction is presented to the wire, which decides how its legs are checked and whether a
/// short payment may wait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Presented {
    /// The ordinary way: leg by leg, and a short payment joins the queue.
    Fresh,
    /// A queued payment tried again because its payer was paid.
    Retry,
    /// A gridlock cycle, settled TOGETHER.
    Together,
}

/// What the legs of one instruction do to each holding, TOGETHER.
fn short_together(
    legs: &[Leg],
    reg: &Register,
    parties: &Parties,
    instruments: &Instruments,
) -> Option<(Outcome, PartyId)> {
    let mut delta: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
    let mut moves = |who: PartyId, what: InstrumentId, by: f64| {
        *delta.entry((who.0, what.0)).or_insert(0.0) += by;
    };
    for leg in legs {
        if let Leg::Money { from, to, instrument, amount, .. } = *leg {
            moves(from, instrument, -amount);
            match across(parties, instruments, to, instrument) {
                Across::Same => moves(to, instrument, amount),
                Across::Banks { payers_bank, payees_bank, payees_money, reserves } => {
                    moves(to, payees_money, amount);
                    // And the reserves the banks move between them, which net over a cycle exactly
                    // as the customers' deposits do.
                    moves(payers_bank, reserves, -amount);
                    moves(payees_bank, reserves, amount);
                }
                // A leg that cannot land at all is refused by the pass above this one, before any
                // balance is read — so by here there is nothing left to be short of.
                Across::Refused(..) => {}
            }
        }
    }
    let short = |who: PartyId, what: InstrumentId| match delta.get(&(who.0, what.0)) {
        Some(by) => *by < 0.0 && reg.quantity(reg.row(who, what)) + by < 0.0,
        None => false,
    };
    for leg in legs {
        if let Leg::Money { from, to, instrument, .. } = *leg {
            if short(from, instrument) {
                return Some((Outcome::ShortOfMoney, from));
            }
            // And the payer's BANK needs the reserves to settle it across.
            if let Across::Banks { payers_bank, reserves, .. } = across(parties, instruments, to, instrument) {
                if short(payers_bank, reserves) {
                    return Some((Outcome::BankCouldNotSettle, payers_bank));
                }
            }
        }
    }
    None
}

/// Who this instruction PAYS.
fn paid_by(legs: &[Leg]) -> Vec<PartyId> {
    legs.iter()
        .filter_map(|l| match *l {
            Leg::Money { from, to, .. } if from != to => Some(to),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct QueueId(pub u32);

impl QueueId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// What a queued payment is doing, in the words this project already has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Waiting {
    Queued,
    Taken,
    Late,
}

/// THE PAYMENT QUEUE — a payment that cannot be made YET is not a payment that failed.
pub struct Queue {
    leg_at: Vec<u32>,
    leg_len: Vec<u32>,
    legs: Vec<Leg>,
    cause: Vec<Cause>,
    delivery: Vec<Delivery>,
    /// Who is short.
    payer: Vec<u32>,
    /// The day it was tried, and the day it stops being early and becomes an arrear.
    queued_on: Vec<i64>,
    late_after: Vec<i64>,
    /// The day it stopped waiting, so what became of a payment is a read off the row and never a
    /// tally kept beside it.
    finished_on: Vec<Option<i64>>,
    state: Vec<Waiting>,
    by_payer: std::collections::HashMap<u32, Vec<u32>>,
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub fn new() -> Self {
        Self {
            leg_at: Vec::new(),
            leg_len: Vec::new(),
            legs: Vec::new(),
            cause: Vec::new(),
            delivery: Vec::new(),
            payer: Vec::new(),
            queued_on: Vec::new(),
            late_after: Vec::new(),
            finished_on: Vec::new(),
            state: Vec::new(),
            by_payer: std::collections::HashMap::new(),
        }
    }

    /// A payment joins the queue: its legs are OWNED here, because the instruction that made it is
    /// over and the queue outlives it.
    pub fn joins(&mut self, ins: &Instruction<'_>, payer: PartyId, on: Day, late_after: Day) -> QueueId {
        assert!(payer.some(), "A-20: a payment that is nobody's is not queued");
        assert!(
            late_after.0 >= on.0,
            "22d.1: a payment late before it was tried has no time to wait at all"
        );
        let row = self.state.len() as u32;
        self.leg_at.push(self.legs.len() as u32);
        self.leg_len.push(ins.legs.len() as u32);
        self.legs.extend_from_slice(ins.legs);
        self.cause.push(ins.cause);
        self.delivery.push(ins.delivery);
        self.payer.push(payer.0);
        self.queued_on.push(on.0);
        self.late_after.push(late_after.0);
        self.finished_on.push(None);
        self.state.push(Waiting::Queued);
        self.by_payer.entry(payer.0).or_default().push(row);
        QueueId(row)
    }

    /// The legs of one queued payment, as a contiguous slice.
    pub fn legs_of(&self, q: QueueId) -> &[Leg] {
        let at = self.leg_at[q.row()] as usize;
        let len = self.leg_len[q.row()] as usize;
        &self.legs[at..at + len]
    }

    pub fn cause_of(&self, q: QueueId) -> Cause {
        self.cause[q.row()]
    }

    pub fn delivery_of(&self, q: QueueId) -> Delivery {
        self.delivery[q.row()]
    }

    pub fn payer_of(&self, q: QueueId) -> PartyId {
        PartyId(self.payer[q.row()])
    }

    pub fn state_of(&self, q: QueueId) -> Waiting {
        self.state[q.row()]
    }

    pub fn queued_on(&self, q: QueueId) -> Day {
        Day(self.queued_on[q.row()])
    }

    pub fn late_after(&self, q: QueueId) -> Day {
        Day(self.late_after[q.row()])
    }

    /// A SELLER AGREED TO WAIT.
    pub fn given_time(&mut self, q: QueueId, until: Day) {
        assert!(self.state[q.row()] == Waiting::Queued, "36 C1: only a waiting payment is given time");
        assert!(
            until.0 > self.late_after[q.row()],
            "36 C1: terms that end sooner than the payment's own day are not time given"
        );
        self.late_after[q.row()] = until.0;
    }

    /// It went through on a retry.
    pub fn took(&mut self, q: QueueId, on: Day) {
        assert!(self.state[q.row()] == Waiting::Queued, "22d.1: only a waiting payment is taken");
        self.state[q.row()] = Waiting::Taken;
        self.finished_on[q.row()] = Some(on.0);
    }

    /// Its days ran out.
    pub fn gave_up(&mut self, q: QueueId, on: Day) {
        assert!(self.state[q.row()] == Waiting::Queued, "22d.1: only a waiting payment gives up");
        self.state[q.row()] = Waiting::Late;
        self.finished_on[q.row()] = Some(on.0);
    }

    /// What this party is still waiting to pay — the one question a retry asks.
    pub fn of_payer(&self, p: PartyId) -> Vec<QueueId> {
        match self.by_payer.get(&p.0) {
            Some(rows) => rows
                .iter()
                .map(|r| QueueId(*r))
                .filter(|q| self.state_of(*q) == Waiting::Queued)
                .collect(),
            None => Vec::new(),
        }
    }

    /// Every payment whose day has passed, in the order they joined.
    pub fn out_of_days(&self, on: Day) -> Vec<QueueId> {
        (0..self.state.len() as u32)
            .map(QueueId)
            .filter(|q| self.state_of(*q) == Waiting::Queued && self.late_after(*q).0 < on.0)
            .collect()
    }

    /// A CYCLE IN THE QUEUE — A waits on B, B waits on C, C waits on A.
    pub fn a_cycle(&self, skip: &std::collections::HashSet<u32>) -> Option<Vec<QueueId>> {
        let links: Vec<QueueId> = (0..self.state.len() as u32)
            .map(QueueId)
            .filter(|q| self.state_of(*q) == Waiting::Queued && !skip.contains(&q.0))
            .filter(|q| self.legs_of(*q).iter().all(|l| matches!(l, Leg::Money { .. })))
            .collect();
        let mut owes: std::collections::HashMap<u32, Vec<QueueId>> = std::collections::HashMap::new();
        for q in &links {
            owes.entry(self.payer_of(*q).0).or_default().push(*q);
        }
        // A party the walk has left behind without finding a ring through it is not walked again:
        // the answer would be the same, and this is what keeps the search over the whole queue.
        let mut done: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for q in &links {
            let mut path: Vec<QueueId> = Vec::new();
            let mut at: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
            if let Some(ring) = self.ring_from(self.payer_of(*q).0, &owes, &mut path, &mut at, &mut done) {
                return Some(ring);
            }
        }
        None
    }

    /// Who this queued payment pays.
    fn payee_of(&self, q: QueueId) -> Option<PartyId> {
        self.legs_of(q).iter().find_map(|l| match *l {
            Leg::Money { from, to, .. } if from != to => Some(to),
            _ => None,
        })
    }

    fn ring_from(
        &self,
        who: u32,
        owes: &std::collections::HashMap<u32, Vec<QueueId>>,
        path: &mut Vec<QueueId>,
        at: &mut std::collections::HashMap<u32, usize>,
        done: &mut std::collections::HashSet<u32>,
    ) -> Option<Vec<QueueId>> {
        if let Some(from) = at.get(&who) {
            return Some(path[*from..].to_vec());
        }
        if done.contains(&who) {
            return None;
        }
        let edges = match owes.get(&who) {
            Some(edges) => edges.clone(),
            None => {
                done.insert(who);
                return None;
            }
        };
        at.insert(who, path.len());
        for e in edges {
            let to = match self.payee_of(e) {
                Some(to) => to.0,
                None => continue,
            };
            path.push(e);
            if let Some(ring) = self.ring_from(to, owes, path, at, done) {
                return Some(ring);
            }
            path.pop();
        }
        at.remove(&who);
        done.insert(who);
        None
    }

    /// Every payment ever queued.
    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// How many are waiting right now, and how many each of the other two states holds.
    pub fn census(&self) -> (usize, usize, usize) {
        self.between(Day(i64::MIN), Day(i64::MAX))
    }

    /// WHAT BECAME OF THE PAYMENTS THAT WERE SHORT, between two days — how many are still waiting,
    /// how many went through after waiting, how many ran out of days.
    pub fn between(&self, from: Day, to: Day) -> (usize, usize, usize) {
        let mut waiting = 0;
        let mut taken = 0;
        let mut late = 0;
        for row in 0..self.state.len() {
            let when = match self.state[row] {
                Waiting::Queued => self.queued_on[row],
                _ => match self.finished_on[row] {
                    Some(day) => day,
                    None => continue,
                },
            };
            if when < from.0 || when > to.0 {
                continue;
            }
            match self.state[row] {
                Waiting::Queued => waiting += 1,
                Waiting::Taken => taken += 1,
                Waiting::Late => late += 1,
            }
        }
        (waiting, taken, late)
    }
}

/// EVERY INSTRUCTION EVER APPLIED, numbered, in order, with its legs.
pub struct Settlement {
    outcomes: Vec<Outcome>,
    at_period: Vec<u32>,
    cause: Vec<Cause>,
    leg_at: Vec<u32>,
    leg_len: Vec<u32>,
    legs: Vec<Leg>,
    /// Where each period's instructions begin and end, written as they arrive.
    by_period: Vec<(u32, u32, u32)>,
    /// Every free delivery: who performed, who was trusted, and when.
    delivered_free: Vec<(PartyId, PartyId, u32)>,
    /// The payments it could not make yet.
    pub queue: Queue,
    /// One TECHNOLOGY: how many days a payment may wait before it is late.
    waits_for: i64,
}

impl Settlement {
    pub fn new(waits_for: i64) -> Self {
        assert!(waits_for >= 0, "22d.1: a payment that may wait {waits_for} days may not wait");
        Self {
            outcomes: Vec::new(),
            at_period: Vec::new(),
            cause: Vec::new(),
            leg_at: Vec::new(),
            leg_len: Vec::new(),
            legs: Vec::new(),
            by_period: Vec::new(),
            delivered_free: Vec::new(),
            queue: Queue::new(),
            waits_for,
        }
    }

    /// How long a payment may wait here, as this payment system was built.
    pub fn waits_for(&self) -> i64 {
        self.waits_for
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

    /// The legs of one instruction, as a contiguous slice.
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

    /// This period's instructions, without walking the history.
    pub fn in_period(&self, period: u32) -> std::ops::Range<usize> {
        for &(p, from, to) in &self.by_period {
            if p == period {
                return from as usize..to as usize;
            }
        }
        0..0
    }


    /// ALL LEGS OR NONE.
    pub fn settle(&mut self, ins: &Instruction<'_>, period: u32, on: &mut Settling<'_>) -> Outcome {
        let out = self.attempt(ins, period, on, Presented::Fresh);
        if out == Outcome::Settled {
            self.release(ins.legs, period, on);
        }
        out
    }

    /// A receipt is a retry.
    fn release(&mut self, legs: &[Leg], period: u32, on: &mut Settling<'_>) {
        let today = on.calendar.start_of(Period(period));
        let mut funded: Vec<PartyId> = paid_by(legs);
        while let Some(who) = funded.pop() {
            for q in self.queue.of_payer(who) {
                let waiting: Vec<Leg> = self.queue.legs_of(q).to_vec();
                let ins = Instruction {
                    legs: &waiting,
                    cause: self.queue.cause_of(q),
                    delivery: self.queue.delivery_of(q),
                };
                if self.attempt(&ins, period, on, Presented::Retry) == Outcome::Settled {
                    self.queue.took(q, today);
                    funded.extend(paid_by(&waiting));
                }
            }
        }
    }

    /// ONE PASS THAT FINDS THE CYCLES AND SETTLES THEM TOGETHER.
    pub fn unwind(&mut self, period: u32, on: &mut Settling<'_>) -> usize {
        let today = on.calendar.start_of(Period(period));
        let mut went = 0usize;
        // The cycles that were found and did not balance — somebody in them is short beyond what the
        // cycle itself funds.
        let mut stuck: std::collections::HashSet<u32> = std::collections::HashSet::new();
        while let Some(rows) = self.queue.a_cycle(&stuck) {
            let legs: Vec<Leg> = rows.iter().flat_map(|q| self.queue.legs_of(*q).to_vec()).collect();
            let ins = Instruction { legs: &legs, cause: Cause::Settlement, delivery: Delivery::Nothing };
            if self.attempt(&ins, period, on, Presented::Together) == Outcome::Settled {
                for q in &rows {
                    self.queue.took(*q, today);
                }
                went += rows.len();
                // And a cycle that settled has paid people, so whatever THAT funds goes too.
                self.release(&legs, period, on);
            } else {
                stuck.extend(rows.iter().map(|q| q.0));
            }
        }
        went
    }

    /// The queue's day passed.
    pub fn give_up(&mut self, today: Day, period: u32, on: &mut Settling<'_>) -> usize {
        let done = self.queue.out_of_days(today);
        for q in &done {
            let waiting: Vec<Leg> = self.queue.legs_of(*q).to_vec();
            let ins = Instruction {
                legs: &waiting,
                cause: self.queue.cause_of(*q),
                delivery: self.queue.delivery_of(*q),
            };
            let who = self.queue.payer_of(*q);
            self.queue.gave_up(*q, today);
            let failed = on.says.failed;
            self.record(Outcome::ShortOfMoney, who, &ins, period, on.journal, failed);
        }
        done.len()
    }

    fn attempt(
        &mut self,
        ins: &Instruction<'_>,
        period: u32,
        on: &mut Settling<'_>,
        how: Presented,
    ) -> Outcome {
        let may_queue = how == Presented::Fresh;
        let Settling { register: reg, journal, parties, instruments, calendar, says } = on;
        let (parties, says, calendar) = (*parties, *says, *calendar);
        let (settled_kind, failed_kind, realised_kind) = (says.settled, says.failed, says.realised);
        // What each disposal realised, gathered as the legs apply and said once they all have, so
        // a half-applied instruction never publishes a gain.
        let mut realised: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
        // The declaration, against the legs.
        let shape = ins.shape();
        assert!(
            shape == ins.delivery,
            "XI-5: this instruction is declared {:?} and its legs are {shape:?} — \
             a delivery with no money against it is either FREE OF PAYMENT or a payment somebody \
             forgot, and the wire cannot tell which unless the writer says",
            ins.delivery
        );

        // The pre-check.

        // A gridlock cycle is checked on what the whole of it does to each holding, and every other
        // instruction leg by leg.
        for leg in ins.legs {
            match *leg {
                Leg::Money { to, instrument, .. } => {
                    // Where it lands is asked FIRST, before any balance is read.
                    if let Across::Refused(outcome, who) = across(parties, instruments, to, instrument) {
                        return self.record(outcome, who, ins, period, journal, failed_kind);
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
                // NO MONEY WITHOUT AN ISSUER, checked at the one site in this engine that creates
                // money.
                Leg::Mint { issuer, money, amount, .. } => {
                    let owes = instruments.issuer_of(money);
                    assert!(
                        owes == issuer,
                        "Money A1, A1.d: party {} minting money issued by {} — every unit is owed \
                         by a NAMED issuer, and a balance that is nobody's liability is money \
                         created from nothing",
                        issuer.0,
                        owes.0
                    );
                    assert!(
                        instruments.class_of(money) == crate::instruments::Class::Money,
                        "Money D2: instrument {} is not money. Minting it writes the holding \
                         through `money_delta`, which makes the row a TOTAL — so the lots its \
                         basis lives in would be thrown away by an instruction that settled",
                        money.0
                    );
                    assert!(
                        amount > 0.0,
                        "Money C4: a mint of {amount} creates nothing. Retiring money is a \
                         different act with a different clause, and this is not it"
                    );
                }
                // The other side of the same line.
                Leg::Create { instrument, qty, .. } => {
                    assert!(
                        instruments.class_of(instrument) != crate::instruments::Class::Money,
                        "Money A1, D2: money is MINTED by the party that owes it and never created \
                         on a holder's book — instrument {} is money, and that is the line \
                         `Leg::Mint` exists to draw",
                        instrument.0
                    );
                    assert!(qty > 0.0, "37 B3: {qty} units is not a thing coming into existence");
                }
                // Appendix B #9, Register C3: a lien over units nobody holds.
                Leg::Pledge { holder, instrument, qty, .. } => {
                    let row = reg.row(holder, instrument);
                    if !row.some() || reg.quantity(row) < qty {
                        return self.record(Outcome::ShortOfUnits, holder, ins, period, journal, failed_kind);
                    }
                    if reg.free(row) < qty {
                        return self.record(Outcome::Encumbered, holder, ins, period, journal, failed_kind);
                    }
                }
            }
        }
        // And what the money legs do to each holding TOGETHER, which is the only test that is right
        // for an instruction with two legs out of one account.
        if let Some((outcome, who)) = short_together(ins.legs, reg, parties, instruments) {
            return self.short(outcome, who, ins, period, journal, calendar, says, may_queue);
        }
        // The application.

        for leg in ins.legs {
            match *leg {
                Leg::Money { from, to, instrument, amount, .. } => {
                    // THE INTERBANK LEG.
                    reg.money_delta(from, instrument, -amount);
                    match across(parties, instruments, to, instrument) {
                        Across::Same => {
                            reg.money_delta(to, instrument, amount);
                        }
                        Across::Banks { payers_bank, payees_bank, payees_money, reserves } => {
                            reg.money_delta(to, payees_money, amount);
                            reg.money_delta(payers_bank, reserves, -amount);
                            reg.money_delta(payees_bank, reserves, amount);
                        }
                        // Nothing here can fail, because the pre-check is what made that true — and
                        // 0k.3's refusal is one of the things it made true.
                        Across::Refused(outcome, who) => panic!(
                            "XI-5: {outcome:?} on party {} reached the application, which the \
                             pre-check exists to make impossible",
                            who.0
                        ),
                    }
                }
                Leg::Asset { from, to, instrument, qty, price_per_unit } => {
                    let row = reg.row(from, instrument);
                    let drawn = reg.debit(row, qty);
                    // What the units cost goes with them where it is a transfer, and the price is
                    // the basis where a market struck one.
                    match price_per_unit {
                        Some(price) => {
                            // And what the seller REALISED, which is the one thing only this line
                            // knows: the proceeds against what the lots that left cost.
                            let cost: f64 = drawn.iter().map(|d| d.qty * d.basis_per_unit).sum();
                            realised.push((from, instrument, qty * price - cost));
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
                    // Register B1, Goods B: and that many units of the line now EXIST.
                    instruments.moves(instrument, crate::instruments::Issuance::Made, qty);
                }
                // The issuer's own money, as a TOTAL, and NO equity — what it created is what it
                // owes, and `Instruments::owed_by` reads that from issued against held.
                Leg::Mint { issuer, money, amount, .. } => {
                    reg.money_delta(issuer, money, amount);
                    // Money issued is money that exists, and the count moves here for the same
                    // reason a line's does — it is the one place it comes into being.
                    instruments.moves(money, crate::instruments::Issuance::Made, amount);
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    let drawn = reg.debit(row, qty);
                    // What perished cost something, and the loss is an EVENT rather than a number
                    // that quietly stops existing.
                    let cost: f64 = drawn.iter().map(|d| d.qty * d.basis_per_unit).sum();
                    if cost != 0.0 {
                        realised.push((party, instrument, -cost));
                    }
                    // And there is that much less of it in the world.
                    instruments.moves(instrument, crate::instruments::Issuance::Gone, qty);
                }
                Leg::Pledge { holder, instrument, to, qty } => {
                    reg.pledge(holder, instrument, to, qty);
                }
            }
        }
        // A free delivery is a risk somebody took, so the record NAMES who took it.
        if ins.delivery == Delivery::Free {
            for leg in ins.legs {
                if let Leg::Asset { from, to, .. } = *leg {
                    if from != to {
                        self.delivered_free.push((from, to, period));
                    }
                }
            }
        }
        // And what the disposals realised, now that every leg has applied.
        for (who, line, amount) in realised {
            journal.say(
                period,
                0,
                realised_kind,
                &[who.0],
                &[(0, crate::journal::Value::Num(amount)), (1, crate::journal::Value::Num(f64::from(line.0)))],
                false,
            );
        }
        self.record(Outcome::Settled, PartyId::NONE, ins, period, journal, settled_kind)
    }

    /// Short of money is a QUEUE and not an arrear — where the instruction is a payment.
    #[allow(clippy::too_many_arguments)]
    fn short(
        &mut self,
        outcome: Outcome,
        who: PartyId,
        ins: &Instruction<'_>,
        period: u32,
        journal: &mut Journal,
        calendar: &Calendar,
        says: Outcomes,
        may_queue: bool,
    ) -> Outcome {
        if !may_queue || ins.delivery != Delivery::Nothing {
            return self.record(outcome, who, ins, period, journal, says.failed);
        }
        let today = calendar.start_of(Period(period));
        self.queue.joins(ins, who, today, Day(today.0 + self.waits_for));
        journal.say(period, 0, says.queued, &[who.0], &[(0, Value::Num(ins.legs.len() as f64))], true);
        Outcome::Queued
    }

    fn record(
        &mut self,
        outcome: Outcome,
        // WHOSE failure it is: a fail with no subjects is a recorded state nobody can find by
        // looking for their own.
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

    /// A small world these tests settle in.
    fn world() -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
        let bank = PartyId::at(9);
        let mut p = Parties::new();
        for _ in 0..16 {
            p.add(0, RegionId::at(0), bank, Representation::Named, 0);
        }
        let mut i = Instruments::new();
        i.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        for _ in 1..16 {
            i.issue(PartyId::at(1), CurrencyCode::at(0), Class::Good, UnitId::at(0), None, None);
        }
        (Register::new(), j, p, i, Settlement::new(6), Calendar::new(Day(0), 7, 3), says)
    }

    /// What a party is worth, READ from what it holds against what it owes.
    fn worth(reg: &Register, ins: &Instruments, p: PartyId) -> f64 {
        // Settlement moves instruments, and no instruction in these cases makes a claim on an estate
        // — so an empty book of them is what this world has, not a corner being avoided.
        crate::instruments::equity(p, reg, ins, &crate::stores::Claims::new())
    }

    /// The four stores settlement works on, gathered for a call.
    fn on<'a>(
        register: &'a mut Register,
        journal: &'a mut Journal,
        parties: &'a Parties,
        instruments: &'a mut Instruments,
        calendar: &'a Calendar,
        says: Outcomes,
    ) -> Settling<'a> {
        Settling { register, journal, parties, instruments, calendar, says }
    }

    #[test]
    fn a_refused_instruction_moves_nothing_at_all() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 3.0, 1);
        // Delivery-versus-payment: the money is there, the shares are not enough.
        let legs = [
            Leg::Money { from: a, to: b, instrument: cash, amount: 500.0, receipt: Receipt::Sale },
            Leg::Asset { from: b, to: a, instrument: share, qty: 99.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfUnits);
        // NOTHING moved: the money is where it was and so are the shares.
        assert_eq!(reg.quantity(reg.row(a, cash)), 500.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 10.0);
        // And the fail is a RECORDED state, not a silence.
        assert_eq!(j.in_period(1).len(), 1);
    }

    #[test]
    fn both_legs_of_a_trade_move_in_the_same_pass() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 3.0, 1);
        let legs = [
            Leg::Money { from: a, to: b, instrument: cash, amount: 50.0, receipt: Receipt::Sale },
            Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(a, cash)), 450.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 50.0);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 0.0);
        // The buyer's lot carries what the market struck, not what the seller paid.
        assert_eq!(reg.lots(reg.row(a, share))[0].basis_per_unit, 5.0);

        // And what the SELLER realised — 50 of proceeds against 30 the lots cost.
        let said: Vec<u32> = j.in_period(1).filter(|r| j.kind_of(*r) == says.realised).collect();
        let gain = said
            .iter()
            .find(|r| j.subjects_of(**r) == [b.0])
            .expect("the disposal reaches the party that made it, and nobody else");
        assert_eq!(j.says(*gain, 0), Some(Value::Num(20.0)));
        assert!(!j.is_public(*gain), "Observer A3: what one holder made on a sale is its own business");
    }

    #[test]
    fn a_thing_that_perished_realises_what_it_cost_as_a_loss() {
        // A loss is an EVENT rather than a number that quietly stops existing.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let grain = InstrumentId::at(1);
        reg.credit(a, grain, 10.0, 4.0, 1);
        let legs = [Leg::Destroy { party: a, instrument: grain, qty: 10.0, why: Gone::Perished }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Production), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        let gain = j
            .in_period(1)
            .find(|r| j.kind_of(*r) == says.realised && j.subjects_of(*r) == [a.0])
            .expect("what perished cost somebody something");
        assert_eq!(j.says(gain, 0), Some(Value::Num(-40.0)));
    }

    #[test]
    fn encumbered_units_refuse_the_whole_instruction() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let lender = PartyId::at(2);
        let share = InstrumentId::at(1);
        reg.credit(a, share, 10.0, 3.0, 1);
        reg.pledge(a, share, lender, 8.0);
        let legs = [Leg::Asset { from: a, to: b, instrument: share, qty: 5.0, price_per_unit: None }];
        let out = s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
    }

    #[test]
    fn free_of_payment_delivers_and_records_who_was_trusted() {
        // A restructured bond handed over for the old one: the units move and nothing moves against
        // them.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
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
            &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says),
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
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(a, cash, 500.0);
        reg.credit(b, share, 10.0, 1.0, 1);
        let legs = [
            Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
            Leg::Money { from: a, to: b, instrument: cash, amount: 50.0, receipt: Receipt::Sale },
        ];
        s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    fn money_moving_with_a_delivery_is_not_a_payment_against_it() {
        // A cell splitting its book in two moves money AND units, both from the parent to the part
        // that left.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let parent = PartyId::at(0);
        let part = PartyId::at(1);
        let cash = InstrumentId::at(0);
        let share = InstrumentId::at(1);
        reg.money_delta(parent, cash, 900.0);
        reg.credit(parent, share, 30.0, 2.0, 1);
        let legs = [
            Leg::Asset { from: parent, to: part, instrument: share, qty: 10.0, price_per_unit: None },
            Leg::Money { from: parent, to: part, instrument: cash, amount: 300.0, receipt: Receipt::Transfer },
        ];
        let out = s.settle(
            &Instruction::free_of_payment(&legs, Cause::CorporateAction),
            1,
            &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says),
        );
        assert_eq!(out, Outcome::Settled);
        // A third of the people took a third of each, and the basis went with the units: nothing was
        // sold, so nothing was realised.
        assert_eq!(reg.quantity(reg.row(part, share)), 10.0);
        assert_eq!(reg.quantity(reg.row(parent, share)), 20.0);
        assert_eq!(reg.lots(reg.row(part, share))[0].basis_per_unit, 2.0);
    }

    #[test]
    #[should_panic(expected = "a payment somebody forgot")]
    fn a_trade_that_lost_its_money_leg_is_caught_instead_of_settling_free() {
        // The defect this pathway exists to make findable.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let share = InstrumentId::at(1);
        reg.credit(b, share, 10.0, 1.0, 1);
        let legs = [Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) }];
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    fn a_free_delivery_is_still_all_legs_or_none() {
        // XI-5 holds for a basket handed over free: one line short and NOTHING moves, because a
        // half-delivered restructuring is not a restructuring.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
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
            &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says),
        );
        assert_eq!(out, Outcome::ShortOfUnits);
        assert_eq!(reg.quantity(reg.row(issuer, one)), 100.0, "the first line did not move either");
        assert!(s.delivered_free().is_empty(), "nothing was delivered, so nobody was trusted");
    }

    #[test]
    fn a_payment_moves_money_and_makes_nobody_richer() {

        // Every flow has two sides, so what one account loses another gains and the world's equity
        // is unchanged.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let cash = InstrumentId::at(0);
        reg.money_delta(a, cash, 500.0);
        let before = worth(&reg, &ins, a) + worth(&reg, &ins, b);
        let bank_before = worth(&reg, &ins, PartyId::at(9));
        let legs = [Leg::Money {
            from: a,
            to: b,
            instrument: cash,
            amount: 120.0,
            receipt: Receipt::Wage,
        }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        // A LEVEL, not a running delta: what each is worth is read from its book every time.
        assert_eq!(worth(&reg, &ins, a), 500.0 - 120.0);
        assert_eq!(worth(&reg, &ins, b), 120.0);
        assert_eq!(worth(&reg, &ins, a) + worth(&reg, &ins, b), before);
        // And the BANK is no better or worse off for having moved it: it owes 120 less to one
        // depositor and 120 more to the other, which nets to nothing.
        assert_eq!(worth(&reg, &ins, PartyId::at(9)), bank_before);
    }

    #[test]
    fn a_sale_books_the_gain_and_never_plugs_it() {
        // The seller gives up what the units cost it and takes in what it was paid.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
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
            Leg::Money { from: b, to: a, instrument: cash, amount: 50.0, receipt: Receipt::Sale },
        ];
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        // It gave up 30 of book and took in 50: it is 20 better off, and nothing was invented.
        assert_eq!(seller_before, 30.0, "ten shares that cost three");
        assert_eq!(worth(&reg, &ins, a), 50.0);
        // The buyer paid 50 and holds 50 of stock: unchanged, which is what a purchase is.
        assert_eq!(worth(&reg, &ins, b), buyer_before);
    }

    #[test]
    fn a_transfer_carries_the_basis_and_a_trade_does_not() {

        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let good = InstrumentId::at(3);
        reg.credit(a, good, 10.0, 7.0, 1);
        let legs = [Leg::Asset { from: a, to: b, instrument: good, qty: 10.0, price_per_unit: None }];
        s.settle(&Instruction::free_of_payment(&legs, Cause::CorporateAction), 2, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        // What it cost went with it.
        assert_eq!(reg.lots(reg.row(b, good))[0].basis_per_unit, 7.0);
    }

    /// Two banks, and a payment between their customers.
    fn two_banks() -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes, [PartyId; 5], [InstrumentId; 3]) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
        let mut p = Parties::new();
        let region = RegionId::at(0);
        // The central bank banks nowhere, and both banks bank at it.
        let cb = p.add(0, region, PartyId::NONE, Representation::Named, 0);
        let one = p.add(0, region, cb, Representation::Named, 0);
        let two = p.add(0, region, cb, Representation::Named, 0);
        let payer = p.add(0, region, one, Representation::Named, 0);
        let payee = p.add(0, region, two, Representation::Named, 0);
        let mut i = Instruments::new();
        let unit = UnitId::at(0);
        let ccy = CurrencyCode::at(0);
        let reserves = i.issue(cb, ccy, Class::Money, unit, None, None);
        let ones = i.issue(one, ccy, Class::Money, unit, None, None);
        let twos = i.issue(two, ccy, Class::Money, unit, None, None);
        (Register::new(), j, p, i, Settlement::new(6), Calendar::new(Day(0), 7, 3), says, [cb, one, two, payer, payee], [reserves, ones, twos])
    }

    #[test]
    fn a_payment_across_two_banks_moves_reserves_between_them() {
        // Money D2, worklist 1: a deposit is a claim on the bank that ISSUED it, so a payee banking
        // elsewhere cannot simply come to hold the payer's bank's money.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says, who, lines) = two_banks();
        let (one, two, payer, payee) = (who[1], who[2], who[3], who[4]);
        let (reserves, ones, twos) = (lines[0], lines[1], lines[2]);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);

        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);

        // The payer's deposit at its own bank fell; the payee holds ITS OWN bank's money, not the
        // payer's — which is the whole point, because a deposit names who owes it.
        assert_eq!(reg.quantity(reg.row(payer, ones)), 200.0);
        assert_eq!(reg.quantity(reg.row(payee, twos)), 300.0);
        assert_eq!(reg.quantity(reg.row(payee, ones)), 0.0);
        // And the reserves moved: bank one is 300 poorer at the central bank and bank two 300
        // richer.
        assert_eq!(reg.quantity(reg.row(one, reserves)), 500.0);
        assert_eq!(reg.quantity(reg.row(two, reserves)), 300.0);
    }

    #[test]
    fn a_bank_without_the_reserves_cannot_settle_its_customer_out() {
        // What the leg makes possible: a bank's liquidity is tested by its customers' payments.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says, who, lines) = two_banks();
        let (payer, payee) = (who[3], who[4]);
        let ones = lines[1];
        reg.money_delta(payer, ones, 500.0);
        // Its bank has none: the deposit is there and the settlement asset is not.
        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        // It WAITS, and the row it waits on is the BANK's.
        assert_eq!(out, Outcome::Queued);
        let q = crate::ledger::QueueId(0);
        // And it is the BANK's, not the payer's.
        assert_eq!(s.queue.payer_of(q), who[1]);
        assert_ne!(s.queue.payer_of(q), payer, "the payer HAD it; its bank could not settle it out");
        // And nothing moved, because nothing settled.
        assert_eq!(reg.quantity(reg.row(payer, ones)), 500.0);

        // And when its days run out, THIS is the arrear — recorded on the wire, on the bank, which
        // is the failure this world used to record the instant the payer was short.
        let over = crate::calendar::Day(cal.start_of(crate::calendar::Period(1)).0 + s.waits_for() + 1);
        assert_eq!(s.give_up(over, 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)), 1);
        let failed: Vec<u32> = j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(j.subjects_of(failed[0]), &[who[1].0]);
        assert_eq!(s.queue.state_of(q), crate::ledger::Waiting::Late);
    }

    #[test]
    fn a_gridlock_unwinds_when_the_money_arrives_and_no_new_money_is_made() {
        // A cannot pay B because B has not yet paid A.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (a, b, c) = (PartyId::at(0), PartyId::at(1), PartyId::at(2));
        let cash = InstrumentId::at(0);
        // C has the money; A and B have none of their own.
        reg.money_delta(c, cash, 100.0);
        let pays = |from: PartyId, to: PartyId, amount: f64| Leg::Money {
            from,
            to,
            instrument: cash,
            amount,
            receipt: Receipt::Sale,
        };

        // A owes B and cannot pay; B owes C and cannot pay.
        let a_to_b = [pays(a, b, 100.0)];
        let b_to_c = [pays(b, c, 100.0)];
        assert_eq!(
            s.settle(&Instruction::plain(&a_to_b, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)),
            Outcome::Queued
        );
        assert_eq!(
            s.settle(&Instruction::plain(&b_to_c, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)),
            Outcome::Queued
        );
        assert_eq!(s.queue.census(), (2, 0, 0));

        // C pays A, and the whole chain goes through behind it.
        let c_to_a = [pays(c, a, 100.0)];
        assert_eq!(
            s.settle(&Instruction::plain(&c_to_a, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)),
            Outcome::Settled
        );
        assert_eq!(s.queue.census(), (0, 2, 0), "both waiting payments were taken, and neither was late");

        // And the money is back where it started.
        assert_eq!(reg.quantity(reg.row(c, cash)), 100.0);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 0.0);
        // A-20: and not one of the three is recorded as having failed anything.
        assert_eq!(j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).count(), 0);
    }

    #[test]
    fn a_ring_of_payers_with_nothing_between_them_settles_together_and_none_of_them_defaults() {
        // THE CASE THE RETRY CANNOT REACH.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (a, b, c) = (PartyId::at(0), PartyId::at(1), PartyId::at(2));
        let cash = InstrumentId::at(0);
        let pays = |from: PartyId, to: PartyId| {
            [Leg::Money { from, to, instrument: cash, amount: 70.0, receipt: Receipt::Sale }]
        };
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let legs = pays(from, to);
            assert_eq!(
                s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)),
                Outcome::Queued
            );
        }
        assert_eq!(s.queue.census(), (3, 0, 0));

        // The pass finds the ring and settles it as ONE instruction — three legs, each at its full
        // seventy, none of them cancelled against another.
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)), 3);
        assert_eq!(s.queue.census(), (0, 3, 0));
        let n = s.len() - 1;
        assert_eq!(s.outcome_of(n), Outcome::Settled);
        assert_eq!(s.legs_of(n).len(), 3, "every leg is on the wire at full value");

        // And everybody is exactly where they started, because that is what a ring of equal debts
        // IS.
        for who in [a, b, c] {
            assert_eq!(reg.quantity(reg.row(who, cash)), 0.0);
        }
        assert_eq!(j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).count(), 0);
    }

    #[test]
    fn a_ring_that_does_not_balance_is_not_forced_through() {
        // A cycle where somebody owes more than the cycle funds does not settle — it is refused and
        // stays queued, and nothing is clamped to make it fit.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (a, b) = (PartyId::at(0), PartyId::at(1));
        let cash = InstrumentId::at(0);
        let pays = |from: PartyId, to: PartyId, amount: f64| {
            [Leg::Money { from, to, instrument: cash, amount, receipt: Receipt::Sale }]
        };
        let out = pays(a, b, 100.0);
        let back = pays(b, a, 60.0);
        for legs in [&out, &back] {
            s.settle(&Instruction::plain(legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        }
        // A pays 100 and receives 60: it is forty short whichever order they go in.
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)), 0);
        assert_eq!(s.queue.census(), (2, 0, 0));

        // Give A the forty and it goes through, the ring and all.
        reg.money_delta(a, cash, 40.0);
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says)), 2);
        assert_eq!(reg.quantity(reg.row(b, cash)), 40.0);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
    }

    #[test]
    fn a_delivery_it_cannot_pay_for_still_fails_because_holding_it_open_would_sell_the_units_twice() {
        // The queue holds PAYMENTS.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (buyer, seller) = (PartyId::at(0), PartyId::at(1));
        let (cash, share) = (InstrumentId::at(0), InstrumentId::at(1));
        reg.credit(seller, share, 10.0, 3.0, 1);
        let legs = [
            Leg::Money { from: buyer, to: seller, instrument: cash, amount: 50.0, receipt: Receipt::Sale },
            Leg::Asset { from: seller, to: buyer, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfMoney);
        assert!(s.queue.is_empty());
        assert_eq!(reg.quantity(reg.row(seller, share)), 10.0, "XI-5: nothing moved");
    }

    #[test]
    fn a_payment_within_one_bank_never_touches_reserves() {
        // The ordinary case, and it has to stay ordinary: two customers of one bank settle on that
        // bank's books and the central bank never hears about it.
        let (mut reg, mut j, mut ps, mut ins, mut s, cal, says, who, lines) = two_banks();
        let one = who[1];
        let (reserves, ones) = (lines[0], lines[1]);
        let payer = who[3];
        let alongside = ps.add(0, RegionId::at(0), one, Representation::Named, 0);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);
        let legs = [Leg::Money { from: payer, to: alongside, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(alongside, ones)), 300.0);
        assert_eq!(reg.quantity(reg.row(one, reserves)), 800.0, "nothing left the bank");
    }

    #[test]
    fn what_comes_into_existence_over_the_wire_is_what_the_line_says_is_issued() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (maker, buyer) = (PartyId::at(0), PartyId::at(1));
        let good = InstrumentId::at(1);
        assert_eq!(ins.issued_of(good), 0.0, "a line exists before any of it does");

        let made = [Leg::Create { party: maker, instrument: good, qty: 60.0, cost_per_unit: 2.0 }];
        let out = s.settle(&Instruction::plain(&made, Cause::Production), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(ins.issued_of(good), 60.0);

        // A transfer moves units between holders and makes none, so the issue does not move.
        let sold = [Leg::Asset { from: maker, to: buyer, instrument: good, qty: 25.0, price_per_unit: None }];
        s.settle(&Instruction::free_of_payment(&sold, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(ins.issued_of(good), 60.0, "ownership changed; nothing came into existence");
        let (held, dust) = reg.held_total(good);
        assert!((held - ins.issued_of(good)).abs() <= dust, "Register B2");

        // And what is consumed stops existing on both sides at once.
        let eaten = [Leg::Destroy { party: buyer, instrument: good, qty: 10.0, why: Gone::Consumed }];
        s.settle(&Instruction::plain(&eaten, Cause::Production), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(ins.issued_of(good), 50.0);
        let (held, dust) = reg.held_total(good);
        assert!((held - ins.issued_of(good)).abs() <= dust, "Register B2 after a destruction");
    }

    #[test]
    fn two_legs_out_of_one_account_are_weighed_together_and_never_overdraw_it() {
        // Appendix B #5, Money B3.c: an overdraft is never a silent negative.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (a, b, c) = (PartyId::at(0), PartyId::at(1), PartyId::at(2));
        let cash = InstrumentId::at(0);
        reg.money_delta(a, cash, 60.0);
        let legs = [
            Leg::Money { from: a, to: b, instrument: cash, amount: 50.0, receipt: Receipt::Interest },
            Leg::Money { from: a, to: c, instrument: cash, amount: 50.0, receipt: Receipt::Interest },
        ];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        // A payment may WAIT — what it may not do is half-happen or go negative.
        assert_eq!(out, Outcome::Queued);
        assert_eq!(reg.quantity(reg.row(a, cash)), 60.0, "XI-5: nothing moved");
        assert_eq!(reg.quantity(reg.row(b, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(c, cash)), 0.0);

        // And with enough for both, both go — at full value, nothing netted.
        reg.money_delta(a, cash, 40.0);
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 50.0);
        assert_eq!(reg.quantity(reg.row(c, cash)), 50.0);
    }

    #[allow(clippy::type_complexity)]
    fn two_countries(
    ) -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes, [PartyId; 6], [InstrumentId; 4]) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
        let mut p = Parties::new();
        let (here, there) = (RegionId::at(0), RegionId::at(1));
        let cb_here = p.add(0, here, PartyId::NONE, Representation::Named, 0);
        let cb_there = p.add(0, there, PartyId::NONE, Representation::Named, 0);
        let bank_here = p.add(0, here, cb_here, Representation::Named, 0);
        let bank_there = p.add(0, there, cb_there, Representation::Named, 0);
        let payer = p.add(0, here, bank_here, Representation::Named, 0);
        let payee = p.add(0, there, bank_there, Representation::Named, 0);
        let mut i = Instruments::new();
        let unit = UnitId::at(0);
        let (a, b) = (CurrencyCode::at(0), CurrencyCode::at(1));
        let reserves_here = i.issue(cb_here, a, Class::Money, unit, None, None);
        let reserves_there = i.issue(cb_there, b, Class::Money, unit, None, None);
        let money_here = i.issue(bank_here, a, Class::Money, unit, None, None);
        let money_there = i.issue(bank_there, b, Class::Money, unit, None, None);
        (
            Register::new(),
            j,
            p,
            i,
            Settlement::new(6),
            Calendar::new(Day(0), 7, 3),
            says,
            [cb_here, cb_there, bank_here, bank_there, payer, payee],
            [reserves_here, reserves_there, money_here, money_there],
        )
    }

    #[test]
    fn a_payment_in_a_money_the_payee_cannot_hold_is_refused_and_never_converted() {
        // `amount` of one currency left and `amount` of another arrived, at a rate of one, with
        // nobody on the other side.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says, who, lines) = two_countries();
        let (bank_here, payer, payee) = (who[2], who[4], who[5]);
        let (reserves_here, money_here, money_there) = (lines[0], lines[2], lines[3]);
        reg.money_delta(payer, money_here, 500.0);
        reg.money_delta(bank_here, reserves_here, 900.0);

        let legs = [Leg::Money { from: payer, to: payee, instrument: money_here, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::NoAccountInThatMoney);

        // And nothing moved at all — not the payer's money, not the payee's, not a reserve.
        assert_eq!(reg.quantity(reg.row(payer, money_here)), 500.0);
        assert_eq!(reg.quantity(reg.row(payee, money_there)), 0.0);
        assert_eq!(reg.quantity(reg.row(payee, money_here)), 0.0);
        assert_eq!(reg.quantity(reg.row(bank_here, reserves_here)), 900.0);
        // It is a FAIL and not a queue: waiting does not give the payee an account.
        assert!(s.queue.is_empty(), "§12 F1: what the payer does is buy the money, not wait");
    }

    #[test]
    fn two_banks_with_no_reserve_line_in_common_cannot_settle_between_them() {
        let (mut reg, mut j, mut ps, mut ins, mut s, cal, says, who, lines) = two_countries();
        let (cb_there, bank_here) = (who[1], who[2]);
        let (reserves_here, money_here) = (lines[0], lines[2]);
        let a = CurrencyCode::at(0);
        // A bank abroad that issues the SAME money as here, at its own central bank.
        let bank_abroad = ps.add(0, RegionId::at(1), cb_there, Representation::Named, 0);
        let abroad = ps.add(0, RegionId::at(1), bank_abroad, Representation::Named, 0);
        let theirs = ins.issue(bank_abroad, a, Class::Money, UnitId::at(0), None, None);
        let payer = who[4];
        reg.money_delta(payer, money_here, 500.0);
        reg.money_delta(bank_here, reserves_here, 900.0);

        let legs = [Leg::Money { from: payer, to: abroad, instrument: money_here, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::BankCouldNotSettle);
        assert_eq!(reg.quantity(reg.row(payer, money_here)), 500.0, "XI-5: nothing moved");
        assert_eq!(reg.quantity(reg.row(abroad, theirs)), 0.0);
        assert_eq!(reg.quantity(reg.row(bank_here, reserves_here)), 900.0);
        // And it does not WAIT.
        assert!(s.queue.is_empty(), "22d.1: a missing reserve line is not a timing failure");
    }

    #[test]
    fn a_payment_home_to_its_own_currency_still_settles_across_two_banks() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says, who, lines) = two_banks();
        let (one, payer, payee) = (who[1], who[3], who[4]);
        let (reserves, ones, twos) = (lines[0], lines[1], lines[2]);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);
        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(payee, twos)), 300.0, "it landed in its own bank's money");
        assert_eq!(reg.quantity(reg.row(one, reserves)), 500.0, "and the reserves moved");
    }

    // The three leg kinds the pre-check skipped.

    #[test]
    fn an_issuer_mints_its_own_money_and_owes_it_to_whoever_ends_up_holding_it() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let bank = PartyId::at(9);
        let (holder, cash) = (PartyId::at(0), InstrumentId::at(0));
        let made = [Leg::Mint { issuer: bank, money: cash, amount: 700.0 }];
        let out = s.settle(&Instruction::plain(&made, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(bank, cash)), 700.0);
        // And what it OWES is what others hold of what it issued — read from the register, never a
        // tally beside it.
        fn owed(who: PartyId, reg: &Register, ins: &Instruments) -> f64 {
            crate::instruments::owed_by(who, ins, |i| {
                let (held, _) = reg.held_total(i);
                held - reg.quantity(reg.row(who, i))
            })
        }
        assert_eq!(owed(bank, &reg, &ins), 0.0, "unissued: it holds its own liability");
        let paid = [Leg::Money { from: bank, to: holder, instrument: cash, amount: 300.0, receipt: Receipt::Transfer }];
        let out = s.settle(&Instruction::plain(&paid, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(owed(bank, &reg, &ins), 300.0, "Money A1: every unit is owed by a named issuer");
        // And the issued amount is the OTHER side of that read — what exists of the line, moved by
        // the mint and by nothing else.
        assert_eq!(ins.issued_of(cash), 700.0);
        let (held, _) = reg.held_total(cash);
        assert_eq!(held, ins.issued_of(cash), "Register B2: holdings sum to the issued amount");
    }

    #[test]
    #[should_panic(expected = "money created from nothing")]
    fn a_party_cannot_mint_money_it_does_not_owe() {
        // Appendix B #1, the single most consequential FORBID in the document: a balance that is
        // nobody's liability.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let legs = [Leg::Mint { issuer: PartyId::at(0), money: InstrumentId::at(0), amount: 700.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "creates nothing")]
    fn a_negative_mint_is_not_a_mint() {
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let legs = [Leg::Mint { issuer: PartyId::at(9), money: InstrumentId::at(0), amount: -700.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "the lots its basis lives in")]
    fn minting_a_line_that_carries_lots_would_throw_its_basis_away() {
        // `money_delta` sets the row to a TOTAL.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let good = InstrumentId::at(1);
        let legs = [Leg::Mint { issuer: PartyId::at(1), money: good, amount: 5.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "`Leg::Mint` exists to draw")]
    fn money_is_never_created_on_a_holders_book() {
        // The other side of the same line: `Create` writes LOTS, and a money account has none.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let legs = [Leg::Create { party: PartyId::at(0), instrument: InstrumentId::at(0), qty: 700.0, cost_per_unit: 1.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
    }

    #[test]
    fn nothing_is_pledged_that_is_not_held_and_nothing_is_pledged_twice() {
        // Appendix B #9.
        let (mut reg, mut j, ps, mut ins, mut s, cal, says) = world();
        let (holder, lender) = (PartyId::at(0), PartyId::at(1));
        let share = InstrumentId::at(1);
        let over = [Leg::Pledge { holder, instrument: share, to: lender, qty: 10.0 }];
        let out = s.settle(&Instruction::plain(&over, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfUnits, "it holds none of it");

        reg.credit(holder, share, 10.0, 3.0, 1);
        let ok = [Leg::Pledge { holder, instrument: share, to: lender, qty: 6.0 }];
        let out = s.settle(&Instruction::plain(&ok, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.free(reg.row(holder, share)), 4.0);

        // And the four that are left cannot be pledged twice: the units are there and somebody else
        // has a claim over them, which is what `Encumbered` says.
        let again = [Leg::Pledge { holder, instrument: share, to: PartyId::at(2), qty: 6.0 }];
        let out = s.settle(&Instruction::plain(&again, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &mut ins, &cal, says));
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.free(reg.row(holder, share)), 4.0, "XI-5: the refusal moved nothing");
    }
}
