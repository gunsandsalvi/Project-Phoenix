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

use crate::calendar::{Calendar, Day, Period};
use crate::ids::{InstrumentId, PartyId};
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
    ///
    /// **0k.2: there is no `ccy` here, because the instrument already holds one** (Law 4: one
    /// representation per real thing). A leg that carried both was two answers to *what money is
    /// this*, and settlement validated neither — it destructured past the field in the pre-check
    /// and in the application alike. It was not dead: `CrossBorder` read it as `invoiced_in`, so a
    /// region's current and financial accounts were built from the copy nobody checked. The read
    /// that replaces it is `instruments.ccy_of(instrument)` (Law 19).
    Money { from: PartyId, to: PartyId, instrument: InstrumentId, amount: f64, receipt: Receipt },
    /// Units of an instrument moving between two holders, at a price if it is a trade.
    Asset { from: PartyId, to: PartyId, instrument: InstrumentId, qty: f64, price_per_unit: Option<f64> },
    /// Goods B: a physical thing coming into existence. ONE side, because nobody is on the other
    /// end of a harvest — and what keeps it honest is the units identity, checked by a family.
    Create { party: PartyId, instrument: InstrumentId, qty: f64, cost_per_unit: f64 },
    /// **Money A1: an issuer creating its own money.** Not `Create`: a money account is a TOTAL and
    /// carries no lots (Money D2), and money created is the issuer's own LIABILITY rather than an
    /// asset it earned — a bank that booked the deposits it prints as income would be the closest
    /// thing to free money this engine could write.
    /// 0k.2: and no `ccy` here either, for the same reason and with the same read — `money` is an
    /// instrument and an instrument has one currency.
    Mint { issuer: PartyId, money: InstrumentId, amount: f64 },
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
    /// **22d.1: the payer had not got it YET.** A gridlock — A cannot pay B because B has not yet
    /// paid A — is a timing failure and not a default, and resolving it needs no new money. The
    /// instruction is standing in the queue with a day it must settle by; nothing moved.
    Queued,
    /// The payer had not got it and its bank would not lend (Money B3.a).
    ShortOfMoney,
    /// **The payer had it and its BANK could not settle across** (Money E1, Banks Capital C1.a,
    /// 21.2). It was `ShortOfMoney` too, which made two different failures one word: a customer
    /// that could not pay and a bank that could not deliver its customer's money read the same on
    /// the record, and the row that should have been the BANK's was nobody's.
    BankCouldNotSettle,
    /// **Currency B3, Spot FX E1: the payee has no account in the money this was sent in.**
    ///
    /// A payment in one money lands as that money or it does not land. This world used to land it
    /// anyway: `amount` of one currency left and `amount` of another arrived, at a rate of one,
    /// with nobody on the other side — the conversion at the ledger boundary B3 forbids by name,
    /// and the one that makes the currency market invisible because the position never exists.
    ///
    /// It is a FAIL and not a queue: waiting does not give the payee an account. What the payer
    /// does about it is buy the money first (§12 F1, XI-12), which is a trade with a counterparty.
    NoAccountInThatMoney,
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
    /// **G3.a, 22d.1: the one calendar**, because a queued payment is late on a DATE and never
    /// after a count of periods. Settlement had no way to place a date at all, which is one of the
    /// two reasons this world could not hold a payment open.
    pub calendar: &'a Calendar,
    /// What an outcome is SAID under. They were two loose arguments to `settle` and one field
    /// here, which is one fact kept in two shapes (Law 4).
    pub says: Outcomes,
}

/// The journal kinds an instruction's outcome is said under, named once.
#[derive(Clone, Copy, Debug)]
pub struct Outcomes {
    pub settled: u32,
    pub failed: u32,
    /// 22d.1: it is waiting, which is neither of the other two.
    pub queued: u32,
    /// XI-3, 22i.19: and the kind a process ending is said under. It is here with the others
    /// because it is the same sort of fact — what the kernel did, said once (Law 4).
    pub closed: u32,
    /// 21.112: the kind **what a disposal realised** is said under. Settlement is the only place
    /// that holds both halves of the answer at once — the price the leg moved at, and the basis the
    /// lots carried — so it is the only place that can say it without re-deriving one of them
    /// (Law 19). It is a kind rather than a store because a realised gain is an EVENT: it happens at
    /// the moment the units leave, to a named party, for an amount.
    pub realised: u32,
}

impl Outcomes {
    /// The four kinds, declared once on a journal. Every world in this repository says an outcome
    /// under the same four names, and a fixture that named its own would be a second vocabulary.
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

/// **Where a payment lands**, which is one of three answers and used to be one of two.
#[derive(Clone, Copy)]
enum Across {
    /// Nothing crosses. Three cases: the payee banks at the issuer already, the payee IS the
    /// issuer (money coming home extinguishes the deposit), or the payee banks nowhere — which is
    /// what a central bank does, and is why no special case names it.
    Same,
    /// Two banks, one money, and the reserve line they both settle in.
    Banks {
        payers_bank: PartyId,
        payees_bank: PartyId,
        payees_money: InstrumentId,
        reserves: InstrumentId,
    },
    /// **0k.3: it cannot land, and the outcome says whose failure it is.** This arm did not exist,
    /// so both of its cases settled: one of them converted a currency at a rate of one.
    Refused(Outcome, PartyId),
}

/// **Whether this payment crosses two banks**, and what it takes if it does.
///
/// It THROWS on the one thing that is a contract violation rather than an outcome: a bank that
/// issues no money is not a bank, and a payment to its customer cannot be told where to land.
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
    // **Currency B3: A PAYMENT IN ONE MONEY LANDS AS THAT MONEY.** The payee's account is its own
    // bank's, so where that bank issues a different currency there is nowhere for this payment to
    // go — and this function used to send it there anyway: `amount` of one currency left the payer
    // and `amount` of another arrived, at a rate of one, with nobody on the other side (Spot FX
    // E1). `Prints` was never consulted, so the currency market could not have priced it if it had
    // wanted to. Converting on arrival makes that market invisible and unmeasurable, because the
    // position never exists and so can never be seen to be wrong.
    //
    // What the payer does about it is BUY the money first — a trade with a counterparty, in the
    // seller's money (§12 F1, XI-12). `mechanisms::currency`'s `short_of` and `MustBuy` are that
    // decision and nothing calls them yet (0r).
    if instruments.ccy_of(money) != instruments.ccy_of(payees_money) {
        return Across::Refused(Outcome::NoAccountInThatMoney, to);
    }
    // **Money C2.a: the reserve line is the one BOTH banks settle in**, and it was read off the
    // PAYEE's bank alone. So across two banking systems the payer's bank was debited in reserves
    // it need never have held — a claim on a central bank nothing established, going negative in
    // silence where it had none (Appendix B #5).
    //
    // For the ordinary case — two banks at one central bank — this is the same instrument it
    // always was, which is why the world runs unchanged.
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
    ///
    /// **A payment is money going the OTHER WAY, and that is what "versus" means** (XI-5, Money
    /// C3.a). Any money leg at all used to count, so an instruction in which money and units moved
    /// TOGETHER between the same two parties — a cell splitting its book in two (21h), a position
    /// transferred whole — read as delivery-versus-payment, and the only way to get it past the wire
    /// was to declare something false. Money flowing alongside a delivery is part of that delivery;
    /// money flowing back against it is the payment.
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

/// **How an instruction is presented to the wire**, which decides how its legs are checked and
/// whether a short payment may wait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Presented {
    /// The ordinary way: leg by leg, and a short payment joins the queue.
    Fresh,
    /// A queued payment tried again because its payer was paid. Leg by leg, and it is already in
    /// the queue, so it never joins it twice.
    Retry,
    /// **22d.2: a gridlock cycle, settled TOGETHER.** Checked on what the whole of it does to each
    /// holding, because in a cycle nobody has the money on their own — which is what a gridlock IS.
    Together,
}

