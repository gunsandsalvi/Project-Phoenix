//! THE STORES THE MODULES NEEDED AND THE KERNEL DID NOT HAVE.
//!
//! @spec ARCHITECTURE 4.9b · XI-10 · XI-15 · §46 · Law 4, Law 8, Law 10, Law 19 · Appendix B
//!
//! The kernel owned seven stores — parties, instruments, register, prints, journal, wire, params —
//! and every module carried its own state types with nowhere to live: an `Engagement`, a `Loan`'s
//! schedule, an `Owed`, a `Subscription`, a `Dwelling`, a `Mortgage`, an `Outlook`, a `Platform`.
//! So a mechanism for employment had no engagements to read and one for housing had no dwellings,
//! and fifty systems were listed as wired while one of them ran.
//!
//! **These are four stores and not forty because the things are four kinds of thing.** CLAUDE.md
//! names two of them out loud — *a standing-order register as a kernel noun beside AGREEMENTS and
//! PROCESSES* — and §46 names the third. What the modules were each inventing was:
//!
//! - an **agreement**: two named parties, terms, a start and an end. An engagement, a mortgage, a
//!   policy, a supply contract, a subscription, a tenancy, a mandate.
//! - a **schedule**: what an instrument owes, and when. A loan's repayments, a bond's coupons, a
//!   premium due, a rent.
//! - an **outlook**: what one party expects of one thing, formed from its own history (§46).
//! - a **process**: something in flight across periods with an owner and an end. A capital
//!   programme, a foreclosure, a buy-back, an election, a workout.
//!
//! **A kind is DATA here, never a branch** (Law 15). `Agreements` does not know what an engagement
//! is; it holds a kind id the registry declares, and a mechanism asks for its own kind's rows. That
//! is what stops this becoming one store with a `match` in it.
//!
//! **Both directions are indexed where a reader needs them** (Register A3), because the alternative
//! is every mechanism walking every row every period — which is the quadratic the whole engine was
//! rewritten to escape.

use crate::calendar::Day;
use crate::ids::{InstrumentId, PartyId};
use std::collections::{BTreeMap, HashMap};

/// Where an agreement's terms live. A term is a NUMBER THE PARTIES AGREED — a wage, a rate, a rent,
/// a notice period — and what each position means is the declaring kind's business, not this store's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AgreementId(pub u32);

impl AgreementId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// XI-10, 17f: **an agreement is a relation, recorded** — two named parties, terms, a start and an
/// end. Employment is one, a mortgage is one, a policy is one. What they have in common is that they
/// are not an instrument anybody holds and not an event that happened once.
#[derive(Default)]
pub struct Agreements {
    kind: Vec<u32>,
    /// Law 5: two sides, always. An agreement with one party is a decision, not an agreement.
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

    /// The only way to make one. Both parties are named because a one-sided agreement is a defect
    /// even when nothing fails (Law 5).
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

    /// The terms as the kind declared them. Law 8: what each position means is the kind's, and a
    /// reader that wants the third one asks for the third one.
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

    /// 17f: **it ends, and the ending is recorded.** A relation that stops existing without anybody
    /// ending it is the kind of silent disappearance Law 5 is about.
    pub fn end(&mut self, a: AgreementId) {
        self.live[a.row()] = false;
    }

    /// XI-15, Labour A4.c: **the same relationship, now naming the cell that actually holds those
    /// people.** A split does not end an engagement and strike a new one — that would be a
    /// separation, with a severance owed and a start date lost — so the row moves rather than being
    /// replaced. The kernel calls this as part of the split; nothing else has business re-pointing
    /// an agreement at a party that did not agree it.
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

