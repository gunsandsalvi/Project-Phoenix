//! THE STORES THE MODULES NEEDED AND THE KERNEL DID NOT HAVE.
//!
//! @spec ARCHITECTURE 4.9b · XI-10 · XI-15 · §46 · Law 4, Law 8, Law 10, Law 19 · Appendix B

use crate::calendar::Day;
use crate::ids::{InstrumentId, PartyId};
use std::collections::{BTreeMap, HashMap};

// THE KIND COLUMNS, BESIDE THE STORES THEY NAME.

/// The agreement kinds this world has.
pub mod agreed {
    /// An employer and a worker.
    pub const ENGAGEMENT: u32 = 0;
    pub const MORTGAGE: u32 = 1;
    pub const POLICY: u32 = 2;
    pub const SUPPLY: u32 = 3;
    pub const TENANCY: u32 = 4;
    /// What a pool is run under.
    pub const MANDATE: u32 = 5;
    pub const SUBSCRIPTION: u32 = 6;
    pub const PRIME_BROKERAGE: u32 = 7;
    pub const SECURITIES_LOAN: u32 = 8;
    pub const DERIVATIVE: u32 = 9;
    pub const CARRIAGE: u32 = 10;
    pub const TRADE_CREDIT: u32 = 11;
    /// A named lender's committed line to a named borrower — the backstop an issuer keeps behind its
    /// paper and the facility a borrower draws on are ONE object under two names.
    pub const COMMITMENT: u32 = 12;
}

/// What a party STANDS BEHIND, one-sided, until it withdraws it.
pub mod standing {
    /// XI-10, §39 B: an open position an employer holds.
    pub const POSTING: u32 = 0;
    /// What a lender is currently lending at.
    pub const LENDING_STANDARD: u32 = 1;
    /// THE GRADE AN ASSESSOR HOLDS ON A NAME.
    pub const GRADE: u32 = 2;
    /// The rate a bank pays on deposits.
    pub const DEPOSIT_RATE: u32 = 3;
    /// A lender's OWN view of a borrower.
    pub const OWN_VIEW: u32 = 4;
    /// WHAT A BANK EXPECTS A COMPANY TO REPORT.
    pub const ESTIMATE: u32 = 5;
}

/// THE SCALE `standing::GRADE`'s FIRST TERM IS ON, named as a market names it.
///
/// AAA down to D, with the notches, because a notch is what a downgrade moves by and the boundary
/// between BBB- and BB+ is what a mandate is written against — a fall past it is a forced sale by
/// every holder bound by it, at the same time. A seven-label scale cannot express that boundary,
/// and a grade is DATA: the judgement that puts a name on it is the assessor's.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Grade {
    AAA,
    AAplus,
    AA,
    AAminus,
    Aplus,
    A,
    Aminus,
    BBBplus,
    BBB,
    BBBminus,
    BBplus,
    BB,
    BBminus,
    Bplus,
    B,
    Bminus,
    CCCplus,
    CCC,
    CCCminus,
    CC,
    C,
    D,
}

impl Grade {
    /// The best grade there is, and the worst. A scale with no ends is not a scale.
    pub const BEST: Grade = Grade::AAA;
    pub const WORST: Grade = Grade::D;

    /// THE INVESTMENT-GRADE BOUNDARY: the lowest grade that is still investment grade. It is a fact
    /// about the market's own scale, not a number anybody here chose.
    pub const LOWEST_INVESTMENT_GRADE: Grade = Grade::BBBminus;

    /// Whether this grade is one a mandate written for investment grade may hold.
    pub fn investment_grade(self) -> bool {
        self <= Grade::LOWEST_INVESTMENT_GRADE
    }

    /// The scale itself, best first. The POSITION is the rank and the string is how a market writes
    /// it, so there is one table rather than three that can disagree.
    const SCALE: &'static [(Grade, &'static str)] = &[
        (Grade::AAA, "AAA"),
        (Grade::AAplus, "AA+"),
        (Grade::AA, "AA"),
        (Grade::AAminus, "AA-"),
        (Grade::Aplus, "A+"),
        (Grade::A, "A"),
        (Grade::Aminus, "A-"),
        (Grade::BBBplus, "BBB+"),
        (Grade::BBB, "BBB"),
        (Grade::BBBminus, "BBB-"),
        (Grade::BBplus, "BB+"),
        (Grade::BB, "BB"),
        (Grade::BBminus, "BB-"),
        (Grade::Bplus, "B+"),
        (Grade::B, "B"),
        (Grade::Bminus, "B-"),
        (Grade::CCCplus, "CCC+"),
        (Grade::CCC, "CCC"),
        (Grade::CCCminus, "CCC-"),
        (Grade::CC, "CC"),
        (Grade::C, "C"),
        (Grade::D, "D"),
    ];

    /// How many notches there are, which is what a move is measured in.
    pub const NOTCHES: usize = Grade::SCALE.len();

    /// The grade as a term, so a house can STAND behind it (`standing::GRADE`).
    pub fn rank(self) -> f64 {
        self as u8 as f64
    }

    /// And back, reading a term a house is standing behind.
    pub fn at_rank(rank: f64) -> Option<Grade> {
        match rank as i64 {
            at if at >= 0 => Grade::SCALE.get(at as usize).map(|(g, _)| *g),
            _ => None,
        }
    }

    /// The rung a rank falls on. There are 22 rungs and no more, so a credit better than the best
    /// grade is still the best grade — a fact about the scale, not a limit on the credit.
    pub fn nearest(rank: f64) -> Grade {
        assert!(!rank.is_nan(), "44 A2: a grade read off a rank that is not a number");
        match Grade::at_rank(rank) {
            Some(g) => g,
            None => match rank < 0.0 {
                true => Grade::BEST,
                false => Grade::WORST,
            },
        }
    }

    /// As a market writes it. An internal id is never a display name.
    pub fn shown(self) -> &'static str {
        Grade::SCALE[self as usize].1
    }
}


/// WHAT A LENDER IS CURRENTLY LENDING AT — the shape of `standing::LENDING_STANDARD`'s terms.
///
/// The bank decides it from its own book and writes it; housing reads it to see what a buyer can
/// bid. It sits beside the column so neither has to import the other to mean the same thing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Standard {
    pub income_multiple: f64,
    /// The share of the price the buyer must find itself.
    pub deposit_share: f64,
}

/// The processes this world runs.
pub mod afoot {
    pub const CAPITAL_PROGRAMME: u32 = 0;
    pub const FORECLOSURE: u32 = 1;
    pub const BUY_BACK: u32 = 2;
    pub const ELECTION: u32 = 3;
    pub const WORKOUT: u32 = 4;
    pub const FLOTATION: u32 = 5;
    pub const TAKEOVER: u32 = 6;
    pub const SECURITISATION: u32 = 7;
}