/// **XI-5: what the legs of one instruction do to each holding, TOGETHER.** Returns the first party
/// this instruction would leave holding less than nothing, and which failure that is.
///
/// Every leg is at FULL VALUE and every leg reaches the wire: nothing here cancels a payment
/// against another (Appendix B: no netting across counterparties). What it does is read the one
/// instant they all happen at, which is what atomic settlement means and is the whole of why a
/// gridlock cycle can settle when none of its members could pay alone.
///
/// **0k.5: it is the check for EVERY instruction now, not only a cycle.** The pre-check used to ask
/// each money leg against the standing balance, which is the same question only while an
/// instruction has one leg out of any one account. A coupon paid to three holders has three, and a
/// payer holding sixty passed a fifty and then passed another fifty — `money_delta` took the
/// account to minus forty and nothing said a word. That is Appendix B #5 exactly: an overdraft
/// nobody lent and nobody refused.
///
/// **The walk is in LEG ORDER and the map is only read**, because a `HashMap`'s own order would put
/// a different party's name on the same failure between two runs of one world.
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
                    // Money D2: and the reserves the banks move between them, which net over a
                    // cycle exactly as the customers' deposits do.
                    moves(payers_bank, reserves, -amount);
                    moves(payees_bank, reserves, amount);
                }
                // 0k.3: a leg that cannot land at all is refused by the pass above this one,
                // before any balance is read — so by here there is nothing left to be short of.
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
            // Money D2: and the payer's BANK needs the reserves to settle it across. A bank that
            // cannot is a bank whose customers' payments do not go through — which is what a
            // liquidity problem IS, and it is refused rather than overdrawn (Appendix B #5; the
            // lender of last resort is a mechanism, not a default). It is its OWN outcome because
            // a customer that could not pay and a bank that could not deliver its customer's money
            // read the same on the record otherwise, and the row that should be the BANK's is
            // nobody's.
            if let Across::Banks { payers_bank, reserves, .. } = across(parties, instruments, to, instrument) {
                if short(payers_bank, reserves) {
                    return Some((Outcome::BankCouldNotSettle, payers_bank));
                }
            }
        }
    }
    None
}

/// 22d.1: who this instruction PAYS. A receipt is what makes a queued payment worth trying again,
/// so the payees of what just settled are the parties whose queues are retried.
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

/// 22d.1: **what a queued payment is doing**, in the words this project already has. `Queued` is
/// waiting; `Taken` settled on a retry; `Late` ran out of days and is the arrear.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Waiting {
    Queued,
    Taken,
    Late,
}

/// **XI-9, Money D5, 22d.1: THE PAYMENT QUEUE — a payment that cannot be made YET is not a payment
/// that failed.**
///
/// A gridlock — A cannot pay B because B has not yet paid A — is a **timing** failure, and
/// resolving it needs no new money at all: it needs the two payments to be tried in the other
/// order, or together. This world had no queue, so every such payment was an arrear the moment it
/// was tried, and a party was recorded as having missed a payment it could have met an instant
/// later. That is a default invented by the order the phases happened to run in.
///
/// **It holds payments and not deliveries.** A queue row is an instruction declared
/// `Delivery::Nothing` whose money leg was short — which is what a payment IS. An instruction that
/// delivers units against payment and cannot pay is a FAIL TO DELIVER, and holding one open would
/// leave the seller's units unencumbered and sellable twice; that is a different mechanism and it
/// is not built (see `docs/IMPLEMENTATION.md` 22d).
///
/// **Every row carries the DAY it is late on** (G3.a): a payment may wait, and how long it may wait
/// is a fact about the payment system, stated once where the wire is built.
pub struct Queue {
    leg_at: Vec<u32>,
    leg_len: Vec<u32>,
    legs: Vec<Leg>,
    cause: Vec<Cause>,
    delivery: Vec<Delivery>,
    /// Who is short. A later receipt to THIS party is what makes the payment worth trying again,
    /// which is the whole mechanism: a queue nobody retries is a list.
    payer: Vec<u32>,
    /// The day it was tried, and the day it stops being early and becomes an arrear.
    queued_on: Vec<i64>,
    late_after: Vec<i64>,
    /// 22d.3: **the day it stopped waiting**, so what became of a payment is a read off the row and
    /// never a tally kept beside it (Law 19). `Missing` while it is still waiting.
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

    /// The legs of one queued payment, as a contiguous slice. Nothing is copied to read them.
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

    /// **§36 C1, 22i.6: A SELLER AGREED TO WAIT.** The payment is not made and is not an arrear —
    /// its day moves out, and it goes on waiting.
    ///
    /// **It is the SAME debt.** The terms are struck beside it as a relation (`agreed::TRADE_CREDIT`)
    /// and the money owed stays here, because the alternative is one debt in two places: an invoice
    /// the buyer owes and a payment the buyer owes, both real, neither aware of the other (Law 4).
    /// What trade credit changes is WHEN, and this is the when.
    pub fn given_time(&mut self, q: QueueId, until: Day) {
        assert!(self.state[q.row()] == Waiting::Queued, "36 C1: only a waiting payment is given time");
        assert!(
            until.0 > self.late_after[q.row()],
            "36 C1: terms that end sooner than the payment's own day are not time given"
        );
        self.late_after[q.row()] = until.0;
    }