    /// Register A3: both directions. What this party is party to.
    pub fn of_party(&self, p: PartyId) -> &[u32] {
        match self.by_party.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// Law 15: a mechanism asks for ITS OWN kind's rows and never branches on somebody else's.
    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// §9 B4, §7 C9, 21.71: **a committed line, and there is one of them.** An issuer's backstop and a
/// borrower's facility were two structs in two modules describing the same real thing — a named
/// lender's commitment to a named borrower, at a limit, at a margin, with headroom it may draw — which
/// is the parallel representation Law 4 is about. This is that thing, once, as the terms of an
/// `agreed::COMMITMENT` agreement read back.
///
/// **The fee is not optional.** §9 B4: a committed line with no fee on its undrawn headroom is a free
/// option the lender did not sell, so `costs` refuses one (Law 5: the fee is paid to somebody). What
/// DOES vary is the END, and it varies the way every agreement's does: `until` is `Missing` for a line
/// that stands until somebody ends it.
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

    /// Whether this line is still available on a given day. A lapsed line is not a line.
    pub fn live_on(&self, day: Day) -> bool {
        match self.until {
            None => true,
            Some(end) => day <= end,
        }
    }
}

/// What a payment on a schedule IS to the party that owes it. Only one of them reduces what is owed,
/// and a store that could not tell them apart would make that distinction unreadable (Households E3.a).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owing {
    Interest,
    Principal,
    Premium,
    Rent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DueId(pub u32);

impl DueId {
    #[inline]
    pub const fn row(self) -> usize {
        self.0 as usize
    }
}

/// 5 D2, XI-9: **what an instrument owes and when.** A claim with terms and no schedule is a claim
/// nobody can fall behind on, which is why every maturity in the old world arrived at once.
#[derive(Default)]
pub struct Schedules {
    instrument: Vec<u32>,
    owed_by: Vec<u32>,
    due: Vec<i64>,
    amount: Vec<f64>,
    of: Vec<Owing>,
    paid: Vec<bool>,
    by_instrument: HashMap<u32, Vec<u32>>,
    /// 5 C3.a: by DAY, so "what falls due this period" is a read and not a walk of everything.
    by_day: BTreeMap<i64, Vec<u32>>,
    /// XI-9, 21j.1: by PAYER, so a party can be asked what IT must find — which is the question a
    /// participant deciding about money has, and the one this store could not answer.
    by_payer: HashMap<u32, Vec<u32>>,
}

impl Schedules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.instrument.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instrument.is_empty()
    }

    /// One payment, on one day, owed by one party. Appendix B: no liability without a beneficiary —
    /// the beneficiary is whoever holds the instrument when the day comes, which the register says.
    pub fn owes(
        &mut self,
        instrument: InstrumentId,
        owed_by: PartyId,
        due: Day,
        amount: f64,
        of: Owing,
    ) -> DueId {
        assert!(owed_by.some(), "Appendix B: no liability without somebody who owes it");
        assert!(amount > 0.0, "5 D2: a payment of nothing is not a payment that falls due");
        let row = self.instrument.len() as u32;
        self.instrument.push(instrument.0);
        self.owed_by.push(owed_by.0);
        self.due.push(due.0);
        self.amount.push(amount);
        self.of.push(of);
        self.paid.push(false);
        self.by_instrument.entry(instrument.0).or_default().push(row);
        self.by_day.entry(due.0).or_default().push(row);
        self.by_payer.entry(owed_by.0).or_default().push(row);
        DueId(row)
    }

    #[inline]
    pub fn instrument_of(&self, d: DueId) -> InstrumentId {
        InstrumentId(self.instrument[d.row()])
    }

    #[inline]
    pub fn owed_by(&self, d: DueId) -> PartyId {
        PartyId(self.owed_by[d.row()])
    }

    #[inline]
    pub fn due(&self, d: DueId) -> Day {
        Day(self.due[d.row()])
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

    /// A-20: settled is a recorded state. What is NOT marked is the arrear, and it stays readable.
    pub fn settle(&mut self, d: DueId) {
        self.paid[d.row()] = true;
    }

    /// **What falls due between two days**, which is what a period asks. A read over the index, so a
    /// mechanism that services its schedule does not walk every schedule in the world.
    pub fn falling(&self, from: Day, to: Day) -> Vec<DueId> {
        self.by_day
            .range(from.0..=to.0)
            .flat_map(|(_, rows)| rows.iter().map(|r| DueId(*r)))
            .filter(|d| !self.paid(*d))
            .collect()
    }

    /// XI-9, 21j.1: **what THIS PARTY must find, and by when.** Register A3's both-directions rule
    /// applied to the one index this store was missing: it could say what a LINE owed and what fell
    /// due on a DAY, and not what a party was on the hook for — which is the question every party
    /// deciding about money actually has.
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

    /// Law 19: what is still owed on a line, read from the rows rather than kept beside them.
    pub fn outstanding(&self, i: InstrumentId) -> f64 {
        self.of_instrument(i)
            .iter()
            .map(|r| DueId(*r))
            .filter(|d| !self.paid(*d))
            .map(|d| self.amount(d))
            .sum()
    }
}

