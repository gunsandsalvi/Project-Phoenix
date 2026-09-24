use std::collections::BTreeMap;

use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_exec::mix64;
use phx_id::{Date, Day};
use phx_macros::clause;
use phx_num::{Missing, Money, Qty, Rate, RatePeriod, capacity_exceeded, violation};

use crate::algebra::{
    Collateral, ContingentAmount, Conversion, EventRef, Leg, Reference, Repayment, Schedule, Side, Termination, Terms,
    Underlying,
};
use crate::consts::TERMS_SHARDS;

/// An interned set of terms: every contract with these terms reads them here, and none keeps a copy.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TermsId(u32);

impl TermsId {
    pub const fn new(raw: u32) -> TermsId {
        TermsId(raw)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Terms' canonical words: each part tagged and each list preceded by its length, so equal terms give equal words and
/// different terms never the same.
struct Words(Vec<u64>);

impl Words {
    fn u(&mut self, v: u64) {
        self.0.push(v);
    }

    fn i(&mut self, v: i64) {
        self.0.push(v.cast_unsigned());
    }

    /// A part's tag: its name's length, then its bytes eight to a word.
    fn tag(&mut self, name: &str) {
        self.len(name.len());
        for chunk in name.as_bytes().chunks(size_of::<u64>()) {
            let mut word = [0_u8; size_of::<u64>()];
            word.iter_mut().zip(chunk).for_each(|(w, b)| *w = *b);
            self.u(u64::from_le_bytes(word));
        }
    }

    fn money(&mut self, m: Money) {
        self.u(u64::from(m.ccy().index()));
        self.i(m.amt());
    }

    fn rate(&mut self, r: Rate) {
        self.i(r.raw());
        self.tag(match r.per() {
            RatePeriod::Year => "year",
            RatePeriod::Month => "month",
            RatePeriod::Day => "day",
        });
    }

    fn day(&mut self, d: Day) {
        self.u(u64::from(d.get()));
    }

    fn date(&mut self, d: Date) {
        self.i(i64::from(d.year()));
        self.u(u64::from(d.month()));
        self.u(u64::from(d.day()));
    }

    fn period(&mut self, p: Period) {
        self.u(u64::from(p.month_count()));
        self.u(u64::from(p.day_count()));
    }

    fn qty(&mut self, q: Qty) {
        self.u(u64::from(q.unit().index()));
        self.i(q.n());
    }

    fn missing<T: Copy>(&mut self, m: Missing<T>, f: impl FnOnce(&mut Self, T)) {
        match m {
            Missing::Absent => self.u(0),
            Missing::Present(v) => {
                self.u(1);
                f(self, v);
            }
        }
    }

    fn day_count(&mut self, d: DayCount) {
        self.tag(match d {
            DayCount::Act360 => "act/360",
            DayCount::Act365F => "act/365f",
            DayCount::ActActIsda => "act/act isda",
            DayCount::Thirty360Bond => "30/360 bond",
            DayCount::Thirty360E => "30e/360",
        });
    }

    fn dates(&mut self, s: ScheduleDates) {
        self.date(s.anchor);
        self.period(s.period);
        self.tag(match s.eom {
            EndOfMonth::Plain => "plain",
            EndOfMonth::Keep => "keep",
        });
        self.tag(match s.convention {
            BusinessDayConvention::Following => "following",
            BusinessDayConvention::ModifiedFollowing => "modified following",
            BusinessDayConvention::Preceding => "preceding",
            BusinessDayConvention::ModifiedPreceding => "modified preceding",
            BusinessDayConvention::Unadjusted => "unadjusted",
        });
        self.u(u64::from(s.country.get()));
    }

    fn schedule(&mut self, s: Schedule) {
        self.dates(s.dates);
        self.missing(s.count, |w, n| w.u(u64::from(n)));
    }

    fn event(&mut self, e: EventRef) {
        self.u(u64::from(e.kind));
        self.missing(e.party, |w, p| w.u(p.get()));
    }

    fn len(&mut self, n: usize) {
        let Ok(n) = u64::try_from(n) else {
            capacity_exceeded!("a list in a contract's terms", u64::MAX, n);
        };
        self.u(n);
    }

    fn leg(&mut self, leg: &Leg) {
        match leg {
            Leg::FixedAmount(m) => {
                self.tag("fixed amount");
                self.money(*m);
            }
            Leg::Principal { amount, repayment } => {
                self.tag("principal");
                self.money(*amount);
                self.tag(match repayment {
                    Repayment::Bullet => "bullet",
                    Repayment::Linear => "linear",
                });
            }
            Leg::RateOnNotional { reference, day_count } => {
                self.tag("rate on notional");
                match reference {
                    Reference::Fixed(r) => {
                        self.tag("fixed");
                        self.rate(*r);
                    }
                    Reference::Floating(f) => {
                        self.tag("floating");
                        self.u(u64::from(f.series.get()));
                        self.rate(f.spread);
                        self.period(f.reset);
                        self.rate(f.fixing.rate);
                        self.day(f.fixing.on);
                    }
                }
                self.day_count(*day_count);
            }
            Leg::StepSchedule { steps, day_count } => {
                self.tag("step schedule");
                self.len(steps.len());
                for (d, r) in steps {
                    self.day(*d);
                    self.rate(*r);
                }
                self.day_count(*day_count);
            }
            Leg::PayableInKind { rate, day_count, instrument } => {
                self.tag("payable in kind");
                self.rate(*rate);
                self.day_count(*day_count);
                self.u(u64::from(instrument.get()));
            }
            Leg::PerTime { amount, per } => {
                self.tag("per time");
                self.money(*amount);
                self.rate(Rate::new(0, *per));
            }
            Leg::Indexed { series, base, current, leg } => {
                self.tag("indexed");
                self.u(u64::from(series.get()));
                self.i(*base);
                self.i(*current);
                self.leg(leg);
            }
            Leg::Contingent { event, amount } => {
                self.tag("contingent");
                self.event(*event);
                match amount {
                    ContingentAmount::Fixed(m) => {
                        self.tag("fixed");
                        self.money(*m);
                    }
                    ContingentAmount::ValuedLoss { valuer, limit, deductible } => {
                        self.tag("valued loss");
                        self.u(u64::from(*valuer));
                        self.money(*limit);
                        self.money(*deductible);
                    }
                    ContingentAmount::WhileState { benefit, waiting, state } => {
                        self.tag("while state");
                        self.money(*benefit);
                        self.period(*waiting);
                        self.u(u64::from(*state));
                    }
                }
            }
            Leg::Delivery(q) => {
                self.tag("delivery");
                self.qty(*q);
            }
            Leg::Elective { side, schedule, legs } => {
                self.tag("elective");
                self.tag(match side {
                    Side::Asset => "asset",
                    Side::Liability => "liability",
                });
                self.schedule(*schedule);
                self.len(legs.len());
                for l in legs {
                    self.leg(l);
                }
            }
        }
    }

    fn terms(&mut self, t: &Terms) {
        self.u(u64::from(t.ccy.index()));
        self.len(t.legs.len());
        for l in &t.legs {
            self.leg(l);
        }
        self.schedule(t.schedule);
        self.u(u64::from(t.seniority.0));
        self.missing(t.collateral, |w, c: Collateral| {
            w.u(u64::from(c.kind));
            w.missing(c.zone, |w, z| w.u(u64::from(z.get())));
            w.u(u64::from(c.class));
        });
        self.u(u64::from(t.payment_order.0));
        match t.termination {
            Termination::None => self.tag("none"),
            Termination::Callable { schedule, price_ppm } => {
                self.tag("callable");
                self.schedule(schedule);
                self.u(u64::from(price_ppm));
            }
            Termination::Putable { schedule, price_ppm } => {
                self.tag("putable");
                self.schedule(schedule);
                self.u(u64::from(price_ppm));
            }
            Termination::MakeWhole { series, spread } => {
                self.tag("make whole");
                self.u(u64::from(series.get()));
                self.rate(spread);
            }
        }
        self.missing(t.conversion, |w, c| match c {
            Conversion::IntoShares { instrument, ratio_ppm } => {
                w.tag("into shares");
                w.u(u64::from(instrument.get()));
                w.u(ratio_ppm);
            }
            Conversion::Trigger { ratio, level_ppm, write_down_ppm } => {
                w.tag("trigger");
                w.u(u64::from(ratio));
                w.i(level_ppm);
                w.u(u64::from(write_down_ppm));
            }
        });
        self.u(u64::from(t.default.missed_payments));
        self.u(u64::from(t.default.grace_days));
        self.missing(t.underlying, |w, u| match u {
            Underlying::Series(s) => {
                w.tag("series");
                w.u(u64::from(s.get()));
            }
            Underlying::Event(e) => {
                w.tag("event");
                w.event(e);
            }
        });
        self.missing(t.facility, |w, f| {
            w.money(f.limit);
            w.rate(f.rate);
            w.day_count(f.day_count);
        });
        self.missing(t.stay, |w, procedure| w.u(u64::from(procedure)));
    }
}

/// The canonical words of a set of terms, which the interner keys by and saves and hashes read.
#[must_use]
pub fn words(terms: &Terms) -> Vec<u64> {
    let mut w = Words(Vec::new());
    w.terms(terms);
    w.0
}

/// One interned set of terms and how many holders of it there are.
#[derive(Clone, Debug, phx_macros::Saved)]
struct Entry {
    terms: Terms,
    words: Vec<u64>,
    refs: u32,
}

/// One shard: its terms by their words, its entries by local identity, and the identities freed for reuse.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
struct Shard {
    #[saved(skip)]
    index: BTreeMap<Vec<u64>, u32>,
    entries: Vec<Option<Entry>>,
    free: Vec<u32>,
}

/// Every contract's terms, each held once and counted by its holders, sharded by the terms' words so that parallel
/// interning never shares a shard, and each identity is the same whatever order the shards are reached in.
#[clause("REP.3", "REG.14")]
#[derive(Clone, Debug)]
pub struct TermsInterner {
    shards: Vec<Shard>,
}

/// The interner saved with its entries, reference counts and free identities, and its index of terms by their words
/// rebuilt on load; each entry's words are checked against its terms.
impl phx_store::Saved for TermsInterner {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        self.shards.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<TermsInterner, phx_store::LoadError> {
        let mut shards: Vec<Shard> = phx_store::Saved::load(r)?;
        for shard in &mut shards {
            for (local, entry) in shard.entries.iter().enumerate() {
                let Some(e) = entry else { continue };
                if e.words != words(&e.terms) || e.refs == 0 {
                    return Err(phx_store::LoadError::Invalid("terms whose words or holders are wrong".to_owned()));
                }
                let local = phx_store::narrow::<u32>(local, "a terms identity")?;
                if shard.index.insert(e.words.clone(), local).is_some() {
                    return Err(phx_store::LoadError::Invalid("one set of terms interned twice".to_owned()));
                }
            }
        }
        if shards.len() != at(TERMS_SHARDS) {
            return Err(phx_store::LoadError::Invalid("an interner of another shard count".to_owned()));
        }
        Ok(TermsInterner { shards })
    }
}

impl Default for TermsInterner {
    fn default() -> Self {
        TermsInterner { shards: (0..TERMS_SHARDS).map(|_| Shard::default()).collect() }
    }
}

fn at(n: u32) -> usize {
    let Ok(i) = usize::try_from(n) else {
        violation!(clause = "REP.3", "a terms index beyond the machine's words", n = n);
    };
    i
}

fn shard_of(words: &[u64]) -> u32 {
    let mixed = words.iter().fold(0_u64, |h, w| mix64(h ^ w));
    let Ok(s) = u32::try_from(mixed % u64::from(TERMS_SHARDS)) else {
        violation!(clause = "REP.3", "a terms shard beyond the interner's");
    };
    s
}

fn split(id: TermsId) -> (usize, usize) {
    (at(id.0 % TERMS_SHARDS), at(id.0 / TERMS_SHARDS))
}

impl TermsInterner {
    /// The identity of these terms, counting one more holder; equal terms share one identity.
    pub fn intern(&mut self, terms: Terms) -> TermsId {
        let w = words(&terms);
        let s = shard_of(&w);
        let Some(shard) = self.shards.get_mut(at(s)) else {
            violation!(clause = "REP.3", "a terms shard beyond the interner's", shard = s);
        };
        let local = if let Some(local) = shard.index.get(&w) {
            *local
        } else {
            let local = if let Some(l) = shard.free.pop() {
                l
            } else {
                let Ok(l) = u32::try_from(shard.entries.len()) else {
                    capacity_exceeded!("terms in one shard", u32::MAX, shard.entries.len());
                };
                shard.entries.push(None);
                l
            };
            let Some(slot) = shard.entries.get_mut(at(local)) else {
                violation!(clause = "REP.3", "a terms slot beyond its shard", local = local);
            };
            *slot = Some(Entry { terms, words: w.clone(), refs: 0 });
            shard.index.insert(w, local);
            local
        };
        let Some(id) = local.checked_mul(TERMS_SHARDS).and_then(|x| x.checked_add(s)) else {
            capacity_exceeded!("terms identities", u32::MAX, local);
        };
        self.retain(TermsId(id));
        TermsId(id)
    }

    fn entry_mut(&mut self, id: TermsId) -> &mut Entry {
        let (s, local) = split(id);
        let Some(entry) = self.shards.get_mut(s).and_then(|sh| sh.entries.get_mut(local)).and_then(Option::as_mut)
        else {
            violation!(clause = "REP.3", "terms read that were never interned or were released", id = id.0);
        };
        entry
    }

    /// One more holder of these terms.
    pub fn retain(&mut self, id: TermsId) {
        let entry = self.entry_mut(id);
        let Some(refs) = entry.refs.checked_add(1) else {
            capacity_exceeded!("holders of one set of terms", u32::MAX, entry.refs);
        };
        entry.refs = refs;
    }

    /// One holder fewer; the terms go when none holds them.
    pub fn release(&mut self, id: TermsId) {
        let entry = self.entry_mut(id);
        entry.refs -= 1;
        if entry.refs > 0 {
            return;
        }
        let words = core::mem::take(&mut entry.words);
        let (s, local) = (id.0 % TERMS_SHARDS, id.0 / TERMS_SHARDS);
        let Some(shard) = self.shards.get_mut(at(s)) else {
            violation!(clause = "REP.3", "a terms shard beyond the interner's", shard = s);
        };
        shard.index.remove(&words);
        let Some(slot) = shard.entries.get_mut(at(local)) else {
            violation!(clause = "REP.3", "a terms slot beyond its shard", local = local);
        };
        *slot = None;
        shard.free.push(local);
    }

    /// The terms of an identity.
    #[must_use]
    pub fn get(&self, id: TermsId) -> &Terms {
        let (s, local) = split(id);
        match self.shards.get(s).and_then(|sh| sh.entries.get(local)).and_then(Option::as_ref) {
            Some(entry) => &entry.terms,
            None => violation!(clause = "REP.3", "terms read that were never interned or were released", id = id.0),
        }
    }

    /// How many holders the terms have.
    #[must_use]
    pub fn holders(&self, id: TermsId) -> u32 {
        let (s, local) = split(id);
        self.shards.get(s).and_then(|sh| sh.entries.get(local)).and_then(Option::as_ref).map_or(0, |e| e.refs)
    }

    /// How many distinct terms are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shards.iter().map(|s| s.index.len()).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use phx_core::calendar::bizday::BusinessDayConvention;
    use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
    use phx_id::{CountryId, Date};
    use phx_num::{Ccy, Missing, Money};

    use super::{TermsInterner, words};
    use crate::algebra::{DefaultDefinition, Leg, PaymentOrder, Schedule, Seniority, Termination, Terms};

    fn terms(amount: i64) -> Terms {
        Terms {
            ccy: Ccy::new(0),
            legs: vec![Leg::FixedAmount(Money::new(amount, Ccy::new(0)))],
            schedule: Schedule {
                dates: ScheduleDates {
                    anchor: Date::new(2026, 1, 1).unwrap(),
                    period: Period::months(1).unwrap(),
                    eom: EndOfMonth::Plain,
                    convention: BusinessDayConvention::Following,
                    country: CountryId::new(0),
                },
                count: Missing::Present(12),
            },
            seniority: Seniority(0),
            collateral: Missing::Absent,
            payment_order: PaymentOrder(0),
            termination: Termination::None,
            conversion: Missing::Absent,
            default: DefaultDefinition { missed_payments: 1, grace_days: 30 },
            underlying: Missing::Absent,
            facility: Missing::Absent,
            stay: Missing::Absent,
        }
    }

    #[test]
    fn words_tell_terms_apart() {
        assert_eq!(words(&terms(5)), words(&terms(5)));
        assert_ne!(words(&terms(5)), words(&terms(6)));
    }

    #[test]
    fn terms_interner_refcounts() {
        let mut interner = TermsInterner::default();
        let a = interner.intern(terms(5));
        let b = interner.intern(terms(5));
        let c = interner.intern(terms(6));
        assert_eq!(a, b, "equal terms share one identity");
        assert_ne!(a, c);
        assert_eq!((interner.holders(a), interner.len()), (2, 2));
        interner.release(a);
        assert_eq!(interner.get(b), &terms(5), "still held once");
        interner.release(b);
        assert_eq!((interner.holders(a), interner.len()), (0, 1), "released when none holds them");
        let again = interner.intern(terms(5));
        assert_eq!(interner.get(again), &terms(5));
        assert_eq!(interner.get(c), &terms(6));
    }
}