    /// It went through on a retry. The row stays readable — how long a payment waited before it was
    /// made is the measurement 22d.3 is about, and a row deleted is a measurement nobody can take.
    pub fn took(&mut self, q: QueueId, on: Day) {
        assert!(self.state[q.row()] == Waiting::Queued, "22d.1: only a waiting payment is taken");
        self.state[q.row()] = Waiting::Taken;
        self.finished_on[q.row()] = Some(on.0);
    }

    /// Its days ran out. This is the arrear, and it is where a queued payment becomes the failure
    /// this world always recorded immediately.
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

    /// Every payment whose day has passed, in the order they joined. The caller records the arrear:
    /// the queue holds payments and the WIRE is what says one failed (Law 4).
    pub fn out_of_days(&self, on: Day) -> Vec<QueueId> {
        (0..self.state.len() as u32)
            .map(QueueId)
            .filter(|q| self.state_of(*q) == Waiting::Queued && self.late_after(*q).0 < on.0)
            .collect()
    }

    /// **22d.2: A CYCLE IN THE QUEUE — A waits on B, B waits on C, C waits on A.**
    ///
    /// Nobody in it can pay alone and all of them can pay together, which is the case the retry
    /// cannot reach: a retry needs money from OUTSIDE the cycle and a pure cycle has no outside.
    /// This is the thing a liquidity-saving mechanism does in a real large-value system, once a day.
    ///
    /// It walks the payer graph — every waiting payment of a party is an edge, and the walk tries
    /// all of them, because a ring may close through a party's second payment and not its first.
    /// When the walk comes back to a party already on the path, the cycle is the path from there.
    /// `skip` holds the rows of cycles that were found and did NOT balance, so the search moves on
    /// rather than offering the same one for ever.
    ///
    /// Only money-only payments are walked: a queued row carrying anything else is not a link in a
    /// chain of payments and settling it inside one would be settling something nobody asked about.
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

    /// Who this queued payment pays. A payment with no payee is not a link in a chain.
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

    /// Every payment ever queued. The history, not the standing queue.
    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// How many are waiting right now, and how many each of the other two states holds. A read over
    /// the rows, never a tally kept beside them (Law 19).
    pub fn census(&self) -> (usize, usize, usize) {
        self.between(Day(i64::MIN), Day(i64::MAX))
    }

    /// **22d.3: WHAT BECAME OF THE PAYMENTS THAT WERE SHORT**, between two days — how many are
    /// still waiting, how many went through after waiting, how many ran out of days. It is a READ
    /// and it causes nothing.
    ///
    /// **The middle number is the measure.** Every one of those was a payment this world would have
    /// recorded as an arrear the instant it was tried, and it was a TIMING failure: the money
    /// existed, it had not arrived yet.
    ///
    /// **The third number is not "insolvency" and must not be read as one.** A payment that ran out
    /// of days was either owed by somebody who could not have paid it at all, or owed in a chain
    /// that never closed; the queue cannot tell those apart, and a number that claimed to would be a
    /// diagnosis nobody measured. What separates them is a solvency test on the payer, which is
    /// `mechanisms/mortality`'s, not this.
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
    /// **22d.1: the payments it could not make yet.** The wire is what happened; this is what is
    /// still trying to. It lives here because settlement is what decides an instruction cannot go
    /// through, and a queue somebody else wrote would be a second writer of that decision (Law 4).
    pub queue: Queue,
    /// **One TECHNOLOGY: how many days a payment may wait before it is late.** A fact about the
    /// payment system, stated where the system is built — not a policy anybody sets and not a
    /// preference anybody has.
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

    /// 22d.1: how long a payment may wait here, as this payment system was built.
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
    ///
    /// **22d.1: and a payment that could not be made YET waits.** A payment whose payer is short
    /// joins the queue instead of becoming an arrear on the spot, and every settlement that pays
    /// somebody money RETRIES what that party was waiting to pay — which is how a gridlock unwinds
    /// without a penny of new money.
    pub fn settle(&mut self, ins: &Instruction<'_>, period: u32, on: &mut Settling<'_>) -> Outcome {
        let out = self.attempt(ins, period, on, Presented::Fresh);
        if out == Outcome::Settled {
            self.release(ins.legs, period, on);
        }
        out
    }

    /// **22d.1: a receipt is a retry.** Whoever was just paid may now be able to pay what it was
    /// waiting on, and whoever THAT pays may be able to pay in turn — so the funded parties are a
    /// worklist and not a single pass. It terminates because every retry that goes through takes a
    /// row out of the queue for good.
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