/// What a party's outlook is ABOUT.
pub mod about {
    pub const WHAT_IT_SELLS_FOR: u32 = 0;
    pub const WHAT_IT_KEEPS_EARNING: u32 = 1;
    pub const WHAT_CREDIT_COSTS: u32 = 2;
    pub const WHAT_A_HOUSE_IS_WORTH: u32 = 3;
    pub const WHETHER_IT_IS_PAID_BACK: u32 = 4;
    /// How much it expects to sell — a quantity, and a different fact from the price it
    /// expects to get.
    pub const HOW_MUCH_IT_SELLS: u32 = 5;
}

/// Where an agreement's terms live.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AgreementId(pub u32);

impl AgreementId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// An agreement is a relation, recorded — two named parties, terms, a start and an end.
#[derive(Default)]
pub struct Agreements {
    kind: Vec<u32>,
    /// Two sides, always.
    one: Vec<u32>,
    other: Vec<u32>,
    term_at: Vec<u32>,
    term_len: Vec<u32>,
    terms: Vec<f64>,
    from: Vec<i64>,
    /// `Missing` where it runs until somebody ends it — which is not the same as ending today.
    until: Vec<Option<i64>>,
    live: Vec<bool>,
    by_party: HashMap<u32, Vec<u32>>,
    by_kind: HashMap<u32, Vec<u32>>,
}

impl Agreements {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    /// The only way to make one.
    pub fn strike(
        &mut self,
        kind: u32,
        one: PartyId,
        other: PartyId,
        terms: &[f64],
        from: Day,
        until: Option<Day>,
    ) -> AgreementId {
        assert!(one.some() && other.some(), "Law 5: an agreement needs two named parties");
        assert!(one != other, "Law 5: a party does not agree with itself");
        if let Some(end) = until {
            assert!(end.0 >= from.0, "17f: an agreement that ends before it begins is not one");
        }
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.one.push(one.0);
        self.other.push(other.0);
        self.term_at.push(self.terms.len() as u32);
        self.term_len.push(terms.len() as u32);
        self.terms.extend_from_slice(terms);
        self.from.push(from.0);
        self.until.push(until.map(|d| d.0));
        self.live.push(true);
        self.by_party.entry(one.0).or_default().push(row);
        self.by_party.entry(other.0).or_default().push(row);
        self.by_kind.entry(kind).or_default().push(row);
        AgreementId(row)
    }

    #[inline]
    pub fn kind_of(&self, a: AgreementId) -> u32 {
        self.kind[a.row()]
    }

    #[inline]
    pub fn between(&self, a: AgreementId) -> (PartyId, PartyId) {
        (PartyId(self.one[a.row()]), PartyId(self.other[a.row()]))
    }

    /// The terms as the kind declared them.
    pub fn terms(&self, a: AgreementId) -> &[f64] {
        let at = self.term_at[a.row()] as usize;
        let len = self.term_len[a.row()] as usize;
        &self.terms[at..at + len]
    }

    #[inline]
    pub fn from(&self, a: AgreementId) -> Day {
        Day(self.from[a.row()])
    }

    #[inline]
    pub fn until(&self, a: AgreementId) -> Option<Day> {
        self.until[a.row()].map(Day)
    }

    #[inline]
    pub fn live(&self, a: AgreementId) -> bool {
        self.live[a.row()]
    }

    /// How many relations are live, counted over the rows rather than kept beside them.
    pub fn live_now(&self) -> usize {
        self.live.iter().filter(|l| **l).count()
    }

    /// It ends, and the ending is recorded.
    pub fn end(&mut self, a: AgreementId) {
        self.live[a.row()] = false;
    }

    /// The same relationship, now naming the cell that actually holds those people.
    pub fn moves(&mut self, a: AgreementId, from: PartyId, to: PartyId) {
        assert!(self.live[a.row()], "17f: an agreement that has ended moves nowhere");
        if self.one[a.row()] == from.0 {
            self.one[a.row()] = to.0;
        } else if self.other[a.row()] == from.0 {
            self.other[a.row()] = to.0;
        } else {
            panic!("Law 5: {from:?} is not a party to this agreement");
        }
        self.by_party.entry(to.0).or_default().push(a.0);
        if let Some(rows) = self.by_party.get_mut(&from.0) {
            rows.retain(|r| *r != a.0);
        }
    }

    /// Both directions.
    pub fn of_party(&self, p: PartyId) -> &[u32] {
        match self.by_party.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// A mechanism asks for ITS OWN kind's rows and never branches on somebody else's.
    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// A committed line, and there is one of them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Commitment {
    pub lender: PartyId,
    pub borrower: PartyId,
    pub limit: f64,
    pub drawn: f64,
    /// The margin it was STRUCK at — never the one the lender would quote today.
    pub margin: f64,
    /// Paid on the UNDRAWN headroom, every period, whether or not it is used.
    pub fee_on_undrawn: f64,
    /// `Missing` where it stands until somebody ends it, which is not the same as ending today.
    pub until: Option<Day>,
}

impl Commitment {
    /// The headroom: what the borrower may still draw, and what the fee is paid on.
    pub fn undrawn(&self) -> f64 {
        self.limit - self.drawn
    }

    /// What it costs this period, to the lender who sold the option.
    pub fn costs(&self) -> (PartyId, f64) {
        assert!(
            self.fee_on_undrawn > 0.0,
            "9 B4: a committed line with no commitment fee is a free option the lender did not sell"
        );
        (self.lender, self.undrawn() * self.fee_on_undrawn)
    }

    /// Whether this line is still available on a given day.
    pub fn live_on(&self, day: Day) -> bool {
        match self.until {
            None => true,
            Some(end) => day <= end,
        }
    }
}

/// What a payment on a schedule IS to the party that owes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owing {
    Interest,
    Principal,
    Premium,
    Rent,
    /// 29 A2: capital committed and not paid, called on a date the investor cannot refuse.
    Call,
}

/// WHAT A PAYMENT IS ON. An instrument's payment goes to whoever the register says holds it, THEN
/// (Register A2.a); a bilateral one goes to the party it was struck with. They are two different
/// facts rather than one with a special case, and a payment that cannot say which it is has no
/// payee.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owed {
    On(InstrumentId),
    To(PartyId),
}