/// §46, XI-16: **every deciding party has its own outlook, formed from its OWN history.** There is no
/// global expectation and no model forecast: this store holds one number per party per subject, and
/// the subject is whatever the asking module declared.
///
/// **Outlooks DISAGREE, and the disagreement is load-bearing** (§46 A3): it is what gives a market
/// two sides. A store that held one number per subject would have deleted that by construction.
#[derive(Default)]
pub struct Outlooks {
    party: Vec<u32>,
    about: Vec<u32>,
    level: Vec<f64>,
    /// The period it was last formed in, so a stale outlook is visibly stale (Law 8).
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

    /// §46 B1: it is formed ADAPTIVELY from what the party itself saw — the caller does the forming,
    /// because how much weight to give the surprise is that party's own PREFERENCE (one primitive).
    /// This records the answer and when it was reached.
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

    /// What this party expects of this thing. `Missing` is missing: a party that has never formed
    /// one has no outlook, and that is not an expectation of zero (Appendix A).
    pub fn of(&self, party: PartyId, about: u32) -> Option<f64> {
        self.at.get(&held_by(party.0, about)).map(|row| self.level[*row as usize])
    }

    /// Law 8: when it was formed, so a reader can see that it is old.
    pub fn formed(&self, party: PartyId, about: u32) -> Option<u32> {
        self.at.get(&held_by(party.0, about)).map(|row| self.formed[*row as usize])
    }

    /// §46 A3: **how much they disagree**, which is what a shock transmits through. A read over the
    /// holders of one subject — never a mean anybody stored.
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

/// **Something in flight across periods, with an owner and an end.** A capital programme being
/// built, a foreclosure running its course, a buy-back authorised, an election called, a workout.
///
/// XI-3: nothing is immortal, and that includes a process. One with no end is one nobody has to
/// finish, which is how a world accumulates things that never resolve.
#[derive(Default)]
pub struct Processes {
    kind: Vec<u32>,
    owner: Vec<u32>,
    began: Vec<u32>,
    /// The period it is expected to close in. `Missing` where that is itself an outcome.
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

/// **XI-8, Appendix B: WHO IS OWED WHAT BY A PARTY WHOSE LIFE HAS ENDED, AND AT WHAT RANK.**
///
/// The fifth kind of thing, and it arrived at 21c. An estate pays its claimants IN RANK ORDER, and
/// until now there was nowhere for a claimant to stand: `estate::Claim` was a value a function
/// returned and no store kept, so **the only way for anybody to be paid by an estate was to be paid
/// directly** — which is the whole of the finding (21.107: the treasury took 388 twice, and the
/// audit said it had no claim on the estate it took it from).
///
/// It is not an agreement: nobody entered into it, and a tax assessment is owed whether the payer
/// agreed to it or not. It is not a schedule: there is no instrument, and it is not due on a date —
/// it is due when the estate pays its rank. **No liability without beneficiaries** (Appendix B) is
/// what makes it the kernel's rather than a module's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClaimId(pub u32);

#[derive(Default)]
pub struct Claims {
    /// Whose estate it is a claim ON.
    on: Vec<u32>,
    /// And who holds it. Law 5: both sides are named, because a claim on nobody's behalf is not one.
    holder: Vec<u32>,
    owed: Vec<f64>,
    /// XI-8: where it stands. A rank is DATA here — this store does not know what `Preferential`
    /// means and never orders by it; the module that owns the waterfall does (Law 15).
    ranks: Vec<u32>,
    paid: Vec<f64>,
    by_estate: HashMap<u32, Vec<u32>>,
    /// Register A3, 21.110: **both directions.** A claim is an asset to the party that holds it and a
    /// liability to the estate it is on, and a store that could only be walked from the estate made
    /// the claimant's side unreadable — which is how an estate's equity came to ignore what it owed.
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

