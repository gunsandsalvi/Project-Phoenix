//! The wire: state changes ONLY by a numbered, two-sided instruction applied by settlement, all legs
//! atomic (delivery-versus-payment, XI-5), settlement is FINAL, and a fail is a recorded state.

use crate::calendar::{Calendar, Week};
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Instruments;
use crate::journal::{Journal, Value};
use crate::parties::Parties;
use crate::register::Register;
/// A QUANTITY THAT MOVES, and it is positive by construction: a leg of nothing is not a leg, and a
/// negative one is the other direction wearing a minus sign.
///
/// `Mint` and `Create` refused a quantity of nothing at the wire and the other four leg kinds did
/// not, so a transfer of zero settled silently — an instruction number, a journal entry and a
/// settlement for a flow that moved nothing. The refusal belongs where the quantity is decided.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Units(f64);

impl Units {
    /// `None` where there is nothing to move, so the caller says what it does about that — usually
    /// propose nothing, which is an answer.
    pub fn new(of: f64) -> Option<Units> {
        match of > 0.0 && of.is_finite() {
            true => Some(Units(of)),
            false => None,
        }
    }

    #[inline]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Every flow has two sides, both legs, same pass, same week, same currency.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Leg {
    /// Money moving between two accounts.
    Money {
        from: PartyId,
        to: PartyId,
        instrument: InstrumentId,
        amount: Units,
        receipt: Receipt,
    },
    /// Units of an instrument moving between two holders, at a price if it is a trade.
    Asset {
        from: PartyId,
        to: PartyId,
        instrument: InstrumentId,
        qty: Units,
        price_per_unit: Option<f64>,
    },
    /// Goods B: a physical thing coming into existence.
    Create {
        party: PartyId,
        instrument: InstrumentId,
        qty: Units,
        cost_per_unit: f64,
    },
    /// An issuer creating its own money.
    Mint {
        issuer: PartyId,
        money: InstrumentId,
        amount: Units,
    },
    /// And a thing leaving it.
    Destroy {
        party: PartyId,
        instrument: InstrumentId,
        qty: Units,
        why: Gone,
    },
    /// A claim over units, which refuses their move rather than adjusting it.
    Pledge {
        holder: PartyId,
        instrument: InstrumentId,
        to: PartyId,
        qty: Units,
    },
    /// A settled reduction in the carrying basis of plant, without moving or recreating its units.
    Depreciate {
        party: PartyId,
        instrument: InstrumentId,
        amount: f64,
    },
    /// Physical carriage coupled to the title-and-money instruction it performs.
    Dispatch {
        shipper: PartyId,
        consignee: PartyId,
        owner: PartyId,
        carrier: PartyId,
        instrument: InstrumentId,
        from: crate::ids::RegionId,
        to: crate::ids::RegionId,
        qty: Units,
        carrier_capacity: f64,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gone {
    Consumed,
    Perished,
    Scrapped,
    Redeemed,
    WrittenOff,
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
    /// Money paid in FOR a claim on the payee itself. Audit B5's first named mover of an equity
    /// account, and the one receipt where the payee keeps what it received.
    Capital,
    /// One side of a reciprocal spot exchange. Both FX receipts must be present in one instruction.
    Fx,
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

/// Audit B5: what a receipt does to the accounts it touches. Money paid FOR something the payer
/// receives moves neither of them — what it cost is the basis the units carry, and what the sale
/// earned is the disposal's own gain — so only income, a cost and a claim on the payee say anything
/// here.
fn moves_equity(receipt: Receipt) -> Option<crate::stores::Moved> {
    match receipt {
        Receipt::Wage | Receipt::Tax | Receipt::Interest | Receipt::Dividend => {
            Some(crate::stores::Moved::Income)
        }
        Receipt::Capital => Some(crate::stores::Moved::CapitalPaidIn),
        Receipt::Sale | Receipt::Principal | Receipt::Transfer | Receipt::Fx => None,
    }
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
    /// The carrier has no room left this week. 38 D6: capacity rations the quantity, so it has to
    /// be able to turn a shipper away.
    NoCapacity,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DueOutcome {
    Settled { on: Week, paid: f64 },
    Queued { until: Week },
    Failed { on: Week, outcome: Outcome },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DueUpdate {
    pub due: crate::stores::DueId,
    pub outcome: DueOutcome,
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
    /// weeks.
    pub calendar: &'a Calendar,
    /// What an outcome is SAID under.
    pub says: Outcomes,
    /// Audit B5: the account each named event moves. Settlement writes it because settlement is
    /// where the flow and both its sides are, and a movement booked anywhere else is one side.
    pub equity: &'a mut crate::stores::Equity,
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
    /// And the kind a weight moving is said under, so a population event has a dated row to name.
    pub population: u32,
}

impl Outcomes {
    /// The kinds the kernel itself says under, declared once on a journal.
    pub fn declared(journal: &mut Journal) -> Self {
        Self {
            settled: journal.kinds.declare("instruction.settled"),
            failed: journal.kinds.declare("instruction.failed"),
            queued: journal.kinds.declare("instruction.queued"),
            closed: journal.kinds.declare("process.closed"),
            realised: journal.kinds.declare("disposal.realised"),
            population: journal.kinds.declare("population.moved"),
        }
    }
}

/// The account a party pays out of and is paid into.
pub fn account_of(
    parties: &Parties,
    instruments: &Instruments,
    p: PartyId,
) -> Option<InstrumentId> {
    let bank = parties.bank_of(p);
    instruments.money_issued_by(if bank.some() { bank } else { p })
}

/// Where a payment lands.
#[derive(Clone, Copy)]
pub(crate) enum Across {
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
pub(crate) fn across(
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
    let settles_in = |b: PartyId| {
        match account_of(parties, instruments, b) {
        Some(r) => r,
        None => panic!(
            "Money D2, 31 A1: bank {} settles in no money, so two banks have no way to settle between them",
            b.0
        ),
    }
    };
    let (mine, theirs) = (settles_in(payers_bank), settles_in(payees_bank));
    if mine != theirs {
        return Across::Refused(Outcome::BankCouldNotSettle, payers_bank);
    }
    Across::Banks {
        payers_bank,
        payees_bank,
        payees_money,
        reserves: mine,
    }
}

/// A spot exchange is two reciprocal money legs in different currencies.  Those legs land as the
/// named monies themselves; routing either through the recipient's ordinary account would silently
/// convert it and destroy the position the exchange exists to create.
pub(crate) fn is_exchange_leg(leg: &Leg, legs: &[Leg], instruments: &Instruments) -> bool {
    let Leg::Money {
        from,
        to,
        instrument,
        receipt: Receipt::Fx,
        ..
    } = *leg
    else {
        return false;
    };
    legs.iter().any(|other| match *other {
        Leg::Money {
            from: back_from,
            to: back_to,
            instrument: other_money,
            receipt: Receipt::Fx,
            ..
        } => {
            back_from == to
                && back_to == from
                && instruments.class_of(instrument) == crate::instruments::Class::Money
                && instruments.class_of(other_money) == crate::instruments::Class::Money
                && instruments.ccy_of(instrument) != instruments.ccy_of(other_money)
        }
        _ => false,
    })
}

/// WHAT THESE LEGS ARE, read off the legs themselves rather than taken from what the writer said.
///
/// A delivery with money moving between the same two parties is delivery-versus-payment; one with
/// money that has nothing to do with it is free of payment, and somebody is being trusted.
pub fn shape_of(legs: &[Leg]) -> Delivery {
    let mut deliveries: Vec<(PartyId, PartyId)> = Vec::new();
    let mut money: Vec<(PartyId, PartyId)> = Vec::new();
    let mut redemptions: Vec<PartyId> = Vec::new();
    for leg in legs {
        match *leg {
            Leg::Asset { from, to, .. } if from != to => deliveries.push((from, to)),
            Leg::Money { from, to, .. } if from != to => money.push((from, to)),
            Leg::Destroy {
                party,
                why: Gone::Redeemed,
                ..
            } => redemptions.push(party),
            _ => {}
        }
    }
    for holder in redemptions {
        if let Some((issuer, _)) = money.iter().find(|(_, payee)| *payee == holder) {
            deliveries.push((holder, *issuer));
        }
    }
    if deliveries.is_empty() {
        return Delivery::Nothing;
    }
    // Against: the payer received units here, or the payee delivered them.
    let against = money
        .iter()
        .any(|(mf, mt)| deliveries.iter().any(|(df, dt)| mf == dt || mt == df));
    match against {
        true => Delivery::AgainstPayment,
        false => Delivery::Free,
    }
}

pub struct Instruction<'a> {
    pub legs: &'a [Leg],
    pub cause: Cause,
    /// What the writer says this is.
    pub delivery: Delivery,
    /// The contractual due this instruction performs, if it performs one.
    pub due: Option<crate::stores::DueId>,
}

impl<'a> Instruction<'a> {
    /// The ordinary way.
    pub fn against_payment(legs: &'a [Leg], cause: Cause) -> Self {
        Self {
            legs,
            cause,
            delivery: Delivery::AgainstPayment,
            due: None,
        }
    }

    /// Free of payment: the deliverer performs and takes the other side on trust.
    pub fn free_of_payment(legs: &'a [Leg], cause: Cause) -> Self {
        Self {
            legs,
            cause,
            delivery: Delivery::Free,
            due: None,
        }
    }

    /// A payment, a thing made, a thing that perished: nothing is delivered against anything.
    pub fn plain(legs: &'a [Leg], cause: Cause) -> Self {
        Self {
            legs,
            cause,
            delivery: Delivery::Nothing,
            due: None,
        }
    }

    /// What the LEGS say this is, so the declaration can be held to them.
    fn shape(&self) -> Delivery {
        shape_of(self.legs)
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
        match *leg {
            Leg::Mint {
                issuer,
                money,
                amount,
            } => moves(issuer, money, amount.get()),
            Leg::Money {
                from,
                to,
                instrument,
                amount,
                ..
            } => {
                moves(from, instrument, -amount.get());
                if is_exchange_leg(leg, legs, instruments) {
                    moves(to, instrument, amount.get());
                    continue;
                }
                match across(parties, instruments, to, instrument) {
                    Across::Same => moves(to, instrument, amount.get()),
                    Across::Banks {
                        payers_bank,
                        payees_bank,
                        payees_money,
                        reserves,
                    } => {
                        moves(to, payees_money, amount.get());
                        // And the reserves the banks move between them, which net over a cycle exactly
                        // as the customers' deposits do.
                        moves(payers_bank, reserves, -amount.get());
                        moves(payees_bank, reserves, amount.get());
                    }
                    // A leg that cannot land at all is refused by the pass above this one, before any
                    // balance is read — so by here there is nothing left to be short of.
                    Across::Refused(..) => {}
                }
            }
            _ => {}
        }
    }
    let short = |who: PartyId, what: InstrumentId| match delta.get(&(who.0, what.0)) {
        Some(by) => *by < 0.0 && reg.quantity(reg.row(who, what)) + by < 0.0,
        None => false,
    };
    for leg in legs {
        if let Leg::Money {
            from,
            to,
            instrument,
            ..
        } = *leg
        {
            if short(from, instrument) {
                return Some((Outcome::ShortOfMoney, from));
            }
            // And the payer's BANK needs the reserves to settle it across.
            if let Across::Banks {
                payers_bank,
                reserves,
                ..
            } = across(parties, instruments, to, instrument)
            {
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
    due: Vec<Option<crate::stores::DueId>>,
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
            due: Vec::new(),
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
    pub fn joins(
        &mut self,
        ins: &Instruction<'_>,
        payer: PartyId,
        on: Week,
        late_after: Week,
    ) -> QueueId {
        assert!(
            payer.some(),
            "A-20: a payment that is nobody's is not queued"
        );
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
        self.due.push(ins.due);
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

    pub fn due_of(&self, q: QueueId) -> Option<crate::stores::DueId> {
        self.due[q.row()]
    }

    /// Rebuild the exact instruction that joined the queue. Retrying never peels the money leg
    /// away from delivery, and expiry never applies either half.
    pub fn retry_of(
        &self,
        q: QueueId,
    ) -> (Vec<Leg>, Cause, Delivery, Option<crate::stores::DueId>) {
        (
            self.legs_of(q).to_vec(),
            self.cause_of(q),
            self.delivery_of(q),
            self.due_of(q),
        )
    }

    pub fn payer_of(&self, q: QueueId) -> PartyId {
        PartyId(self.payer[q.row()])
    }

    pub fn state_of(&self, q: QueueId) -> Waiting {
        self.state[q.row()]
    }

    pub fn queued_on(&self, q: QueueId) -> Week {
        Week(self.queued_on[q.row()])
    }

    pub fn late_after(&self, q: QueueId) -> Week {
        Week(self.late_after[q.row()])
    }

    /// A SELLER AGREED TO WAIT.
    pub fn given_time(&mut self, q: QueueId, until: Week) {
        assert!(
            self.state[q.row()] == Waiting::Queued,
            "36 C1: only a waiting payment is given time"
        );
        assert!(
            until.0 > self.late_after[q.row()],
            "36 C1: terms that end sooner than the payment's own day are not time given"
        );
        self.late_after[q.row()] = until.0;
    }

    /// It went through on a retry.
    pub fn took(&mut self, q: QueueId, on: Week) {
        assert!(
            self.state[q.row()] == Waiting::Queued,
            "22d.1: only a waiting payment is taken"
        );
        self.state[q.row()] = Waiting::Taken;
        self.finished_on[q.row()] = Some(on.0);
    }

    /// Its days ran out.
    pub fn gave_up(&mut self, q: QueueId, on: Week) {
        assert!(
            self.state[q.row()] == Waiting::Queued,
            "22d.1: only a waiting payment gives up"
        );
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
    pub fn out_of_days(&self, on: Week) -> Vec<QueueId> {
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
            .filter(|q| {
                self.legs_of(*q)
                    .iter()
                    .all(|l| matches!(l, Leg::Money { .. }))
            })
            .collect();
        let mut owes: std::collections::HashMap<u32, Vec<QueueId>> =
            std::collections::HashMap::new();
        for q in &links {
            owes.entry(self.payer_of(*q).0).or_default().push(*q);
        }
        // A party the walk has left behind without finding a ring through it is not walked again:
        // the answer would be the same, and this is what keeps the search over the whole queue.
        let mut done: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for q in &links {
            let mut path: Vec<QueueId> = Vec::new();
            let mut at: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
            if let Some(ring) =
                self.ring_from(self.payer_of(*q).0, &owes, &mut path, &mut at, &mut done)
            {
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
        self.between(Week(i64::MIN), Week(i64::MAX))
    }

    /// WHAT BECAME OF THE PAYMENTS THAT WERE SHORT, between two days — how many are still waiting,
    /// how many went through after waiting, how many ran out of days.
    pub fn between(&self, from: Week, to: Week) -> (usize, usize, usize) {
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
    /// The contractual due, when the instruction was performance of one. Kept beside the outcome
    /// so an audit can independently reconcile the wire with the schedule store.
    due: Vec<Option<crate::stores::DueId>>,
    at_period: Vec<u32>,
    cause: Vec<Cause>,
    leg_at: Vec<u32>,
    leg_len: Vec<u32>,
    legs: Vec<Leg>,
    /// Where each week's instructions begin and end, written as they arrive.
    by_period: Vec<(u32, u32, u32)>,
    /// Every free delivery: who performed, who was trusted, and when.
    delivered_free: Vec<(PartyId, PartyId, u32)>,
    due_updates: Vec<DueUpdate>,
    /// The payments it could not make yet.
    pub queue: Queue,
    /// Physical dispatches settle on the same wire as title and consideration.
    pub dispatches: crate::mechanisms::freight::Dispatches,
    /// One TECHNOLOGY: how many PERIODS a payment may wait before it is late. A week settles once
    /// (Money G1), so a payment that cannot be made waits a whole one or none at all — a lifetime in
    /// days against a clock with no days in it is the second calendar G3.c forbids.
    waits_for: u32,
}

impl Settlement {
    pub fn new(waits_for: u32) -> Self {
        assert!(
            waits_for > 0,
            "Money G1: a payment that may wait no week does not wait"
        );
        Self {
            outcomes: Vec::new(),
            due: Vec::new(),
            at_period: Vec::new(),
            cause: Vec::new(),
            leg_at: Vec::new(),
            leg_len: Vec::new(),
            legs: Vec::new(),
            by_period: Vec::new(),
            delivered_free: Vec::new(),
            due_updates: Vec::new(),
            queue: Queue::new(),
            dispatches: crate::mechanisms::freight::Dispatches::new(),
            waits_for,
        }
    }

    /// How many weeks a payment may wait here, as this payment system was built.
    pub fn waits_for(&self) -> u32 {
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

    pub fn due_of(&self, n: usize) -> Option<crate::stores::DueId> {
        self.due[n]
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

    pub fn take_due_updates(&mut self) -> Vec<DueUpdate> {
        std::mem::take(&mut self.due_updates)
    }

    /// This week's instructions, without walking the history.
    pub fn in_period(&self, week: u32) -> std::ops::Range<usize> {
        for &(p, from, to) in &self.by_period {
            if p == week {
                return from as usize..to as usize;
            }
        }
        0..0
    }

    /// ALL LEGS OR NONE.
    pub fn settle(&mut self, ins: &Instruction<'_>, week: u32, on: &mut Settling<'_>) -> Outcome {
        let out = self.attempt(ins, week, on, Presented::Fresh);
        if let Some(due) = ins.due {
            let today = on.calendar.at(Week(i64::from(week)));
            let outcome = match out {
                Outcome::Settled => DueOutcome::Settled {
                    on: today,
                    paid: ins
                        .legs
                        .iter()
                        .filter_map(|leg| match leg {
                            Leg::Money { amount, .. } => Some(amount.get()),
                            _ => None,
                        })
                        .sum(),
                },
                Outcome::Queued => DueOutcome::Queued {
                    until: on.calendar.at(Week(i64::from(week + self.waits_for))),
                },
                failed => DueOutcome::Failed {
                    on: today,
                    outcome: failed,
                },
            };
            self.due_updates.push(DueUpdate { due, outcome });
        }
        if out == Outcome::Settled {
            self.release(ins.legs, week, on);
        }
        out
    }

    /// A receipt is a retry.
    fn release(&mut self, legs: &[Leg], week: u32, on: &mut Settling<'_>) {
        let today = on.calendar.at(Week(i64::from(week)));
        let mut funded: Vec<PartyId> = paid_by(legs);
        while let Some(who) = funded.pop() {
            for q in self.queue.of_payer(who) {
                let (waiting, cause, delivery, due) = self.queue.retry_of(q);
                let ins = Instruction {
                    legs: &waiting,
                    cause,
                    delivery,
                    due,
                };
                if self.attempt(&ins, week, on, Presented::Retry) == Outcome::Settled {
                    self.queue.took(q, today);
                    if let Some(due) = ins.due {
                        self.due_updates.push(DueUpdate {
                            due,
                            outcome: DueOutcome::Settled {
                                on: today,
                                paid: waiting
                                    .iter()
                                    .filter_map(|leg| match leg {
                                        Leg::Money { amount, .. } => Some(amount.get()),
                                        _ => None,
                                    })
                                    .sum(),
                            },
                        });
                    }
                    funded.extend(paid_by(&waiting));
                }
            }
        }
    }

    /// ONE PASS THAT FINDS THE CYCLES AND SETTLES THEM TOGETHER.
    pub fn unwind(&mut self, week: u32, on: &mut Settling<'_>) -> usize {
        let today = on.calendar.at(Week(i64::from(week)));
        let mut went = 0usize;
        // The cycles that were found and did not balance — somebody in them is short beyond what the
        // cycle itself funds.
        let mut stuck: std::collections::HashSet<u32> = std::collections::HashSet::new();
        while let Some(rows) = self.queue.a_cycle(&stuck) {
            let legs: Vec<Leg> = rows
                .iter()
                .flat_map(|q| self.queue.legs_of(*q).to_vec())
                .collect();
            let ins = Instruction {
                legs: &legs,
                cause: Cause::Settlement,
                delivery: Delivery::Nothing,
                due: None,
            };
            if self.attempt(&ins, week, on, Presented::Together) == Outcome::Settled {
                for q in &rows {
                    self.queue.took(*q, today);
                    if let Some(due) = self.queue.due_of(*q) {
                        self.due_updates.push(DueUpdate {
                            due,
                            outcome: DueOutcome::Settled {
                                on: today,
                                paid: self
                                    .queue
                                    .legs_of(*q)
                                    .iter()
                                    .filter_map(|leg| match leg {
                                        Leg::Money { amount, .. } => Some(amount.get()),
                                        _ => None,
                                    })
                                    .sum(),
                            },
                        });
                    }
                }
                went += rows.len();
                // And a cycle that settled has paid people, so whatever THAT funds goes too.
                self.release(&legs, week, on);
            } else {
                stuck.extend(rows.iter().map(|q| q.0));
            }
        }
        went
    }

    /// The queue's day passed.
    pub fn give_up(&mut self, today: Week, week: u32, on: &mut Settling<'_>) -> usize {
        let done = self.queue.out_of_days(today);
        for q in &done {
            let (waiting, cause, delivery, due) = self.queue.retry_of(*q);
            let ins = Instruction {
                legs: &waiting,
                cause,
                delivery,
                due,
            };
            let who = self.queue.payer_of(*q);
            self.queue.gave_up(*q, today);
            if let Some(due) = ins.due {
                self.due_updates.push(DueUpdate {
                    due,
                    outcome: DueOutcome::Failed {
                        on: today,
                        outcome: Outcome::ShortOfMoney,
                    },
                });
            }
            let failed = on.says.failed;
            self.record(Outcome::ShortOfMoney, who, &ins, week, on.journal, failed);
        }
        done.len()
    }

    fn attempt(
        &mut self,
        ins: &Instruction<'_>,
        week: u32,
        on: &mut Settling<'_>,
        how: Presented,
    ) -> Outcome {
        let may_queue = how == Presented::Fresh;
        let Settling {
            register: reg,
            journal,
            parties,
            instruments,
            calendar,
            says,
            equity,
        } = on;
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
                    if is_exchange_leg(leg, ins.legs, instruments) {
                        continue;
                    }
                    // Where it lands is asked FIRST, before any balance is read.
                    if let Across::Refused(outcome, who) =
                        across(parties, instruments, to, instrument)
                    {
                        return self.record(outcome, who, ins, week, journal, failed_kind);
                    }
                }
                Leg::Asset {
                    from,
                    instrument,
                    qty,
                    ..
                } => {
                    let row = reg.row(from, instrument);
                    if !row.some() {
                        return self.record(
                            Outcome::ShortOfUnits,
                            from,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                    if reg.quantity(row) < qty.get() {
                        return self.record(
                            Outcome::ShortOfUnits,
                            from,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                    if reg.free(row) < qty.get() {
                        return self.record(
                            Outcome::Encumbered,
                            from,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                }
                Leg::Destroy {
                    party,
                    instrument,
                    qty,
                    ..
                } => {
                    let row = reg.row(party, instrument);
                    if reg.free(row) < qty.get() {
                        return self.record(
                            Outcome::ShortOfUnits,
                            party,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                }
                // NO MONEY WITHOUT AN ISSUER, checked at the one site in this engine that creates
                // money.
                Leg::Mint { issuer, money, .. } => {
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
                }
                // The other side of the same line.
                Leg::Create { instrument, .. } => {
                    assert!(
                        instruments.class_of(instrument) != crate::instruments::Class::Money,
                        "Money A1, D2: money is MINTED by the party that owes it and never created \
                         on a holder's book — instrument {} is money, and that is the line \
                         `Leg::Mint` exists to draw",
                        instrument.0
                    );
                }
                // Appendix B #9, Register C3: a lien over units nobody holds.
                Leg::Pledge {
                    holder,
                    instrument,
                    qty,
                    ..
                } => {
                    let row = reg.row(holder, instrument);
                    if !row.some() || reg.quantity(row) < qty.get() {
                        return self.record(
                            Outcome::ShortOfUnits,
                            holder,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                    if reg.free(row) < qty.get() {
                        return self.record(
                            Outcome::Encumbered,
                            holder,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                }
                Leg::Depreciate {
                    party,
                    instrument,
                    amount,
                } => {
                    assert!(
                        instruments.class_of(instrument) == crate::instruments::Class::Plant,
                        "Capital Programme D1: only plant may be depreciated"
                    );
                    assert!(
                        amount.is_finite() && amount > 0.0,
                        "Capital Programme D1: depreciation must be positive and finite"
                    );
                    let row = reg.row(party, instrument);
                    assert!(
                        row.some(),
                        "Capital Programme D1: depreciation needs a plant holding"
                    );
                    let carrying: f64 = reg
                        .lots(row)
                        .iter()
                        .map(|lot| lot.qty * lot.basis_per_unit)
                        .sum();
                    assert!(
                        amount <= carrying,
                        "Capital Programme D1: depreciation exceeds carrying basis"
                    );
                }
                Leg::Dispatch {
                    carrier,
                    qty,
                    carrier_capacity,
                    ..
                } => {
                    assert!(
                        carrier_capacity.is_finite() && carrier_capacity > 0.0,
                        "38 E2: dispatch needs positive carrier capacity"
                    );
                    // 38 D6: a shipper that cannot be carried is TURNED AWAY. The instruction is
                    // refused whole, so nothing moves and the goods stay where they were.
                    if self.dispatches.used(week, carrier) + qty.get() > carrier_capacity {
                        return self.record(
                            Outcome::NoCapacity,
                            carrier,
                            ins,
                            week,
                            journal,
                            failed_kind,
                        );
                    }
                }
            }
        }
        // And what the money legs do to each holding TOGETHER, which is the only test that is right
        // for an instruction with two legs out of one account.
        if let Some((outcome, who)) = short_together(ins.legs, reg, parties, instruments) {
            return self.short(outcome, who, ins, week, journal, calendar, says, may_queue);
        }
        // The application.

        for leg in ins.legs {
            match *leg {
                Leg::Money {
                    from,
                    to,
                    instrument,
                    amount,
                    receipt,
                } => {
                    // Audit B5: what the flow does to the two accounts, said by the receipt its
                    // writer declared rather than derived from the shape of the instruction.
                    if let Some(why) = moves_equity(receipt) {
                        // Capital is a claim the payee issues, so only the payee's account moves:
                        // what the payer handed over came back to it as the paper it bought.
                        if why != crate::stores::Moved::CapitalPaidIn {
                            equity.moves(from, -amount.get(), why, week);
                        }
                        equity.moves(to, amount.get(), why, week);
                    }
                    // THE INTERBANK LEG.
                    reg.money_delta(from, instrument, -amount.get());
                    if is_exchange_leg(leg, ins.legs, instruments) {
                        reg.money_delta(to, instrument, amount.get());
                        continue;
                    }
                    match across(parties, instruments, to, instrument) {
                        Across::Same => {
                            reg.money_delta(to, instrument, amount.get());
                        }
                        Across::Banks {
                            payers_bank,
                            payees_bank,
                            payees_money,
                            reserves,
                        } => {
                            reg.money_delta(to, payees_money, amount.get());
                            reg.money_delta(payers_bank, reserves, -amount.get());
                            reg.money_delta(payees_bank, reserves, amount.get());
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
                Leg::Asset {
                    from,
                    to,
                    instrument,
                    qty,
                    price_per_unit,
                } => {
                    let row = reg.row(from, instrument);
                    let drawn = reg.debit(row, qty.get());
                    // What the units cost goes with them where it is a transfer, and the price is
                    // the basis where a market struck one.
                    match price_per_unit {
                        Some(price) => {
                            // And what the seller REALISED, which is the one thing only this line
                            // knows: the proceeds against what the lots that left cost.
                            let cost: f64 = drawn.iter().map(|d| d.qty * d.basis_per_unit).sum();
                            realised.push((from, instrument, qty.get() * price - cost));
                            reg.credit(to, instrument, qty.get(), price, week);
                        }
                        None => {
                            for d in &drawn {
                                reg.credit(to, instrument, d.qty, d.basis_per_unit, week);
                            }
                        }
                    }
                }
                Leg::Create {
                    party,
                    instrument,
                    qty,
                    cost_per_unit,
                } => {
                    reg.credit(party, instrument, qty.get(), cost_per_unit, week);
                    // Register B1, Goods B: and that many units of the line now EXIST.
                    instruments.moves(instrument, crate::instruments::Issuance::Made, qty.get());
                }
                // The issuer's own money, as a TOTAL, and NO equity — what it created is what it
                // owes, and `Instruments::owed_by` reads that from issued against held.
                Leg::Mint {
                    issuer,
                    money,
                    amount,
                } => {
                    reg.money_delta(issuer, money, amount.get());
                    // Money issued is money that exists, and the count moves here for the same
                    // reason a line's does — it is the one place it comes into being.
                    instruments.moves(money, crate::instruments::Issuance::Made, amount.get());
                }
                Leg::Destroy {
                    party,
                    instrument,
                    qty,
                    why,
                } => {
                    let row = reg.row(party, instrument);
                    if instruments.class_of(instrument) == crate::instruments::Class::Money {
                        // Money holdings are totals rather than lot positions. Destroying redeemed
                        // money therefore uses the same total door as minting and payment; routing
                        // it through `debit` would draw no lots and leave the balance untouched.
                        reg.money_delta(party, instrument, -qty.get());
                        instruments.moves(
                            instrument,
                            crate::instruments::Issuance::Gone,
                            qty.get(),
                        );
                        continue;
                    }
                    let drawn = reg.debit(row, qty.get());
                    // What perished cost something, and the loss is an EVENT rather than a number
                    // that quietly stops existing.
                    let cost: f64 = drawn.iter().map(|d| d.qty * d.basis_per_unit).sum();
                    let proceeds = if why == Gone::Redeemed {
                        ins.legs
                            .iter()
                            .filter_map(|leg| match leg {
                                Leg::Money {
                                    to,
                                    amount,
                                    receipt: Receipt::Principal,
                                    ..
                                } if *to == party => Some(amount.get()),
                                _ => None,
                            })
                            .sum()
                    } else {
                        0.0
                    };
                    if proceeds != cost {
                        realised.push((party, instrument, proceeds - cost));
                    }
                    // And there is that much less of it in the world.
                    instruments.moves(instrument, crate::instruments::Issuance::Gone, qty.get());
                }
                Leg::Pledge {
                    holder,
                    instrument,
                    to,
                    qty,
                } => {
                    reg.pledge(holder, instrument, to, qty.get());
                }
                Leg::Depreciate {
                    party,
                    instrument,
                    amount,
                } => {
                    let row = reg.row(party, instrument);
                    reg.depreciate(row, amount);
                }
                Leg::Dispatch {
                    shipper,
                    consignee,
                    owner,
                    carrier,
                    instrument,
                    from,
                    to,
                    qty,
                    ..
                } => {
                    self.dispatches
                        .record(crate::mechanisms::freight::Dispatch {
                            week,
                            shipper,
                            consignee,
                            owner,
                            carrier,
                            what: instrument,
                            on: crate::mechanisms::freight::Route { from, to },
                            units: qty.get(),
                            arrives: week + 1,
                        });
                }
            }
        }
        // A free delivery is a risk somebody took, so the record NAMES who took it.
        if ins.delivery == Delivery::Free {
            for leg in ins.legs {
                if let Leg::Asset { from, to, .. } = *leg {
                    if from != to {
                        self.delivered_free.push((from, to, week));
                    }
                }
            }
        }
        // And what the disposals realised, now that every leg has applied.
        for (who, line, amount) in realised {
            equity.moves(who, amount, crate::stores::Moved::Landed, week);
            journal.say(
                week,
                realised_kind,
                &[who.0],
                &[
                    (0, crate::journal::Value::Num(amount)),
                    (1, crate::journal::Value::Num(f64::from(line.0))),
                ],
                false,
            );
        }
        self.record(
            Outcome::Settled,
            PartyId::NONE,
            ins,
            week,
            journal,
            settled_kind,
        )
    }

    /// Short of money is a QUEUE and not an arrear — where the instruction is a payment.
    #[allow(clippy::too_many_arguments)]
    fn short(
        &mut self,
        outcome: Outcome,
        who: PartyId,
        ins: &Instruction<'_>,
        week: u32,
        journal: &mut Journal,
        calendar: &Calendar,
        says: Outcomes,
        may_queue: bool,
    ) -> Outcome {
        if !may_queue || ins.delivery != Delivery::Nothing {
            return self.record(outcome, who, ins, week, journal, says.failed);
        }
        let today = calendar.at(Week(i64::from(week)));
        // It waits whole weeks, because there is nothing finer for it to wait.
        let late_after = calendar.at(Week(i64::from(week + self.waits_for)));
        self.queue.joins(ins, who, today, late_after);
        self.record(Outcome::Queued, who, ins, week, journal, says.queued)
    }

    fn record(
        &mut self,
        outcome: Outcome,
        // WHOSE failure it is: a fail with no subjects is a recorded state nobody can find by
        // looking for their own.
        on: PartyId,
        ins: &Instruction<'_>,
        week: u32,
        journal: &mut Journal,
        kind: u32,
    ) -> Outcome {
        let n = self.outcomes.len() as u32;
        self.outcomes.push(outcome);
        self.due.push(ins.due);
        self.at_period.push(week);
        self.cause.push(ins.cause);
        self.leg_at.push(self.legs.len() as u32);
        self.leg_len.push(ins.legs.len() as u32);
        self.legs.extend_from_slice(ins.legs);
        match self.by_period.last_mut() {
            Some(last) if last.0 == week => last.2 = n + 1,
            _ => self.by_period.push((week, n, n + 1)),
        }
        let subjects: &[u32] = if on.some() { &[on.0] } else { &[] };
        journal.say(
            week,
            kind,
            subjects,
            &[(0, Value::Num(ins.legs.len() as f64))],
            true,
        );
        outcome
    }
}

// THE WIRE IS MEASURED AGAINST THE REAL WORLD, not against two parties in a fixture.
//
// What the thirty tests here asserted was conservation — a refused instruction moves nothing, both
// legs land in the same pass, a payment makes nobody richer, reserves move between two banks and
// not within one, what comes into existence is what the line says is issued. Those are exactly the
// questions the audit families ask of every instruction the world settles, every week: Money,
// Flows, Names, Units and Ownership run over 1.9M events a week and report an owner and a size,
// where a fixture arranged five parties and asked once.
//
// What the wire REFUSES it refuses at the site, and the refusal is typed: an `Outcome` the caller
// must match, or a panic naming the clause — minting money somebody else owes, minting a line that
// carries lots, creating money on a holder's book, a delivery whose money leg went missing, a lien
// over units nobody holds. None of them needs a world to state.
//
// One type lift is left and it is not small: `Leg`'s quantities are bare `f64`, so "a mint of 0
// creates nothing" and "0 units is not a thing coming into existence" are asserts where they could
// be unconstructible. That is 69 engine call sites across every mechanism, so it is a step of its
// own — 0m3.3a.

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn line(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn pays(from: u32, to: u32, amount: f64) -> Leg {
        Leg::Money {
            from: party(from),
            to: party(to),
            instrument: line(0),
            amount: Units::new(amount).expect("a payment moves something"),
            receipt: Receipt::Sale,
        }
    }

    fn delivers(from: u32, to: u32, qty: f64) -> Leg {
        Leg::Asset {
            from: party(from),
            to: party(to),
            instrument: line(1),
            qty: Units::new(qty).expect("a delivery moves something"),
            price_per_unit: None,
        }
    }

    fn dispatches(from: u32, to: u32, qty: f64) -> Leg {
        Leg::Dispatch {
            shipper: party(from),
            consignee: party(to),
            owner: party(to),
            carrier: party(90),
            instrument: line(1),
            from: crate::ids::RegionId::at(1),
            to: crate::ids::RegionId::at(2),
            qty: Units::new(qty).unwrap(),
            carrier_capacity: 100.0,
        }
    }

    #[test]
    fn money_between_the_two_parties_a_delivery_is_between_is_payment_against_it() {
        // One delivers, the other pays: delivery-versus-payment, and neither side is trusted.
        assert_eq!(
            shape_of(&[delivers(1, 2, 10.0), pays(2, 1, 100.0)]),
            Delivery::AgainstPayment
        );
        // The order of the legs is not what decides it.
        assert_eq!(
            shape_of(&[pays(2, 1, 100.0), delivers(1, 2, 10.0)]),
            Delivery::AgainstPayment
        );
    }

    #[test]
    fn a_delivery_with_money_that_has_nothing_to_do_with_it_is_free_of_payment() {
        // Somebody is being trusted, and the wire says so rather than calling it a trade.
        assert_eq!(
            shape_of(&[delivers(1, 2, 10.0), pays(3, 4, 100.0)]),
            Delivery::Free
        );
        // And a delivery with no money at all is free of payment too.
        assert_eq!(shape_of(&[delivers(1, 2, 10.0)]), Delivery::Free);
    }

    #[test]
    fn an_instruction_that_delivers_nothing_delivers_nothing() {
        assert_eq!(shape_of(&[pays(1, 2, 100.0)]), Delivery::Nothing);
        assert_eq!(shape_of(&[]), Delivery::Nothing);
        // A leg from a party to itself moves nothing, so it is not a delivery.
        assert_eq!(shape_of(&[delivers(1, 1, 10.0)]), Delivery::Nothing);
    }

    #[test]
    fn it_is_the_two_sides_of_the_delivery_that_decide_and_not_who_the_money_went_to() {
        // The party that RECEIVED the units is paying — whoever it pays.
        assert_eq!(
            shape_of(&[delivers(1, 2, 10.0), pays(2, 3, 100.0)]),
            Delivery::AgainstPayment
        );
        // And the party that DELIVERED them is being paid — by whoever.
        assert_eq!(
            shape_of(&[delivers(1, 2, 10.0), pays(3, 1, 100.0)]),
            Delivery::AgainstPayment
        );
        // Free is money touching neither side of the delivery.
        assert_eq!(
            shape_of(&[delivers(1, 2, 10.0), pays(3, 4, 100.0)]),
            Delivery::Free
        );
    }

    #[test]
    fn a_queued_delivery_retries_as_the_same_atomic_dvp_instruction() {
        let legs = [
            delivers(1, 2, 10.0),
            pays(2, 1, 100.0),
            dispatches(1, 2, 10.0),
        ];
        let instruction = Instruction::against_payment(&legs, Cause::Trade);
        let mut queue = Queue::new();
        let queued = queue.joins(&instruction, party(2), Week(10), Week(20));

        let (retry, cause, delivery, due) = queue.retry_of(queued);
        assert_eq!(retry, legs);
        assert_eq!(cause, Cause::Trade);
        assert_eq!(delivery, Delivery::AgainstPayment);
        assert_eq!(due, None);
        assert_eq!(shape_of(&retry), Delivery::AgainstPayment);
    }

    #[test]
    fn a_payment_for_what_the_payer_receives_moves_neither_account() {
        use crate::stores::Moved;
        // What the units cost is the basis they carry and what the sale earned is the disposal's
        // own gain, so booking the proceeds as well would count one trade twice.
        assert_eq!(moves_equity(Receipt::Sale), None);
        assert_eq!(moves_equity(Receipt::Principal), None);
        assert_eq!(moves_equity(Receipt::Fx), None);
        // And what nobody receives anything for is income to one side and a cost to the other.
        assert_eq!(moves_equity(Receipt::Wage), Some(Moved::Income));
        assert_eq!(moves_equity(Receipt::Tax), Some(Moved::Income));
        assert_eq!(moves_equity(Receipt::Capital), Some(Moved::CapitalPaidIn));
    }

    #[test]
    fn an_expired_delivery_keeps_its_title_leg_unapplied() {
        let legs = [
            delivers(1, 2, 10.0),
            pays(2, 1, 100.0),
            dispatches(1, 2, 10.0),
        ];
        let instruction = Instruction::against_payment(&legs, Cause::Trade);
        let mut queue = Queue::new();
        let queued = queue.joins(&instruction, party(2), Week(10), Week(20));

        queue.gave_up(queued, Week(21));

        assert_eq!(queue.state_of(queued), Waiting::Late);
        assert_eq!(queue.legs_of(queued), legs);
    }
}