/// ONE PAYMENT A CLAIM OWES: the interval it covers, the day it falls, how much and of what. The
/// interval is part of it rather than beside it, because a coupon IS a period and a payment that
/// cannot say which one cannot be accrued (Bond N6).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Payment {
    pub from: Day,
    pub due: Day,
    pub amount: f64,
    pub of: Owing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DueState {
    Open,
    Queued { until: Day },
    Settled { on: Day },
    Failed { on: Day, outcome: crate::ledger::Outcome },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DueId(pub u32);

impl DueId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// 5 D2, XI-9: what an instrument owes and when.
#[derive(Default)]
pub struct Schedules {
    /// What each payment is ON — a line, or a named party it was struck with.
    on: Vec<Owed>,
    owed_by: Vec<u32>,
    /// Bond N6: the day this payment STARTED covering, so a coupon says which period it is for and
    /// what has accrued on it is a read rather than a stored balance. A principal covers no days
    /// and carries its own due date here.
    from: Vec<i64>,
    due: Vec<i64>,
    amount: Vec<f64>,
    /// Bond N3: the money the amount is in. Inferring it from where the payer banks is inferring it
    /// wrong exactly when it matters (Currency A4).
    ccy: Vec<u32>,
    of: Vec<Owing>,
    paid: Vec<bool>,
    state: Vec<DueState>,
    by_instrument: HashMap<u32, Vec<u32>>,
    /// By DAY, so "what falls due this period" is a read and not a walk of everything.
    by_day: BTreeMap<i64, Vec<u32>>,
    /// By PAYER, so a party can be asked what IT must find — which is the question a participant
    /// deciding about money has, and the one this store could not answer.
    by_payer: HashMap<u32, Vec<u32>>,
}

impl Schedules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.on.len()
    }

    pub fn is_empty(&self) -> bool {
        self.on.is_empty()
    }

    /// One payment, over one interval, in one money, owed by one party.
    pub fn owes(
        &mut self,
        on: Owed,
        owed_by: PartyId,
        ccy: crate::ids::CurrencyCode,
        p: Payment,
    ) -> DueId {
        assert!(owed_by.some(), "Appendix B: no liability without somebody who owes it");
        assert!(p.amount > 0.0, "5 D2: a payment of nothing is not a payment that falls due");
        assert!(p.from <= p.due, "Money G3.a: a payment cannot cover days after it falls due");
        if let Owed::To(payee) = on {
            assert!(payee.some(), "Appendix B: no liability without beneficiaries");
            assert!(payee != owed_by, "5 D2: a party owing itself is not an obligation");
        }
        let row = self.on.len() as u32;
        self.on.push(on);
        self.owed_by.push(owed_by.0);
        self.from.push(p.from.0);
        self.due.push(p.due.0);
        self.amount.push(p.amount);
        self.ccy.push(ccy.0);
        self.of.push(p.of);
        self.paid.push(false);
        self.state.push(DueState::Open);
        if let Owed::On(line) = on {
            self.by_instrument.entry(line.0).or_default().push(row);
        }
        self.by_day.entry(p.due.0).or_default().push(row);
        self.by_payer.entry(owed_by.0).or_default().push(row);
        DueId(row)
    }

    /// What this payment is on, which is what says who is paid.
    #[inline]
    pub fn on(&self, d: DueId) -> Owed {
        self.on[d.row()]
    }

    #[inline]
    pub fn owed_by(&self, d: DueId) -> PartyId {
        PartyId(self.owed_by[d.row()])
    }

    #[inline]
    pub fn due(&self, d: DueId) -> Day {
        Day(self.due[d.row()])
    }

    /// The day this payment started covering.
    #[inline]
    pub fn from(&self, d: DueId) -> Day {
        Day(self.from[d.row()])
    }

    #[inline]
    pub fn ccy(&self, d: DueId) -> crate::ids::CurrencyCode {
        crate::ids::CurrencyCode(self.ccy[d.row()])
    }

    /// BOND N9.b: WHAT HAS ACCRUED ON THIS PAYMENT BY A DAY — a read over the row's own interval,
    /// never a balance kept beside it. A principal accrues nothing, which is why it answers None
    /// rather than zero.
    pub fn accrued(&self, d: DueId, on: Day) -> Option<f64> {
        match self.of(d) {
            Owing::Interest => {
                Some(crate::instruments::accrued(self.from(d), self.due(d), self.amount(d), on))
            }
            Owing::Principal | Owing::Premium | Owing::Rent | Owing::Call => None,
        }
    }

    /// The unpaid payment one instrument is ACCRUING on a day: the one whose interval the day falls
    /// inside. There is at most one, because a schedule's intervals do not overlap.
    pub fn accruing(&self, i: InstrumentId, on: Day) -> Option<DueId> {
        self.of_instrument(i)
            .iter()
            .map(|r| DueId(*r))
            .find(|d| {
                !self.paid(*d)
                    && self.of(*d) == Owing::Interest
                    && self.from(*d) <= on
                    && on < self.due(*d)
            })
    }

    #[inline]
    pub fn amount(&self, d: DueId) -> f64 {
        self.amount[d.row()]
    }

    #[inline]
    pub fn of(&self, d: DueId) -> Owing {
        self.of[d.row()]
    }

    #[inline]
    pub fn paid(&self, d: DueId) -> bool {
        self.paid[d.row()]
    }

    /// A-20: only the wire's outcome changes contractual performance state.
    pub fn apply(&mut self, update: crate::ledger::DueUpdate) {
        match update.outcome {
            crate::ledger::DueOutcome::Settled { on } => {
                self.paid[update.due.row()] = true;
                self.state[update.due.row()] = DueState::Settled { on };
            }
            crate::ledger::DueOutcome::Queued { until } => {
                self.state[update.due.row()] = DueState::Queued { until };
            }
            crate::ledger::DueOutcome::Failed { on, outcome } => {
                self.state[update.due.row()] = DueState::Failed { on, outcome };
            }
        }
    }

    pub fn state(&self, d: DueId) -> DueState {
        self.state[d.row()]
    }

    /// What falls due between two days, which is what a period asks.
    pub fn falling(&self, from: Day, to: Day) -> Vec<DueId> {
        self.by_day
            .range(from.0..=to.0)
            .flat_map(|(_, rows)| rows.iter().map(|r| DueId(*r)))
            .filter(|d| !self.paid(*d))
            .collect()
    }

    /// What THIS PARTY must find, and by when.
    pub fn of_payer(&self, p: PartyId) -> &[u32] {
        match self.by_payer.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// Everything one instrument owes, in the order it was written.
    pub fn of_instrument(&self, i: InstrumentId) -> &[u32] {
        match self.by_instrument.get(&i.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// WHAT THIS PARTY MUST FIND BETWEEN TWO DATES: what falls due in the window and is not paid.
    pub fn falling_for(&self, p: PartyId, from: Day, to: Day) -> f64 {
        self.of_payer(p)
            .iter()
            .map(|r| DueId(*r))
            .filter(|d| !self.paid(*d) && self.due(*d) >= from && self.due(*d) <= to)
            .map(|d| self.amount(d))
            .sum()
    }

    /// What is still owed on every line there is — the credit stock, walked rather than stored.
    pub fn outstanding_total(&self) -> f64 {
        (0..self.len() as u32)
            .map(DueId)
            .filter(|d| !self.paid(*d))
            .map(|d| self.amount(d))
            .sum()
    }

    /// What is still owed on a line, read from the rows rather than kept beside them.
    pub fn outstanding(&self, i: InstrumentId) -> f64 {
        self.of_instrument(i)
            .iter()
            .map(|r| DueId(*r))
            .filter(|d| !self.paid(*d))
            .map(|d| self.amount(d))
            .sum()
    }
}

/// Every deciding party has its own outlook, formed from its OWN history.
#[derive(Default)]
pub struct Outlooks {
    party: Vec<u32>,
    about: Vec<u32>,
    level: Vec<f64>,
    /// The period it was last formed in, so a stale outlook is visibly stale.
    formed: Vec<u32>,
    at: HashMap<u64, u32>,
    by_party: HashMap<u32, Vec<u32>>,
}

#[inline]
const fn held_by(party: u32, about: u32) -> u64 {
    ((party as u64) << 32) | (about as u64)
}

impl Outlooks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.party.len()
    }

    pub fn is_empty(&self) -> bool {
        self.party.is_empty()
    }

    /// It is formed ADAPTIVELY from what the party itself saw — the caller does the forming, because
    /// how much weight to give the surprise is that party's own PREFERENCE (one primitive).
    pub fn form(&mut self, party: PartyId, about: u32, level: f64, period: u32) {
        assert!(party.some(), "§46: an outlook with no holder is a global expectation");
        assert!(level.is_finite(), "Appendix A: an outlook of NaN is not an outlook");
        let key = held_by(party.0, about);
        match self.at.get(&key) {
            Some(row) => {
                let row = *row as usize;
                self.level[row] = level;
                self.formed[row] = period;
            }
            None => {
                let row = self.party.len() as u32;
                self.party.push(party.0);
                self.about.push(about);
                self.level.push(level);
                self.formed.push(period);
                self.at.insert(key, row);
                self.by_party.entry(party.0).or_default().push(row);
            }
        }
    }

    /// What this party expects of this thing.
    pub fn of(&self, party: PartyId, about: u32) -> Option<f64> {
        self.at.get(&held_by(party.0, about)).map(|row| self.level[*row as usize])
    }

    /// When it was formed, so a reader can see that it is old.
    pub fn formed(&self, party: PartyId, about: u32) -> Option<u32> {
        self.at.get(&held_by(party.0, about)).map(|row| self.formed[*row as usize])
    }

    /// How much they disagree, which is what a shock transmits through.
    pub fn spread_on(&self, about: u32) -> Vec<f64> {
        self.party
            .iter()
            .enumerate()
            .filter(|(row, _)| self.about[*row] == about)
            .map(|(row, _)| self.level[row])
            .collect()
    }

    pub fn of_party(&self, p: PartyId) -> &[u32] {
        match self.by_party.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    #[inline]
    pub fn about_at(&self, row: u32) -> u32 {
        self.about[row as usize]
    }

    #[inline]
    pub fn level_at(&self, row: u32) -> f64 {
        self.level[row as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProcessId(pub u32);

impl ProcessId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// Something in flight across periods, with an owner and an end.
#[derive(Default)]
pub struct Processes {
    kind: Vec<u32>,
    owner: Vec<u32>,
    began: Vec<u32>,
    /// The period it is expected to close in.
    closes: Vec<Option<u32>>,
    size: Vec<f64>,
    done: Vec<bool>,
    by_owner: HashMap<u32, Vec<u32>>,
    by_kind: HashMap<u32, Vec<u32>>,
}

impl Processes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    pub fn begin(&mut self, kind: u32, owner: PartyId, began: u32, closes: Option<u32>, size: f64) -> ProcessId {
        assert!(owner.some(), "XI-3: a process with no owner is one nobody has to finish");
        if let Some(end) = closes {
            assert!(end >= began, "a process that closes before it began is not one");
        }
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.owner.push(owner.0);
        self.began.push(began);
        self.closes.push(closes);
        self.size.push(size);
        self.done.push(false);
        self.by_owner.entry(owner.0).or_default().push(row);
        self.by_kind.entry(kind).or_default().push(row);
        ProcessId(row)
    }

    #[inline]
    pub fn kind_of(&self, p: ProcessId) -> u32 {
        self.kind[p.row()]
    }

    #[inline]
    pub fn owner(&self, p: ProcessId) -> PartyId {
        PartyId(self.owner[p.row()])
    }

    #[inline]
    pub fn began(&self, p: ProcessId) -> u32 {
        self.began[p.row()]
    }

    #[inline]
    pub fn closes(&self, p: ProcessId) -> Option<u32> {
        self.closes[p.row()]
    }

    #[inline]
    pub fn size(&self, p: ProcessId) -> f64 {
        self.size[p.row()]
    }

    #[inline]
    pub fn done(&self, p: ProcessId) -> bool {
        self.done[p.row()]
    }

    pub fn finish(&mut self, p: ProcessId) {
        self.done[p.row()] = true;
    }

    /// What is still running of this kind — which is what a mechanism asks every period.
    pub fn running(&self, kind: u32) -> Vec<ProcessId> {
        self.of_kind(kind).iter().map(|r| ProcessId(*r)).filter(|p| !self.done(*p)).collect()
    }

    pub fn of_owner(&self, p: PartyId) -> &[u32] {
        match self.by_owner.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// WHO IS OWED WHAT BY A PARTY WHOSE LIFE HAS ENDED, AND AT WHAT RANK.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClaimId(pub u32);

#[derive(Default)]
pub struct Claims {
    /// Whose estate it is a claim ON.
    on: Vec<u32>,
    /// And who holds it.
    holder: Vec<u32>,
    owed: Vec<f64>,
    /// Where it stands.
    ranks: Vec<u32>,
    paid: Vec<f64>,
    by_estate: HashMap<u32, Vec<u32>>,
    /// Both directions.
    by_holder: HashMap<u32, Vec<u32>>,
}

impl Claims {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.on.len()
    }

    pub fn is_empty(&self) -> bool {
        self.on.is_empty()
    }

    /// The only way to make one.
    pub fn against(&mut self, estate: PartyId, holder: PartyId, owed: f64, ranks: u32) -> ClaimId {
        assert!(estate != holder, "XI-8: a party is not a claimant on its own estate");
        assert!(owed > 0.0, "XI-8: a claim for {owed} is not a claim");
        let row = self.on.len() as u32;
        self.on.push(estate.0);
        self.holder.push(holder.0);
        self.owed.push(owed);
        self.ranks.push(ranks);
        self.paid.push(0.0);
        self.by_estate.entry(estate.0).or_default().push(row);
        self.by_holder.entry(holder.0).or_default().push(row);
        ClaimId(row)
    }

    /// Every claim on one estate, which is what a waterfall needs and the only walk it should have
    /// to do.
    pub fn on_estate(&self, estate: PartyId) -> &[u32] {
        match self.by_estate.get(&estate.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// The other direction.
    pub fn held_by(&self, holder: PartyId) -> &[u32] {
        match self.by_holder.get(&holder.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// What this party still owes on claims against it, and what it is still owed on claims it
    /// holds.
    pub fn owed_by_estate(&self, estate: PartyId) -> f64 {
        self.on_estate(estate).iter().map(|r| self.outstanding(ClaimId(*r))).sum()
    }

    pub fn owed_to(&self, holder: PartyId) -> f64 {
        self.held_by(holder).iter().map(|r| self.outstanding(ClaimId(*r))).sum()
    }

    pub fn holder_of(&self, c: ClaimId) -> PartyId {
        PartyId(self.holder[c.0 as usize])
    }

    pub fn owed(&self, c: ClaimId) -> f64 {
        self.owed[c.0 as usize]
    }

    pub fn ranks(&self, c: ClaimId) -> u32 {
        self.ranks[c.0 as usize]
    }

    pub fn paid(&self, c: ClaimId) -> f64 {
        self.paid[c.0 as usize]
    }

    /// What a claimant did not get is a LOSS on a named holder, and it is a read of the two numbers
    /// rather than a third one somebody keeps.
    pub fn outstanding(&self, c: ClaimId) -> f64 {
        self.owed[c.0 as usize] - self.paid[c.0 as usize]
    }

    /// What the waterfall actually paid it.
    pub fn pays(&mut self, c: ClaimId, amount: f64) {
        self.paid[c.0 as usize] += amount;
    }
}

/// TERMS A NAMED PARTY CURRENTLY STANDS BEHIND, AND WILL UNTIL IT WITHDRAWS THEM.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StandingId(pub u32);

#[derive(Default)]
pub struct Standing {
    kind: Vec<u32>,
    who: Vec<u32>,
    about: Vec<u32>,
    term_at: Vec<u32>,
    term_len: Vec<u32>,
    terms: Vec<f64>,
    since: Vec<u32>,
    live: Vec<bool>,
    by_party: HashMap<u32, Vec<u32>>,
    by_kind: HashMap<u32, Vec<u32>>,
}

impl Standing {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    /// The only way to make one.
    pub fn stands(&mut self, kind: u32, who: PartyId, about: PartyId, terms: &[f64], since: u32) -> StandingId {
        assert!(who.some(), "XI-10: a standing offer is HELD by a named party, or nobody can withdraw it");
        assert!(!terms.is_empty(), "Law 8: terms nobody stated are not terms");
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.who.push(who.0);
        self.about.push(about.0);
        self.term_at.push(self.terms.len() as u32);
        self.term_len.push(terms.len() as u32);
        self.terms.extend_from_slice(terms);
        self.since.push(since);
        self.live.push(true);
        self.by_party.entry(who.0).or_default().push(row);
        self.by_kind.entry(kind).or_default().push(row);
        StandingId(row)
    }

    /// Withdrawn, by the party that held it.
    pub fn withdraw(&mut self, s: StandingId) {
        self.live[s.0 as usize] = false;
    }

    /// A standard TIGHTENS — the same party, standing behind different terms from now.
    pub fn restates(&mut self, s: StandingId, terms: &[f64], now: u32) -> StandingId {
        let (kind, who, about) =
            (self.kind[s.0 as usize], PartyId(self.who[s.0 as usize]), PartyId(self.about[s.0 as usize]));
        self.withdraw(s);
        self.stands(kind, who, about, terms, now)
    }

    pub fn live(&self, s: StandingId) -> bool {
        self.live[s.0 as usize]
    }

    #[inline]
    pub fn about(&self, s: StandingId) -> PartyId {
        PartyId(self.about[s.0 as usize])
    }

    /// 21 A4, A6, 22i.2: what this party is standing behind about THAT one, live — the read a grade
    /// is.
    pub fn of_party_about(&self, who: PartyId, about: PartyId, kind: u32) -> Option<StandingId> {
        self.of_party(who)
            .iter()
            .map(|r| StandingId(*r))
            .find(|s| self.live(*s) && self.kind_of(*s) == kind && self.about(*s) == about)
    }

    pub fn held_by(&self, s: StandingId) -> PartyId {
        PartyId(self.who[s.0 as usize])
    }

    pub fn kind_of(&self, s: StandingId) -> u32 {
        self.kind[s.0 as usize]
    }

    pub fn since(&self, s: StandingId) -> u32 {
        self.since[s.0 as usize]
    }

    pub fn terms(&self, s: StandingId) -> &[f64] {
        let at = self.term_at[s.0 as usize] as usize;
        let len = self.term_len[s.0 as usize] as usize;
        &self.terms[at..at + len]
    }

    /// Both directions.
    pub fn of_party(&self, p: PartyId) -> &[u32] {
        match self.by_party.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// A mechanism asks for ITS OWN kind's rows.
    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// WORK IN PROGRESS EXISTS BETWEEN INPUT AND OUTPUT, OWNED BY SOMEBODY, AND IT CARRIES WHAT IT COST.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BatchId(pub u32);

#[derive(Default)]
pub struct InProgress {
    owner: Vec<u32>,
    what: Vec<u32>,
    units: Vec<f64>,
    /// What it cost — the inputs, the wages and the capital charge that went in when it was started,
    /// carried with the batch so the unit cost of what comes out is what went into it.
    cost_carried: Vec<f64>,
    started: Vec<u32>,
    ready: Vec<u32>,
    taken: Vec<bool>,
    by_owner: HashMap<u32, Vec<u32>>,
    /// By the period it is ready in, so `ready_in` is a lookup and not a walk over every batch this
    /// world has ever run.
    by_ready: BTreeMap<u32, Vec<u32>>,
}

impl InProgress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.owner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.owner.is_empty()
    }

    pub fn starts(
        &mut self,
        owner: PartyId,
        what: InstrumentId,
        units: f64,
        cost_carried: f64,
        started: u32,
        ready: u32,
    ) -> BatchId {
        assert!(owner.some(), "37 B3: work in progress is owned by SOMEBODY");
        assert!(units > 0.0, "37 B3: a batch of {units} is not work in progress");
        assert!(ready > started, "37 B3: a batch ready in the period it started is not in progress");
        let row = self.owner.len() as u32;
        self.owner.push(owner.0);
        self.what.push(what.0);
        self.units.push(units);
        self.cost_carried.push(cost_carried);
        self.started.push(started);
        self.ready.push(ready);
        self.taken.push(false);
        self.by_owner.entry(owner.0).or_default().push(row);
        self.by_ready.entry(ready).or_default().push(row);
        BatchId(row)
    }

    /// What comes off the line this period — the batches whose time is up and which nobody has taken
    /// yet.
    pub fn ready_in(&self, period: u32) -> Vec<BatchId> {
        self.by_ready
            .range(..=period)
            .flat_map(|(_, rows)| rows.iter().map(|r| BatchId(*r)))
            .filter(|b| !self.taken[b.0 as usize])
            .collect()
    }

    /// Taken off the line.
    pub fn finishes(&mut self, b: BatchId) {
        self.taken[b.0 as usize] = true;
        let when = self.ready[b.0 as usize];
        if let Some(rows) = self.by_ready.get_mut(&when) {
            rows.retain(|r| *r != b.0);
            if rows.is_empty() {
                self.by_ready.remove(&when);
            }
        }
    }

    pub fn owner_of(&self, b: BatchId) -> PartyId {
        PartyId(self.owner[b.0 as usize])
    }

    pub fn what(&self, b: BatchId) -> InstrumentId {
        InstrumentId(self.what[b.0 as usize])
    }

    pub fn units(&self, b: BatchId) -> f64 {
        self.units[b.0 as usize]
    }

    pub fn cost_carried(&self, b: BatchId) -> f64 {
        self.cost_carried[b.0 as usize]
    }

    /// What this party has on the line, at what it cost.
    pub fn held_by(&self, owner: PartyId) -> Vec<BatchId> {
        match self.by_owner.get(&owner.0) {
            Some(rows) => rows.iter().map(|r| BatchId(*r)).filter(|b| !self.taken[b.0 as usize]).collect(),
            None => Vec::new(),
        }
    }
}


/// 3 C2, 22c.2: where a resting order lives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RestingId(pub u32);

impl RestingId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// 3 C2, 22c.2: AN ORDER RESTS.
#[derive(Default)]
pub struct Resting {
    party: Vec<u32>,
    venue: Vec<u32>,
    /// `true` for a buy.
    buying: Vec<bool>,
    /// `None` is an order with no level: it takes what the venue gives it.
    level: Vec<Option<f64>>,
    /// TOTAL pieces, as a whole count.
    left: Vec<i64>,
    from: Vec<u32>,
    /// `Missing` where it rests until somebody cancels it — which is not the same as ending today.
    until: Vec<Option<i64>>,
    live: Vec<bool>,
    /// WHY it was entered, so an order in a book can be traced to the decision that put it there.
    why: Vec<u32>,
    by_venue: HashMap<u32, Vec<u32>>,
    by_party: HashMap<u32, Vec<u32>>,
}

impl Resting {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.party.len()
    }

    pub fn is_empty(&self) -> bool {
        self.party.is_empty()
    }

    /// The only way to enter one.
    #[allow(clippy::too_many_arguments)]
    pub fn enters(
        &mut self,
        party: PartyId,
        venue: u32,
        buying: bool,
        level: Option<f64>,
        qty: i64,
        from: u32,
        until: Option<Day>,
        why: u32,
    ) -> RestingId {
        assert!(party.some(), "Clearing B2: an order is somebody's");
        assert!(qty > 0, "Clearing C1: an order for {qty} pieces is not an order");
        if let Some(p) = level {
            assert!(p.is_finite(), "Law 6: a level of {p} is not a level");
        }
        let row = self.party.len() as u32;
        self.party.push(party.0);
        self.venue.push(venue);
        self.buying.push(buying);
        self.level.push(level);
        self.left.push(qty);
        self.from.push(from);
        self.until.push(until.map(|d| d.0));
        self.live.push(true);
        self.why.push(why);
        self.by_venue.entry(venue).or_default().push(row);
        self.by_party.entry(party.0).or_default().push(row);
        RestingId(row)
    }

    /// Its OWNER cancels it, and nobody else.
    pub fn cancels(&mut self, o: RestingId, by: PartyId) {
        assert!(
            self.party[o.row()] == by.0,
            "Clearing B2: {by:?} did not enter this order and cannot pull it"
        );
        self.live[o.row()] = false;
    }

    /// A match consumes it, and what is left of a partly filled order is a smaller order — not a
    /// filled one, and not a new one somebody re-entered (Law 4: the row is the order).
    pub fn took(&mut self, o: RestingId, qty: i64) {
        assert!(qty > 0, "3 C2: a fill of nothing did not happen");
        assert!(
            qty <= self.left[o.row()],
            "3 C2: {qty} taken of {} left — a fill cannot exceed the order",
            self.left[o.row()]
        );
        self.left[o.row()] -= qty;
        if self.left[o.row()] == 0 {
            self.live[o.row()] = false;
        }
    }

    /// The CALENDAR expires it: an order rests until its own date, which is a date and never a count
    /// of periods.
    pub fn expire(&mut self, on: Day) {
        for row in 0..self.party.len() {
            if self.live[row] && matches!(self.until[row], Some(end) if end < on.0) {
                self.live[row] = false;
            }
        }
    }

    #[inline]
    pub fn live(&self, o: RestingId) -> bool {
        self.live[o.row()]
    }

    #[inline]
    pub fn left(&self, o: RestingId) -> i64 {
        self.left[o.row()]
    }

    #[inline]
    pub fn owner(&self, o: RestingId) -> PartyId {
        PartyId(self.party[o.row()])
    }

    #[inline]
    pub fn level(&self, o: RestingId) -> Option<f64> {
        self.level[o.row()]
    }

    #[inline]
    pub fn venue_of(&self, o: RestingId) -> u32 {
        self.venue[o.row()]
    }

    #[inline]
    pub fn buying(&self, o: RestingId) -> bool {
        self.buying[o.row()]
    }

    #[inline]
    pub fn why(&self, o: RestingId) -> u32 {
        self.why[o.row()]
    }

    #[inline]
    pub fn entered(&self, o: RestingId) -> u32 {
        self.from[o.row()]
    }

    /// Every session opens with the standing book.
    pub fn at(&self, venue: u32) -> Vec<RestingId> {
        match self.by_venue.get(&venue) {
            Some(rows) => rows.iter().map(|r| RestingId(*r)).filter(|o| self.live(*o)).collect(),
            None => Vec::new(),
        }
    }

    pub fn of_party(&self, p: PartyId) -> Vec<RestingId> {
        match self.by_party.get(&p.0) {
            Some(rows) => rows.iter().map(|r| RestingId(*r)).filter(|o| self.live(*o)).collect(),
            None => Vec::new(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn an_agreement_has_two_sides_and_is_found_from_either_of_them() {
        // A relation with one party is a decision, and a store that could only be read from one end
        // would make the other side's obligation invisible.
        let mut a = Agreements::new();
        let hired = a.strike(0, party(1), party(2), &[40.0, 7.0], Day(-100), None);
        assert_eq!(a.between(hired), (party(1), party(2)));
        assert_eq!(a.terms(hired), &[40.0, 7.0]);
        assert_eq!(a.of_party(party(1)), &[0]);
        assert_eq!(a.of_party(party(2)), &[0]);
        assert_eq!(a.of_kind(0), &[0]);
        assert!(a.live(hired) && a.until(hired).is_none());
    }

    #[test]
    #[should_panic(expected = "a party does not agree with itself")]
    fn a_party_cannot_agree_with_itself() {
        Agreements::new().strike(0, party(1), party(1), &[], Day(0), None);
    }

    #[test]
    fn an_agreement_ends_and_the_ending_is_recorded() {
        // A relation that stops existing without anybody ending it is a silent disappearance.
        let mut a = Agreements::new();
        let hired = a.strike(0, party(1), party(2), &[40.0], Day(-100), Some(Day(100)));
        a.end(hired);
        assert!(!a.live(hired));
        // And it is still THERE: what ended is readable, which is what makes a history one.
        assert_eq!(a.of_party(party(1)).len(), 1);
    }

    #[test]
    fn a_schedule_says_what_falls_due_in_a_period_without_walking_the_world() {
        // By day, so a mechanism reads what is due rather than every schedule there
        // is.
        let mut s = Schedules::new();
        let line = InstrumentId::at(3);
        let usd = crate::ids::CurrencyCode::at(0);
        let pays = |from, due, amount, of| Payment { from: Day(from), due: Day(due), amount, of };
        let coupon = s.owes(Owed::On(line), party(1), usd, pays(0, 10, 5.0, Owing::Interest));
        s.owes(Owed::On(line), party(1), usd, pays(100, 100, 100.0, Owing::Principal));
        s.owes(Owed::On(InstrumentId::at(4)), party(2), usd, pays(0, 12, 9.0, Owing::Premium));
        // And one owed to a NAMED party rather than on a line, which falls due the same way and is
        // not on the line's books.
        let call = s.owes(Owed::To(party(9)), party(2), usd, pays(8, 8, 40.0, Owing::Call));

        let this_week = s.falling(Day(7), Day(14));
        assert_eq!(this_week.len(), 3, "three payments fall in the window and the fourth does not");
        assert_eq!(s.outstanding(line), 105.0, "the bilateral one is nobody's line");
        assert_eq!(s.on(call), Owed::To(party(9)));
        assert_eq!(s.accrued(call, Day(8)), None, "a call covers no days");

        // Bond N9.b: what has accrued on the coupon is a READ over its own interval, and half way
        // through it is half the coupon. A principal covers no days and accrues nothing.
        assert_eq!(s.accrued(coupon, Day(5)), Some(2.5));
        assert_eq!(s.accrued(coupon, Day(0)), Some(0.0));
        assert_eq!(s.accruing(line, Day(5)), Some(coupon));
        // Past its due date the line is accruing on nothing: that coupon is owed, not accruing.
        assert_eq!(s.accruing(line, Day(10)), None);
    }

    #[test]
    fn what_is_paid_stops_falling_due_and_what_is_not_stays_readable() {
        // A-20: settled is a recorded state, and the arrear is what is NOT marked.
        let mut s = Schedules::new();
        let line = InstrumentId::at(3);
        let usd = crate::ids::CurrencyCode::at(0);
        let pays = |from, due, amount| Payment { from: Day(from), due: Day(due), amount, of: Owing::Interest };
        let first = s.owes(Owed::On(line), party(1), usd, pays(0, 10, 5.0));
        s.owes(Owed::On(line), party(1), usd, pays(10, 11, 6.0));
        s.apply(crate::ledger::DueUpdate {
            due: first,
            outcome: crate::ledger::DueOutcome::Settled { on: Day(10) },
        });
        assert_eq!(s.falling(Day(0), Day(20)).len(), 1);
        assert_eq!(s.outstanding(line), 6.0);
        assert!(s.paid(first));
    }

    #[test]
    fn an_outlook_belongs_to_one_party_and_two_of_them_may_disagree() {
        // The disagreement is LOAD-BEARING — it is what gives a market two sides.
        let mut o = Outlooks::new();
        o.form(party(1), 7, 1.20, 3);
        o.form(party(2), 7, 0.80, 3);
        assert_eq!(o.of(party(1), 7), Some(1.20));
        assert_eq!(o.of(party(2), 7), Some(0.80));
        let mut spread = o.spread_on(7);
        spread.sort_by(|a, b| a.partial_cmp(b).expect("no NaN reaches a series"));
        assert_eq!(spread, vec![0.80, 1.20]);
    }

    #[test]
    fn a_party_that_never_formed_one_has_none_and_that_is_not_zero() {
        // Missing is missing.
        let mut o = Outlooks::new();
        o.form(party(1), 7, 1.20, 3);
        assert_eq!(o.of(party(2), 7), None);
        assert_eq!(o.of(party(1), 8), None);
        assert_eq!(o.formed(party(1), 7), Some(3));
    }

    #[test]
    fn re_forming_an_outlook_replaces_it_rather_than_keeping_two() {
        // One writer, one fact.
        let mut o = Outlooks::new();
        o.form(party(1), 7, 1.20, 3);
        o.form(party(1), 7, 1.05, 4);
        assert_eq!(o.len(), 1);
        assert_eq!(o.of(party(1), 7), Some(1.05));
        assert_eq!(o.formed(party(1), 7), Some(4));
    }

    #[test]
    fn a_process_is_in_flight_until_somebody_finishes_it() {
        // Nothing is immortal, a process included.
        let mut p = Processes::new();
        let building = p.begin(0, party(1), 2, Some(9), 500.0);
        let other = p.begin(0, party(2), 2, None, 20.0);
        assert_eq!(p.running(0).len(), 2);
        p.finish(building);
        assert_eq!(p.running(0), vec![other]);
        assert_eq!(p.owner(building), party(1));
        assert_eq!(p.size(building), 500.0);
        assert_eq!(p.closes(other), None, "an end that is itself an outcome is Missing, not a guess");
    }

    #[test]
    fn a_claim_on_an_estate_names_both_sides_and_what_it_did_not_get_is_a_read() {
        // A claimant is a named holder and the loss is what was owed less what arrived.
        let mut c = Claims::new();
        let estate = PartyId::at(3);
        let treasury = PartyId::at(9);
        let one = c.against(estate, treasury, 388.0, 1);
        assert_eq!(c.holder_of(one), treasury);
        assert_eq!(c.outstanding(one), 388.0);
        c.pays(one, 288.0);
        assert_eq!(c.paid(one), 288.0);
        assert_eq!(c.outstanding(one), 100.0);
    }

    #[test]
    fn every_claim_on_one_estate_is_one_read_and_not_a_walk_over_all_of_them() {
        // A waterfall needs the claims on ITS estate, and that is the only walk.
        let mut c = Claims::new();
        let (a, b) = (PartyId::at(3), PartyId::at(4));
        c.against(a, PartyId::at(9), 100.0, 1);
        c.against(b, PartyId::at(9), 200.0, 1);
        c.against(a, PartyId::at(8), 300.0, 2);
        assert_eq!(c.on_estate(a).len(), 2);
        assert_eq!(c.on_estate(b).len(), 1);
        // A party nobody has a claim on has none, which is an answer and not a missing row.
        assert!(c.on_estate(PartyId::at(5)).is_empty());
    }

    #[test]
    #[should_panic(expected = "is not a claim")]
    fn a_claim_for_nothing_is_not_a_claim() {
        // A claimant with no claim would take a share of the rank it stands in.
        Claims::new().against(PartyId::at(3), PartyId::at(9), 0.0, 1);
    }

    #[test]
    #[should_panic(expected = "not a claimant on its own estate")]
    fn a_party_is_not_a_claimant_on_its_own_estate() {
        // Both sides, and they are two.
        Claims::new().against(PartyId::at(3), PartyId::at(3), 100.0, 1);
    }

    #[test]
    fn a_posting_is_held_by_somebody_and_that_is_what_lets_it_be_withdrawn() {
        // A vacancy that stops existing without anybody withdrawing it is the silent disappearance
        // Law 5 is against.
        let mut s = Standing::new();
        let employer = party(1);
        let p = s.stands(7, employer, PartyId::NONE, &[900.0, 30.0], 4);
        assert!(s.live(p));
        assert_eq!(s.held_by(p), employer);
        assert_eq!(s.terms(p), &[900.0, 30.0]);
        assert_eq!(s.of_party(employer).len(), 1);
        assert_eq!(s.of_kind(7).len(), 1);

        s.withdraw(p);
        assert!(!s.live(p), "it is withdrawn BY the party that held it");
        // And the row stays, because a vacancy that existed is a fact somebody may read.
        assert_eq!(s.of_party(employer).len(), 1);
    }

    #[test]
    fn a_standard_that_tightens_leaves_the_one_it_replaced_readable() {
        // A standard is a DECISION that tightens, and a tightening is only visible against what it
        // was.
        let mut s = Standing::new();
        let lender = party(2);
        let was = s.stands(9, lender, PartyId::NONE, &[4.0, 0.10], 1);
        let now = s.restates(was, &[3.0, 0.25], 6);

        assert!(!s.live(was));
        assert!(s.live(now));
        assert_eq!(s.terms(was), &[4.0, 0.10], "what it WAS lending at is still there");
        assert_eq!(s.terms(now), &[3.0, 0.25]);
        assert_eq!(s.held_by(now), lender);
        assert_eq!(s.since(now), 6);
        assert_eq!(s.of_kind(9).len(), 2);
    }

    #[test]
    #[should_panic(expected = "or nobody can withdraw it")]
    fn a_standing_offer_nobody_holds_is_not_one() {
        Standing::new().stands(7, PartyId::NONE, PartyId::NONE, &[1.0], 0);
    }

    #[test]
    fn work_in_progress_is_owned_carries_its_cost_and_comes_off_when_its_time_is_up() {
        // 37 B3, 21f.3: the clause whose type existed and whose instances did not.
        let mut w = InProgress::new();
        let maker = party(5);
        let good = InstrumentId::at(9);
        let b = w.starts(maker, good, 120.0, 960.0, 3, 5);

        assert_eq!(w.owner_of(b), maker);
        assert_eq!(w.what(b), good);
        assert_eq!(w.units(b), 120.0);
        assert_eq!(w.cost_carried(b), 960.0);
        // It is on the line, and it is this party's to say so on its own balance sheet.
        assert_eq!(w.held_by(maker).len(), 1);
        assert!(w.ready_in(4).is_empty(), "it is not ready before its time");
        assert_eq!(w.ready_in(5).len(), 1);

        w.finishes(b);
        assert!(w.ready_in(5).is_empty(), "taken once, and not offered again");
        assert!(w.held_by(maker).is_empty(), "it stopped being in progress when it became a holding");
    }

    #[test]
    fn an_order_rests_until_its_owner_pulls_it_its_date_expires_it_or_a_match_takes_it() {
        // 3 C2, 22c.2: the four ways an order stops standing, and nothing else.
        let mut book = Resting::new();
        let seller = party(5);
        let buyer = party(6);
        let venue = 3u32;
        let ask = book.enters(seller, venue, false, Some(2.0), 100, 1, None, 0);
        let bid = book.enters(buyer, venue, true, Some(3.0), 40, 1, Some(Day(20)), 0);

        // Every session opens with the standing book, and both directions are indexed.
        assert_eq!(book.at(venue).len(), 2);
        assert_eq!(book.of_party(seller), vec![ask]);

        // A MATCH consumes it — and what is left of a partly filled order is a smaller order, not a
        // filled one and not a new one somebody re-entered (Law 4: the row IS the order).
        book.took(ask, 30);
        assert_eq!(book.left(ask), 70);
        assert!(book.live(ask));
        book.took(ask, 70);
        assert!(!book.live(ask), "an order with nothing left of it is not standing");

        // The CALENDAR expires it, by DATE and never by a count of periods.
        book.expire(Day(20));
        assert!(book.live(bid), "its own day has not passed");
        book.expire(Day(21));
        assert!(!book.live(bid));
        assert!(book.at(venue).is_empty());
    }

    #[test]
    #[should_panic(expected = "did not enter this order")]
    fn only_its_owner_pulls_it() {
        // An order somebody else can pull is not that party's order.
        let mut book = Resting::new();
        let o = book.enters(party(5), 1, false, Some(2.0), 10, 1, None, 0);
        book.cancels(o, party(6));
    }

    #[should_panic(expected = "ready in the period it started")]
    #[test]
    fn a_batch_that_finishes_where_it_started_was_never_in_progress() {
        // Which is exactly what production did before the recipe had a lead time.
        InProgress::new().starts(party(5), InstrumentId::at(9), 10.0, 100.0, 3, 3);
    }
}