    /// **XI-9, 22d.2: ONE PASS THAT FINDS THE CYCLES AND SETTLES THEM TOGETHER.**
    ///
    /// The retry unwinds a CHAIN, and it needs money from outside the chain to start it. A pure
    /// cycle has no outside: A waits on B, B waits on C, C waits on A, nobody can pay alone and all
    /// of them can pay together. Without this pass every one of them sits in the queue until its day
    /// runs out and then becomes three arrears — three defaults on a problem that was never about
    /// anybody's solvency.
    ///
    /// **Each leg at full value, and every leg on the wire.** Nothing is cancelled against anything
    /// (Appendix B: no netting across counterparties). What settles it is that the legs happen at
    /// one instant and no holding goes negative at that instant — a DvP cycle, which is what a real
    /// large-value system's liquidity-saving mechanism does.
    ///
    /// Returns how many queued payments went through this way.
    pub fn unwind(&mut self, period: u32, on: &mut Settling<'_>) -> usize {
        let today = on.calendar.start_of(Period(period));
        let mut went = 0usize;
        // The cycles that were found and did not balance — somebody in them is short beyond what
        // the cycle itself funds. They stay queued and nothing is forced (Law 6).
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

    /// **22d.1: the queue's day passed.** A payment that ran out of days is the arrear this world
    /// always recorded on the spot, and it is recorded on the WIRE, because the wire is what says
    /// an instruction failed. Returns how many gave up.
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
        let (parties, instruments, says, calendar) = (*parties, *instruments, *says, *calendar);
        let (settled_kind, failed_kind, realised_kind) = (says.settled, says.failed, says.realised);
        // 21.112: what each disposal in this instruction realised, gathered as the legs apply and
        // said once they all have — because an instruction that fails moved nothing, and a gain
        // announced by a leg that was rolled back would be a gain nobody made.
        let mut realised: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
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

        // **22d.2: a gridlock cycle is checked on what the whole of it does to each holding**, and
        // every other instruction leg by leg. In a cycle nobody has the money on their own — that is
        // what a gridlock IS — and the legs all happen at one instant, so what has to be true is that
        // no holding goes negative AT that instant. Nothing is cancelled and every leg moves at full
        // value; this is the ordering that makes the cycle settle, not netting across counterparties.
        for leg in ins.legs {
            match *leg {
                Leg::Money { to, instrument, .. } => {
                    // **0k.3: where it lands is asked FIRST**, before any balance is read. A leg
                    // that cannot land at all is refused for reasons that have nothing to do with
                    // what anybody holds — and what anybody holds is asked once, below, for the
                    // whole instruction at once.
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
                // **Money A1.d: NO MONEY WITHOUT AN ISSUER**, checked at the one site in this
                // engine that creates money. Nothing minted here and nothing was checked, so the
                // first caller — a bank writing a loan (C4.a), a central bank lending (C4.b) —
                // would have arrived at a door that took whatever it was handed.
                //
                // These THROW where a short payer is refused, because they are not outcomes: a
                // mint naming somebody else's line is a writer that named the wrong party, and
                // there is no state of the world in which a retry makes it true.
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
                // The other side of the same line. `Mint` exists because money created is the
                // issuer's own LIABILITY rather than an asset it earned, and nothing held the two
                // apart: `Create` would have credited a money row with LOTS, which is the same
                // fact kept in two shapes (Law 4, Money D2).
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
                // **Appendix B #9, Register C3: a lien over units nobody holds.** This arm was
                // empty, so a party could pledge what it had not got — and `free` is quantity less
                // the liens, so the row would answer NEGATIVE and refuse every later move of it.
                // Collateral invented, and a holding frozen by a claim that could never be honoured.
                //
                // It is REFUSED rather than thrown, unlike the two above: whether a holder has the
                // free units is the same question an `Asset` leg asks, and a module posting margin
                // it cannot cover is meeting a real refusal rather than making a mistake.
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
        // **XI-5: and what the money legs do to each holding TOGETHER**, which is the only test
        // that is right for an instruction with two legs out of one account. It was asked of a
        // gridlock cycle alone — because in a cycle nobody has the money on their own, and that is
        // what a gridlock IS — and asked leg by leg of everything else, which is the same question
        // only while no payer appears twice. 0k.5 made a payer appear twice, per holder of a line,
        // and sixty covered two fifties in silence.
        //
        // It runs AFTER the loop above, which refuses the legs that cannot land whatever anybody
        // holds. A ring containing one of those is not a party that is short.
        if let Some((outcome, who)) = short_together(ins.legs, reg, parties, instruments) {
            return self.short(outcome, who, ins, period, journal, calendar, says, may_queue);
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
                        Across::Same => {
                            reg.money_delta(to, instrument, amount);
                        }
                        Across::Banks { payers_bank, payees_bank, payees_money, reserves } => {
                            reg.money_delta(to, payees_money, amount);
                            reg.money_delta(payers_bank, reserves, -amount);
                            reg.money_delta(payees_bank, reserves, amount);
                        }
                        // Nothing here can fail, because the pre-check is what made that true —
                        // and 0k.3's refusal is one of the things it made true.
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
                    // Register D2: what the units cost goes with them where it is a transfer, and
                    // the price is the basis where a market struck one (C2.a).
                    match price_per_unit {
                        Some(price) => {
                            // 21.112: **and what the seller REALISED**, which is the one thing only
                            // this line knows: the proceeds against what the lots that left cost.
                            // Nothing else can say it without re-deriving a price a market printed
                            // or a basis the register carried (Law 19), which is why a gain existed
                            // on the register and in no read at all.
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
                }
                // Money A1, D2: the issuer's own money, as a TOTAL, and NO equity — what it created
                // is what it owes, and `Instruments::owed_by` reads that from issued against held.
                Leg::Mint { issuer, money, amount, .. } => {
                    reg.money_delta(issuer, money, amount);
                }
                Leg::Destroy { party, instrument, qty, .. } => {
                    let row = reg.row(party, instrument);
                    let drawn = reg.debit(row, qty);
                    // Goods E4, XI-1, 21.112: **what perished cost something, and the loss is an
                    // EVENT** rather than a number that quietly stops existing. It is a disposal at
                    // no proceeds, so it is the same read as a sale and is said under the same kind.
                    let cost: f64 = drawn.iter().map(|d| d.qty * d.basis_per_unit).sum();
                    if cost != 0.0 {
                        realised.push((party, instrument, -cost));
                    }
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
        // 21.112: and what the disposals realised, now that every leg has applied. It reaches the
        // party that disposed and nobody else (Observer A3): what one holder made on a sale is its
        // own business, and it is what a gains tax and a firm's result are both a read of.
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

    /// **22d.1: short of money is a QUEUE and not an arrear — where the instruction is a payment.**
    ///
    /// A payment (`Delivery::Nothing`) waits: it joins the queue with the day it must settle by,
    /// and a later receipt to its payer is what tries it again. Anything else fails as it always
    /// did — an instruction that delivers units against a payment it cannot make is a fail to
    /// deliver, and holding one open would leave the seller's units unencumbered and sellable a
    /// second time. A retry (`may_queue: false`) never re-queues what is already in the queue.
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
    fn world() -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
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
        (Register::new(), j, p, i, Settlement::new(6), Calendar::new(Day(0), 7, 3), says)
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
        calendar: &'a Calendar,
        says: Outcomes,
    ) -> Settling<'a> {
        Settling { register, journal, parties, instruments, calendar, says }
    }

    #[test]
    fn a_refused_instruction_moves_nothing_at_all() {
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfUnits);
        // NOTHING moved: the money is where it was and so are the shares.
        assert_eq!(reg.quantity(reg.row(a, cash)), 500.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 10.0);
        // And the fail is a RECORDED state, not a silence.
        assert_eq!(j.in_period(1).len(), 1);
    }

    #[test]
    fn both_legs_of_a_trade_move_in_the_same_pass() {
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(a, cash)), 450.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 50.0);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
        assert_eq!(reg.quantity(reg.row(b, share)), 0.0);
        // C2.a: the buyer's lot carries what the market struck, not what the seller paid.
        assert_eq!(reg.lots(reg.row(a, share))[0].basis_per_unit, 5.0);

        // 21.112: **and what the SELLER realised** — 50 of proceeds against 30 the lots cost. It
        // existed on the register and in no read at all, which is why nothing could tax a gain.
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
        // Goods E4, XI-1, 21.112: a loss is an EVENT rather than a number that quietly stops
        // existing. A disposal at no proceeds is the same read as a sale.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let grain = InstrumentId::at(1);
        reg.credit(a, grain, 10.0, 4.0, 1);
        let legs = [Leg::Destroy { party: a, instrument: grain, qty: 10.0, why: Gone::Perished }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Production), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        let gain = j
            .in_period(1)
            .find(|r| j.kind_of(*r) == says.realised && j.subjects_of(*r) == [a.0])
            .expect("what perished cost somebody something");
        assert_eq!(j.says(gain, 0), Some(Value::Num(-40.0)));
    }

    #[test]
    fn encumbered_units_refuse_the_whole_instruction() {
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let lender = PartyId::at(2);
        let share = InstrumentId::at(1);
        reg.credit(a, share, 10.0, 3.0, 1);
        reg.pledge(a, share, lender, 8.0);
        let legs = [Leg::Asset { from: a, to: b, instrument: share, qty: 5.0, price_per_unit: None }];
        let out = s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.quantity(reg.row(a, share)), 10.0);
    }

    #[test]
    fn free_of_payment_delivers_and_records_who_was_trusted() {
        // A restructured bond handed over for the old one: the units move and nothing moves
        // against them. The deliverer performs FIRST and carries the other side's performance.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
            &mut on(&mut reg, &mut j, &ps, &ins, &cal, says),
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
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
        s.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    fn money_moving_with_a_delivery_is_not_a_payment_against_it() {
        // **XI-15, 21h: a cell splitting its book in two moves money AND units, both from the parent
        // to the part that left.** Nothing is versus anything — the money is not payment for the
        // units, it is the same members' money going with them. Any money leg used to make the wire
        // read this as delivery-versus-payment, so the only way past it was to declare something
        // false; "versus" means the money goes the OTHER WAY, and now that is what is asked.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
            &mut on(&mut reg, &mut j, &ps, &ins, &cal, says),
        );
        assert_eq!(out, Outcome::Settled);
        // A third of the people took a third of each, and the basis went with the units (Law 19):
        // nothing was sold, so nothing was realised.
        assert_eq!(reg.quantity(reg.row(part, share)), 10.0);
        assert_eq!(reg.quantity(reg.row(parent, share)), 20.0);
        assert_eq!(reg.lots(reg.row(part, share))[0].basis_per_unit, 2.0);
    }

    #[test]
    #[should_panic(expected = "a payment somebody forgot")]
    fn a_trade_that_lost_its_money_leg_is_caught_instead_of_settling_free() {
        // **The defect this pathway exists to make findable.** Before it, an asset-only instruction
        // was indistinguishable from one whose money leg was dropped: both settled, and Law 5's
        // "a one-sided flow is a defect even when nothing fails" could not be checked.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let share = InstrumentId::at(1);
        reg.credit(b, share, 10.0, 1.0, 1);
        let legs = [Leg::Asset { from: b, to: a, instrument: share, qty: 10.0, price_per_unit: Some(5.0) }];
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    fn a_free_delivery_is_still_all_legs_or_none() {
        // XI-5 holds for a basket handed over free: one line short and NOTHING moves, because a
        // half-delivered restructuring is not a restructuring.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
            &mut on(&mut reg, &mut j, &ps, &ins, &cal, says),
        );
        assert_eq!(out, Outcome::ShortOfUnits);
        assert_eq!(reg.quantity(reg.row(issuer, one)), 100.0, "the first line did not move either");
        assert!(s.delivered_free().is_empty(), "nothing was delivered, so nobody was trusted");
    }

    #[test]
    fn a_payment_moves_money_and_makes_nobody_richer() {

        // Law 5: every flow has two sides, so what one account loses another gains and the world's
        // equity is unchanged. A payment that moved the total would be money appearing from nowhere.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
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
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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
        s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        // It gave up 30 of book and took in 50: it is 20 better off, and nothing was invented.
        assert_eq!(seller_before, 30.0, "ten shares that cost three");
        assert_eq!(worth(&reg, &ins, a), 50.0);
        // The buyer paid 50 and holds 50 of stock: unchanged, which is what a purchase is. Its own
        // share line is NOT a liability to itself — a share is the residual, not a promise (Law 8).
        assert_eq!(worth(&reg, &ins, b), buyer_before);
    }

    #[test]
    fn a_transfer_carries_the_basis_and_a_trade_does_not() {

        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let good = InstrumentId::at(3);
        reg.credit(a, good, 10.0, 7.0, 1);
        let legs = [Leg::Asset { from: a, to: b, instrument: good, qty: 10.0, price_per_unit: None }];
        s.settle(&Instruction::free_of_payment(&legs, Cause::CorporateAction), 2, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        // Register D2: what it cost went with it.
        assert_eq!(reg.lots(reg.row(b, good))[0].basis_per_unit, 7.0);
    }

    /// Two banks, and a payment between their customers. **This is the leg the wire did not have.**
    fn two_banks() -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes, [PartyId; 5], [InstrumentId; 3]) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
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
        (Register::new(), j, p, i, Settlement::new(6), Calendar::new(Day(0), 7, 3), says, [cb, one, two, payer, payee], [reserves, ones, twos])
    }