    /// The only way to make one. Both parties are named and the amount is positive: a claim for
    /// nothing is not a claim, and a claimant with no claim would take a share of its rank.
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

    /// Register A3: every claim on one estate, which is what a waterfall needs and the only walk it
    /// should have to do.
    pub fn on_estate(&self, estate: PartyId) -> &[u32] {
        match self.by_estate.get(&estate.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// Register A3: the other direction. Every claim this party holds, on whoever's estate.
    pub fn held_by(&self, holder: PartyId) -> &[u32] {
        match self.by_holder.get(&holder.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// 21.110: **what this party still owes on claims against it**, and what it is still owed on
    /// claims it holds. Both are reads of `outstanding` over one of the two indexes, and they are the
    /// two sides of the same rows (Law 5) — which is why they are here and not computed twice.
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

    /// XI-1: **what a claimant did not get is a LOSS on a named holder**, and it is a read of the
    /// two numbers rather than a third one somebody keeps.
    pub fn outstanding(&self, c: ClaimId) -> f64 {
        self.owed[c.0 as usize] - self.paid[c.0 as usize]
    }

    /// What the waterfall actually paid it. Law 6: nothing here refuses an overpayment by clamping
    /// it — an estate paying a claimant more than it owed is a defect for the audit to report, and a
    /// store that quietly absorbed it would be hiding the thing worth knowing.
    pub fn pays(&mut self, c: ClaimId, amount: f64) {
        self.paid[c.0 as usize] += amount;
    }
}

/// **TERMS A NAMED PARTY CURRENTLY STANDS BEHIND, AND WILL UNTIL IT WITHDRAWS THEM** (21f).
///
/// The one-sided twin of `Agreements`. An agreement has two sides, always (Law 5); this has one,
/// because a posting and a lending standard are not relations — they are a party's own declared
/// position, which anybody may take up or meet and nobody has agreed to.
///
/// Two homeless nouns turned out to be one shape. **A posting** is *a bid at the wage the employer
/// offers, and something an employer HOLDS — which is what lets it be withdrawn, by a named party,
/// as an event* (XI-10, §39 B). **A lending standard** is *a DECISION, and it tightens when the
/// lender is worried* (Housing C5) — a decision that persists and that every borrower meets or does
/// not. Both are terms, held by one party, standing until changed. A store per module would have
/// been the same store twice (Law 4).
///
/// What each position of `terms` means is the declaring kind's business, as it is for an agreement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StandingId(pub u32);

#[derive(Default)]
pub struct Standing {
    kind: Vec<u32>,
    who: Vec<u32>,
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

    /// The only way to make one. A party stands behind terms from a period; there is no anonymous
    /// standing offer, because a posting nobody holds cannot be withdrawn by anybody.
    pub fn stands(&mut self, kind: u32, who: PartyId, terms: &[f64], since: u32) -> StandingId {
        assert!(who.some(), "XI-10: a standing offer is HELD by a named party, or nobody can withdraw it");
        assert!(!terms.is_empty(), "Law 8: terms nobody stated are not terms");
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.who.push(who.0);
        self.term_at.push(self.terms.len() as u32);
        self.term_len.push(terms.len() as u32);
        self.terms.extend_from_slice(terms);
        self.since.push(since);
        self.live.push(true);
        self.by_party.entry(who.0).or_default().push(row);
        self.by_kind.entry(kind).or_default().push(row);
        StandingId(row)
    }

    /// **Withdrawn, by the party that held it.** The event the finding is about: a vacancy that
    /// stops existing without anybody withdrawing it is the silent disappearance Law 5 is against.
    pub fn withdraw(&mut self, s: StandingId) {
        self.live[s.0 as usize] = false;
    }

    /// C5: **a standard TIGHTENS** — the same party, standing behind different terms from now. The
    /// old row is withdrawn rather than overwritten, because what a lender was lending at last period
    /// is a fact somebody may read (Law 19, and it is how a tightening is visible at all).
    pub fn restates(&mut self, s: StandingId, terms: &[f64], now: u32) -> StandingId {
        let (kind, who) = (self.kind[s.0 as usize], PartyId(self.who[s.0 as usize]));
        self.withdraw(s);
        self.stands(kind, who, terms, now)
    }

    pub fn live(&self, s: StandingId) -> bool {
        self.live[s.0 as usize]
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

    /// Register A3: both directions. What this party is standing behind.
    pub fn of_party(&self, p: PartyId) -> &[u32] {
        match self.by_party.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// Law 15: a mechanism asks for ITS OWN kind's rows.
    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.by_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// **§37 B3: WORK IN PROGRESS EXISTS BETWEEN INPUT AND OUTPUT, OWNED BY SOMEBODY, AND IT CARRIES
/// WHAT IT COST** (21f).
///
/// The clause whose type existed and whose instances did not. It is not a register holding — the
/// units are not the finished good and nobody can be delivered them — and it is not a `Process`,
/// because a process carries one size and this carries a quantity AND the cost that went into it.
///
/// **A batch is started in one period and finishes in another** (`ready`), which is what makes there
/// be an in-between at all. Until the recipe had a lead time there was none: production drew its
/// inputs and created its output in one instruction, so nothing was ever in progress.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BatchId(pub u32);

#[derive(Default)]
pub struct InProgress {
    owner: Vec<u32>,
    what: Vec<u32>,
    units: Vec<f64>,
    /// B3: **what it cost** — the inputs, the wages and the capital charge that went in when it was
    /// started, carried with the batch so the unit cost of what comes out is what went into it.
    cost_carried: Vec<f64>,
    started: Vec<u32>,
    ready: Vec<u32>,
    taken: Vec<bool>,
    by_owner: HashMap<u32, Vec<u32>>,
    /// By the period it is ready in, so `ready_in` is a lookup and not a walk over every batch this
    /// world has ever run. The walk is the shape 21.138 is about, and a store built with it would
    /// have grown the period cost by its own history.
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
    /// yet. A read, so no caller keeps its own list of what is due (Law 19), and a range over the
    /// ready index rather than a walk: a batch that finished ten periods ago costs this nothing.
    pub fn ready_in(&self, period: u32) -> Vec<BatchId> {
        self.by_ready
            .range(..=period)
            .flat_map(|(_, rows)| rows.iter().map(|r| BatchId(*r)))
            .filter(|b| !self.taken[b.0 as usize])
            .collect()
    }

    /// Taken off the line. The batch stops being in progress because it became a thing somebody
    /// holds, and that is one event with two sides — this mark and the `Create` leg (Law 5).
    ///
    /// It comes out of the ready index as it goes, and an emptied period's entry goes with it, so
    /// what `ready_in` looks at is what is still ON the line rather than everything ever started.
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

    /// B3: what this party has on the line, at what it cost. A firm's balance sheet has to be able to
    /// say it, which is the reason the clause exists.
    pub fn held_by(&self, owner: PartyId) -> Vec<BatchId> {
        match self.by_owner.get(&owner.0) {
            Some(rows) => rows.iter().map(|r| BatchId(*r)).filter(|b| !self.taken[b.0 as usize]).collect(),
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
        // Law 5, Register A3: a relation with one party is a decision, and a store that could only
        // be read from one end would make the other side's obligation invisible.
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
        // 17f: a relation that stops existing without anybody ending it is a silent disappearance.
        let mut a = Agreements::new();
        let hired = a.strike(0, party(1), party(2), &[40.0], Day(-100), Some(Day(100)));
        a.end(hired);
        assert!(!a.live(hired));
        // And it is still THERE: what ended is readable, which is what makes a history one.
        assert_eq!(a.of_party(party(1)).len(), 1);
    }

    #[test]
    fn a_schedule_says_what_falls_due_in_a_period_without_walking_the_world() {
        // 5 C3.a, 5 D2: by day, so a mechanism reads what is due rather than every schedule there is.
        let mut s = Schedules::new();
        let line = InstrumentId::at(3);
        s.owes(line, party(1), Day(10), 5.0, Owing::Interest);
        s.owes(line, party(1), Day(100), 100.0, Owing::Principal);
        s.owes(InstrumentId::at(4), party(2), Day(12), 9.0, Owing::Premium);

        let this_week = s.falling(Day(7), Day(14));
        assert_eq!(this_week.len(), 2, "two payments fall in the window and the third does not");
        assert_eq!(s.outstanding(line), 105.0);
    }

    #[test]
    fn what_is_paid_stops_falling_due_and_what_is_not_stays_readable() {
        // A-20: settled is a recorded state, and the arrear is what is NOT marked.
        let mut s = Schedules::new();
        let line = InstrumentId::at(3);
        let first = s.owes(line, party(1), Day(10), 5.0, Owing::Interest);
        s.owes(line, party(1), Day(11), 6.0, Owing::Interest);
        s.settle(first);
        assert_eq!(s.falling(Day(0), Day(20)).len(), 1);
        assert_eq!(s.outstanding(line), 6.0);
        assert!(s.paid(first));
    }

    #[test]
    fn an_outlook_belongs_to_one_party_and_two_of_them_may_disagree() {
        // §46 A3: the disagreement is LOAD-BEARING — it is what gives a market two sides. A store of
        // one number per subject would have deleted it by construction.
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
        // Appendix A: missing is missing. An expectation of zero is an expectation.
        let mut o = Outlooks::new();
        o.form(party(1), 7, 1.20, 3);
        assert_eq!(o.of(party(2), 7), None);
        assert_eq!(o.of(party(1), 8), None);
        assert_eq!(o.formed(party(1), 7), Some(3));
    }

    #[test]
    fn re_forming_an_outlook_replaces_it_rather_than_keeping_two() {
        // Law 4: one writer, one fact. Two outlooks for one party about one thing is two answers.
        let mut o = Outlooks::new();
        o.form(party(1), 7, 1.20, 3);
        o.form(party(1), 7, 1.05, 4);
        assert_eq!(o.len(), 1);
        assert_eq!(o.of(party(1), 7), Some(1.05));
        assert_eq!(o.formed(party(1), 7), Some(4));
    }

    #[test]
    fn a_process_is_in_flight_until_somebody_finishes_it() {
        // XI-3: nothing is immortal, a process included. `running` is what a mechanism asks.
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
        // XI-8, XI-1: a claimant is a named holder and the loss is what was owed less what arrived.
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
        // Register A3: a waterfall needs the claims on ITS estate, and that is the only walk.
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
        // Law 5: both sides, and they are two. An estate owing itself would pay itself first.
        Claims::new().against(PartyId::at(3), PartyId::at(3), 100.0, 1);
    }

    #[test]
    fn a_posting_is_held_by_somebody_and_that_is_what_lets_it_be_withdrawn() {
        // XI-10, 21f.1: a vacancy that stops existing without anybody withdrawing it is the silent
        // disappearance Law 5 is against.
        let mut s = Standing::new();
        let employer = party(1);
        let p = s.stands(7, employer, &[900.0, 30.0], 4);
        assert!(s.live(p));
        assert_eq!(s.held_by(p), employer);
        assert_eq!(s.terms(p), &[900.0, 30.0]);
        assert_eq!(s.of_party(employer).len(), 1);
        assert_eq!(s.of_kind(7).len(), 1);

        s.withdraw(p);
        assert!(!s.live(p), "it is withdrawn BY the party that held it");
        // And the row stays, because a vacancy that existed is a fact somebody may read (Law 19).
        assert_eq!(s.of_party(employer).len(), 1);
    }

    #[test]
    fn a_standard_that_tightens_leaves_the_one_it_replaced_readable() {
        // Housing C5, 21f.2: a standard is a DECISION that tightens, and a tightening is only
        // visible against what it was. Overwriting the terms would delete the comparison.
        let mut s = Standing::new();
        let lender = party(2);
        let was = s.stands(9, lender, &[4.0, 0.10], 1);
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
        Standing::new().stands(7, PartyId::NONE, &[1.0], 0);
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
    #[should_panic(expected = "ready in the period it started")]
    fn a_batch_that_finishes_where_it_started_was_never_in_progress() {
        // Which is exactly what production did before the recipe had a lead time.
        InProgress::new().starts(party(5), InstrumentId::at(9), 10.0, 100.0, 3, 3);
    }
}