    #[test]
    fn a_payment_across_two_banks_moves_reserves_between_them() {
        // Money D2, worklist 1: a deposit is a claim on the bank that ISSUED it, so a payee banking
        // elsewhere cannot simply come to hold the payer's bank's money. The payer's bank's deposit is
        // extinguished, the payee's bank's is created, and RESERVES move between the two.
        let (mut reg, mut j, ps, ins, mut s, cal, says, who, lines) = two_banks();
        let (one, two, payer, payee) = (who[1], who[2], who[3], who[4]);
        let (reserves, ones, twos) = (lines[0], lines[1], lines[2]);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);

        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
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
        let (mut reg, mut j, ps, ins, mut s, cal, says, who, lines) = two_banks();
        let (payer, payee) = (who[3], who[4]);
        let ones = lines[1];
        reg.money_delta(payer, ones, 500.0);
        // Its bank has none: the deposit is there and the settlement asset is not.
        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        // **22d.1: it WAITS, and the row it waits on is the BANK's.** A bank short of reserves at
        // the instant a payment is presented is the gridlock this queue exists for: it may have
        // reserves coming in from another bank's customer before the day is out.
        assert_eq!(out, Outcome::Queued);
        let q = crate::ledger::QueueId(0);
        // **21.2, Money E1: and it is the BANK's, not the payer's.** They were one word —
        // `ShortOfMoney` — so a bank that could not deliver its customer's money and a customer
        // that could not pay read identically, and the row that should have been the bank's was
        // nobody's.
        assert_eq!(s.queue.payer_of(q), who[1]);
        assert_ne!(s.queue.payer_of(q), payer, "the payer HAD it; its bank could not settle it out");
        // XI-5: and nothing moved, because nothing settled.
        assert_eq!(reg.quantity(reg.row(payer, ones)), 500.0);

        // **And when its days run out, THIS is the arrear** — recorded on the wire, on the bank,
        // which is the failure this world used to record the instant the payer was short.
        let over = crate::calendar::Day(cal.start_of(crate::calendar::Period(1)).0 + s.waits_for() + 1);
        assert_eq!(s.give_up(over, 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)), 1);
        let failed: Vec<u32> = j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(j.subjects_of(failed[0]), &[who[1].0]);
        assert_eq!(s.queue.state_of(q), crate::ledger::Waiting::Late);
    }

    #[test]
    fn a_gridlock_unwinds_when_the_money_arrives_and_no_new_money_is_made() {
        // **XI-9, 22d.1: A cannot pay B because B has not yet paid A.** Neither is insolvent and
        // neither needs a penny that does not already exist — what they need is for the payments to
        // be tried in the other order. This world had no queue, so both were arrears the instant
        // they were tried, and two parties defaulted on a timing problem.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
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

        // A owes B and cannot pay; B owes C and cannot pay. Both WAIT.
        let a_to_b = [pays(a, b, 100.0)];
        let b_to_c = [pays(b, c, 100.0)];
        assert_eq!(
            s.settle(&Instruction::plain(&a_to_b, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)),
            Outcome::Queued
        );
        assert_eq!(
            s.settle(&Instruction::plain(&b_to_c, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)),
            Outcome::Queued
        );
        assert_eq!(s.queue.census(), (2, 0, 0));

        // **C pays A, and the whole chain goes through behind it.** C's payment funds A; A's
        // queued payment funds B; B's funds C — one receipt releases two payments, and the money
        // that did it is the hundred C already had.
        let c_to_a = [pays(c, a, 100.0)];
        assert_eq!(
            s.settle(&Instruction::plain(&c_to_a, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)),
            Outcome::Settled
        );
        assert_eq!(s.queue.census(), (0, 2, 0), "both waiting payments were taken, and neither was late");

        // Law 5, XI-9: and the money is back where it started. Nothing was created to clear it.
        assert_eq!(reg.quantity(reg.row(c, cash)), 100.0);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 0.0);
        // A-20: and not one of the three is recorded as having failed anything.
        assert_eq!(j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).count(), 0);
    }

    #[test]
    fn a_ring_of_payers_with_nothing_between_them_settles_together_and_none_of_them_defaults() {
        // **XI-9, 22d.2: THE CASE THE RETRY CANNOT REACH.** A owes B, B owes C, C owes A, and not
        // one of them holds a penny. A retry needs money from outside the chain and a ring has no
        // outside — so without this pass all three sit until their day runs out and become three
        // defaults on a problem that was never about anybody's solvency.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let (a, b, c) = (PartyId::at(0), PartyId::at(1), PartyId::at(2));
        let cash = InstrumentId::at(0);
        let pays = |from: PartyId, to: PartyId| {
            [Leg::Money { from, to, instrument: cash, amount: 70.0, receipt: Receipt::Sale }]
        };
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let legs = pays(from, to);
            assert_eq!(
                s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)),
                Outcome::Queued
            );
        }
        assert_eq!(s.queue.census(), (3, 0, 0));

        // The pass finds the ring and settles it as ONE instruction — three legs, each at its full
        // seventy, none of them cancelled against another.
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)), 3);
        assert_eq!(s.queue.census(), (0, 3, 0));
        let n = s.len() - 1;
        assert_eq!(s.outcome_of(n), Outcome::Settled);
        assert_eq!(s.legs_of(n).len(), 3, "every leg is on the wire at full value");

        // Law 5: and everybody is exactly where they started, because that is what a ring of equal
        // debts IS. Nothing was created and nothing was cancelled.
        for who in [a, b, c] {
            assert_eq!(reg.quantity(reg.row(who, cash)), 0.0);
        }
        assert_eq!(j.in_period(1).filter(|r| j.kind_of(*r) == says.failed).count(), 0);
    }

    #[test]
    fn a_ring_that_does_not_balance_is_not_forced_through() {
        // Law 6: a cycle where somebody owes more than the cycle funds does not settle — it is
        // refused and stays queued, and nothing is clamped to make it fit. The pass offers it once
        // and moves on rather than trying the same ring for ever.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let (a, b) = (PartyId::at(0), PartyId::at(1));
        let cash = InstrumentId::at(0);
        let pays = |from: PartyId, to: PartyId, amount: f64| {
            [Leg::Money { from, to, instrument: cash, amount, receipt: Receipt::Sale }]
        };
        let out = pays(a, b, 100.0);
        let back = pays(b, a, 60.0);
        for legs in [&out, &back] {
            s.settle(&Instruction::plain(legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        }
        // A pays 100 and receives 60: it is forty short whichever order they go in.
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)), 0);
        assert_eq!(s.queue.census(), (2, 0, 0));

        // Give A the forty and it goes through, the ring and all.
        reg.money_delta(a, cash, 40.0);
        assert_eq!(s.unwind(1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says)), 2);
        assert_eq!(reg.quantity(reg.row(b, cash)), 40.0);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
    }

    #[test]
    fn a_delivery_it_cannot_pay_for_still_fails_because_holding_it_open_would_sell_the_units_twice() {
        // 22d.1: the queue holds PAYMENTS. An instruction that delivers units against a payment it
        // cannot make is a fail to deliver, and holding one open would leave the seller's units
        // unencumbered and sellable a second time — a different mechanism, and not this one.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let (buyer, seller) = (PartyId::at(0), PartyId::at(1));
        let (cash, share) = (InstrumentId::at(0), InstrumentId::at(1));
        reg.credit(seller, share, 10.0, 3.0, 1);
        let legs = [
            Leg::Money { from: buyer, to: seller, instrument: cash, amount: 50.0, receipt: Receipt::Sale },
            Leg::Asset { from: seller, to: buyer, instrument: share, qty: 10.0, price_per_unit: Some(5.0) },
        ];
        let out = s.settle(&Instruction::against_payment(&legs, Cause::Trade), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfMoney);
        assert!(s.queue.is_empty());
        assert_eq!(reg.quantity(reg.row(seller, share)), 10.0, "XI-5: nothing moved");
    }

    #[test]
    fn a_payment_within_one_bank_never_touches_reserves() {
        // The ordinary case, and it has to stay ordinary: two customers of one bank settle on that
        // bank's books and the central bank never hears about it.
        let (mut reg, mut j, mut ps, ins, mut s, cal, says, who, lines) = two_banks();
        let one = who[1];
        let (reserves, ones) = (lines[0], lines[1]);
        let payer = who[3];
        let alongside = ps.add(0, RegionId::at(0), one, Representation::Named, 1, 0);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);
        let legs = [Leg::Money { from: payer, to: alongside, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(alongside, ones)), 300.0);
        assert_eq!(reg.quantity(reg.row(one, reserves)), 800.0, "nothing left the bank");
    }

    #[test]
    fn two_legs_out_of_one_account_are_weighed_together_and_never_overdraw_it() {
        // **Appendix B #5, Money B3.c: an overdraft is never a silent negative.** The pre-check
        // asked each money leg against the STANDING balance, which is the same question only while
        // no payer appears twice in an instruction. 0k.5 made one appear per holder of a line: a
        // payer holding sixty passed a fifty, then passed another fifty, and `money_delta` took
        // the account to MINUS FORTY with nothing said. Found by a coupon to two holders.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let (a, b, c) = (PartyId::at(0), PartyId::at(1), PartyId::at(2));
        let cash = InstrumentId::at(0);
        reg.money_delta(a, cash, 60.0);
        let legs = [
            Leg::Money { from: a, to: b, instrument: cash, amount: 50.0, receipt: Receipt::Interest },
            Leg::Money { from: a, to: c, instrument: cash, amount: 50.0, receipt: Receipt::Interest },
        ];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        // A payment may WAIT (22d.1) — what it may not do is half-happen or go negative.
        assert_eq!(out, Outcome::Queued);
        assert_eq!(reg.quantity(reg.row(a, cash)), 60.0, "XI-5: nothing moved");
        assert_eq!(reg.quantity(reg.row(b, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(c, cash)), 0.0);

        // And with enough for both, both go — at full value, nothing netted.
        reg.money_delta(a, cash, 40.0);
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(a, cash)), 0.0);
        assert_eq!(reg.quantity(reg.row(b, cash)), 50.0);
        assert_eq!(reg.quantity(reg.row(c, cash)), 50.0);
    }

    /// **0k.3: two countries.** Two central banks, one commercial bank in each, one customer each,
    /// and the two currencies are different — which is the case this world has never once built and
    /// the reason the conversion at par went unseen for the whole of the port. Returns
    /// `[cb_here, cb_there, bank_here, bank_there, payer, payee]` and
    /// `[reserves_here, reserves_there, money_here, money_there]`.
    #[allow(clippy::type_complexity)]
    fn two_countries(
    ) -> (Register, Journal, Parties, Instruments, Settlement, Calendar, Outcomes, [PartyId; 6], [InstrumentId; 4]) {
        let mut j = Journal::new();
        let says = Outcomes::declared(&mut j);
        let mut p = Parties::new();
        let (here, there) = (RegionId::at(0), RegionId::at(1));
        let cb_here = p.add(0, here, PartyId::NONE, Representation::Named, 1, 0);
        let cb_there = p.add(0, there, PartyId::NONE, Representation::Named, 1, 0);
        let bank_here = p.add(0, here, cb_here, Representation::Named, 1, 0);
        let bank_there = p.add(0, there, cb_there, Representation::Named, 1, 0);
        let payer = p.add(0, here, bank_here, Representation::Named, 1, 0);
        let payee = p.add(0, there, bank_there, Representation::Named, 1, 0);
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
        // **Currency B3, Spot FX E1.** `amount` of one currency left and `amount` of another
        // arrived, at a rate of one, with nobody on the other side. `across` compared banks and
        // never compared currencies, so the conversion happened at the ledger boundary — which is
        // the one place B3 forbids by name, because the position never exists and so can never be
        // seen to be wrong.
        let (mut reg, mut j, ps, ins, mut s, cal, says, who, lines) = two_countries();
        let (bank_here, payer, payee) = (who[2], who[4], who[5]);
        let (reserves_here, money_here, money_there) = (lines[0], lines[2], lines[3]);
        reg.money_delta(payer, money_here, 500.0);
        reg.money_delta(bank_here, reserves_here, 900.0);

        let legs = [Leg::Money { from: payer, to: payee, instrument: money_here, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::NoAccountInThatMoney);

        // XI-5: and nothing moved at all — not the payer's money, not the payee's, not a reserve.
        assert_eq!(reg.quantity(reg.row(payer, money_here)), 500.0);
        assert_eq!(reg.quantity(reg.row(payee, money_there)), 0.0);
        assert_eq!(reg.quantity(reg.row(payee, money_here)), 0.0);
        assert_eq!(reg.quantity(reg.row(bank_here, reserves_here)), 900.0);
        // It is a FAIL and not a queue: waiting does not give the payee an account.
        assert!(s.queue.is_empty(), "§12 F1: what the payer does is buy the money, not wait");
    }

    #[test]
    fn two_banks_with_no_reserve_line_in_common_cannot_settle_between_them() {
        // The reserve leg had the same hole: `reserves` was read off the PAYEE's bank alone, so
        // the payer's bank was debited in a line it need never have held — a claim on a central
        // bank nothing established, going negative in silence (Appendix B #5).
        //
        // Here the two banks are in one currency and at two different central banks, so the
        // currency check above passes and only the reserve line refuses.
        let (mut reg, mut j, mut ps, mut ins, mut s, cal, says, who, lines) = two_countries();
        let (cb_there, bank_here) = (who[1], who[2]);
        let (reserves_here, money_here) = (lines[0], lines[2]);
        let a = CurrencyCode::at(0);
        // A bank abroad that issues the SAME money as here, at its own central bank.
        let bank_abroad = ps.add(0, RegionId::at(1), cb_there, Representation::Named, 1, 0);
        let abroad = ps.add(0, RegionId::at(1), bank_abroad, Representation::Named, 1, 0);
        let theirs = ins.issue(bank_abroad, a, Class::Money, UnitId::at(0), None, None);
        let payer = who[4];
        reg.money_delta(payer, money_here, 500.0);
        reg.money_delta(bank_here, reserves_here, 900.0);

        let legs = [Leg::Money { from: payer, to: abroad, instrument: money_here, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::BankCouldNotSettle);
        assert_eq!(reg.quantity(reg.row(payer, money_here)), 500.0, "XI-5: nothing moved");
        assert_eq!(reg.quantity(reg.row(abroad, theirs)), 0.0);
        assert_eq!(reg.quantity(reg.row(bank_here, reserves_here)), 900.0);
        // And it does not WAIT. Before this change the payer's bank was asked for reserves at the
        // other central bank, held none of them, and the payment joined the queue as though the
        // money were on its way — a gridlock invented out of two banking systems that are not
        // connected at all.
        assert!(s.queue.is_empty(), "22d.1: a missing reserve line is not a timing failure");
    }

    #[test]
    fn a_payment_home_to_its_own_currency_still_settles_across_two_banks() {
        // The change must not refuse what it never should have: two banks at ONE central bank, in
        // one money, is the ordinary interbank leg and the reserve line is the same instrument it
        // always was. This is why the assembled world runs unchanged.
        let (mut reg, mut j, ps, ins, mut s, cal, says, who, lines) = two_banks();
        let (one, payer, payee) = (who[1], who[3], who[4]);
        let (reserves, ones, twos) = (lines[0], lines[1], lines[2]);
        reg.money_delta(payer, ones, 500.0);
        reg.money_delta(one, reserves, 800.0);
        let legs = [Leg::Money { from: payer, to: payee, instrument: ones, amount: 300.0, receipt: Receipt::Sale }];
        let out = s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(payee, twos)), 300.0, "it landed in its own bank's money");
        assert_eq!(reg.quantity(reg.row(one, reserves)), 500.0, "and the reserves moved");
    }

    // 0k.1: the three leg kinds the pre-check skipped. In `world()` the bank is `party(9)` and it
    // issues instrument 0, so anybody else minting it is minting a line it does not owe.

    #[test]
    fn an_issuer_mints_its_own_money_and_owes_it_to_whoever_ends_up_holding_it() {
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let bank = PartyId::at(9);
        let (holder, cash) = (PartyId::at(0), InstrumentId::at(0));
        let made = [Leg::Mint { issuer: bank, money: cash, amount: 700.0 }];
        let out = s.settle(&Instruction::plain(&made, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.quantity(reg.row(bank, cash)), 700.0);
        // Money A1: and what it OWES is what others hold of what it issued — read from the register,
        // never a tally beside it (Law 19). Nothing is owed while the money is still in its own
        // hands; paying it away is what makes it somebody's claim.
        let owed = |reg: &Register| {
            crate::instruments::owed_by(bank, &ins, |i| {
                let (held, _) = reg.held_total(i);
                held - reg.quantity(reg.row(bank, i))
            })
        };
        assert_eq!(owed(&reg), 0.0, "unissued: it holds its own liability");
        let paid = [Leg::Money { from: bank, to: holder, instrument: cash, amount: 300.0, receipt: Receipt::Transfer }];
        let out = s.settle(&Instruction::plain(&paid, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(owed(&reg), 300.0, "Money A1: every unit is owed by a named issuer");
    }

    #[test]
    #[should_panic(expected = "money created from nothing")]
    fn a_party_cannot_mint_money_it_does_not_owe() {
        // Appendix B #1, the single most consequential FORBID in the document: a balance that is
        // nobody's liability. Nothing checked it, so any module could have put the bank's money on
        // any book it liked.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let legs = [Leg::Mint { issuer: PartyId::at(0), money: InstrumentId::at(0), amount: 700.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "creates nothing")]
    fn a_negative_mint_is_not_a_mint() {
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let legs = [Leg::Mint { issuer: PartyId::at(9), money: InstrumentId::at(0), amount: -700.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "the lots its basis lives in")]
    fn minting_a_line_that_carries_lots_would_throw_its_basis_away() {
        // `money_delta` sets the row to a TOTAL. A mint of a good would settle, and the lots that
        // hold what those units cost would stop existing (Money D2, XI-5's third rider).
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let good = InstrumentId::at(1);
        let legs = [Leg::Mint { issuer: PartyId::at(1), money: good, amount: 5.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    #[should_panic(expected = "`Leg::Mint` exists to draw")]
    fn money_is_never_created_on_a_holders_book() {
        // The other side of the same line: `Create` writes LOTS, and a money account has none.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let legs = [Leg::Create { party: PartyId::at(0), instrument: InstrumentId::at(0), qty: 700.0, cost_per_unit: 1.0 }];
        s.settle(&Instruction::plain(&legs, Cause::Payment), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
    }

    #[test]
    fn nothing_is_pledged_that_is_not_held_and_nothing_is_pledged_twice() {
        // Appendix B #9. The arm was empty, so a pledge of units nobody held made `free` NEGATIVE
        // and froze the row against every later move of it — collateral invented, and a holding
        // locked by a claim that could never be honoured.
        let (mut reg, mut j, ps, ins, mut s, cal, says) = world();
        let (holder, lender) = (PartyId::at(0), PartyId::at(1));
        let share = InstrumentId::at(1);
        let over = [Leg::Pledge { holder, instrument: share, to: lender, qty: 10.0 }];
        let out = s.settle(&Instruction::plain(&over, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::ShortOfUnits, "it holds none of it");

        reg.credit(holder, share, 10.0, 3.0, 1);
        let ok = [Leg::Pledge { holder, instrument: share, to: lender, qty: 6.0 }];
        let out = s.settle(&Instruction::plain(&ok, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Settled);
        assert_eq!(reg.free(reg.row(holder, share)), 4.0);

        // And the four that are left cannot be pledged twice: the units are there and somebody
        // else has a claim over them, which is what `Encumbered` says.
        let again = [Leg::Pledge { holder, instrument: share, to: PartyId::at(2), qty: 6.0 }];
        let out = s.settle(&Instruction::plain(&again, Cause::Settlement), 1, &mut on(&mut reg, &mut j, &ps, &ins, &cal, says));
        assert_eq!(out, Outcome::Encumbered);
        assert_eq!(reg.free(reg.row(holder, share)), 4.0, "XI-5: the refusal moved nothing");
    }
}
