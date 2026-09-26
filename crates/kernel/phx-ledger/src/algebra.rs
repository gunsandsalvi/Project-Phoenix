use phx_core::calendar::Calendar;
use phx_core::calendar::daycount::{DayCount, actual_days, day_fraction};
use phx_core::calendar::period::{Period, ScheduleDates, advance};
use phx_core::consts::MONTHS_PER_YEAR;
use phx_id::{Date, Day, InstrumentId, PartyId, SeriesId, ZoneId};
use phx_macros::clause;
use phx_num::{Ccy, DayFraction, Missing, Money, Qty, Rate, RatePeriod, Round, accrue, capacity_exceeded};

use crate::consts::DUES_PER_DAY;

/// Which side of a two-sided contract: the one holding it as an asset, or the one owing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub enum Side {
    Asset,
    Liability,
}

/// Where a floating rate comes from: a series some market prints, the spread over it, how often it resets, and the
/// current fixing, which stands until the series next fixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Floating {
    pub series: SeriesId,
    pub spread: Rate,
    pub reset: Period,
    pub fixing: Fixing,
}

/// A reference's value on the day it was fixed or published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Fixing {
    pub rate: Rate,
    pub on: Day,
}

/// A rate a leg pays on its notional: fixed at origination, or floating over a printed series.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Reference {
    Fixed(Rate),
    Floating(Floating),
}

/// How the principal is paid back: all at the last date, or in equal parts on every date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Repayment {
    Bullet,
    Linear,
}

/// A published event a contingent leg waits on: its kind, and the party it must concern if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct EventRef {
    pub kind: u16,
    pub party: Missing<PartyId>,
}

/// What a contingent leg pays when its event happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum ContingentAmount {
    Fixed(Money),
    /// A named valuer's valuation of what was lost, bound by the limit, less the deductible.
    ValuedLoss {
        valuer: u16,
        limit: Money,
        deductible: Money,
    },
    /// A benefit per scheduled date while a declared state lasts, after a waiting period from its start.
    WhileState {
        benefit: Money,
        waiting: Period,
        state: u16,
    },
}

/// One generic leg of a contract; a contract's terms are a composition of these, never code of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Leg {
    /// A fixed sum on every scheduled date.
    FixedAmount(Money),
    /// The principal, paid back on the schedule.
    Principal { amount: Money, repayment: Repayment },
    /// Interest on the outstanding principal at a rate over each period.
    RateOnNotional { reference: Reference, day_count: DayCount },
    /// A rate that steps up (or down) on the days given, each in force from its day, over the outstanding principal.
    StepSchedule { steps: Vec<(Day, Rate)>, day_count: DayCount },
    /// Interest paid as more units of an instrument, not money.
    PayableInKind { rate: Rate, day_count: DayCount, instrument: InstrumentId },
    /// A sum per stated period of time, paid on each date for the time since the last.
    PerTime { amount: Money, per: RatePeriod },
    /// Another leg's amount scaled by an index's current fixing over its base, both in the index's points.
    Indexed { series: SeriesId, base: i64, current: i64, leg: Box<Leg> },
    /// A sum paid on the day a published event happens.
    Contingent { event: EventRef, amount: ContingentAmount },
    /// Units delivered on each scheduled date.
    Delivery(Qty),
    /// Legs that pay only when the named side elects on a date of its own schedule; nothing is exercised by default.
    Elective { side: Side, schedule: Schedule, legs: Vec<Leg> },
}

/// The dates a contract's legs fall on: the anchor's schedule, from its first date, for a count of dates or for ever.
/// A leg saved under its variant's name, since one of them holds a leg of its own.
impl phx_store::Saved for Leg {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        match self {
            Leg::FixedAmount(m) => {
                "fixed amount".to_owned().save(w);
                m.save(w);
            }
            Leg::Principal { amount, repayment } => {
                "principal".to_owned().save(w);
                (*amount, *repayment).save(w);
            }
            Leg::RateOnNotional { reference, day_count } => {
                "rate on notional".to_owned().save(w);
                (*reference, *day_count).save(w);
            }
            Leg::StepSchedule { steps, day_count } => {
                "step schedule".to_owned().save(w);
                steps.save(w);
                day_count.save(w);
            }
            Leg::PayableInKind { rate, day_count, instrument } => {
                "payable in kind".to_owned().save(w);
                (*rate, *day_count, *instrument).save(w);
            }
            Leg::PerTime { amount, per } => {
                "per time".to_owned().save(w);
                (*amount, *per).save(w);
            }
            Leg::Indexed { series, base, current, leg } => {
                "indexed".to_owned().save(w);
                (*series, *base, *current).save(w);
                leg.as_ref().save(w);
            }
            Leg::Contingent { event, amount } => {
                "contingent".to_owned().save(w);
                (*event, *amount).save(w);
            }
            Leg::Delivery(q) => {
                "delivery".to_owned().save(w);
                q.save(w);
            }
            Leg::Elective { side, schedule, legs } => {
                "elective".to_owned().save(w);
                (*side, *schedule).save(w);
                legs.save(w);
            }
        }
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Leg, phx_store::LoadError> {
        let variant = String::load(r)?;
        Ok(match variant.as_str() {
            "fixed amount" => Leg::FixedAmount(Money::load(r)?),
            "principal" => {
                let (amount, repayment) = <(Money, Repayment)>::load(r)?;
                Leg::Principal { amount, repayment }
            }
            "rate on notional" => {
                let (reference, day_count) = <(Reference, DayCount)>::load(r)?;
                Leg::RateOnNotional { reference, day_count }
            }
            "step schedule" => Leg::StepSchedule { steps: Vec::load(r)?, day_count: DayCount::load(r)? },
            "payable in kind" => {
                let (rate, day_count, instrument) = <(Rate, DayCount, InstrumentId)>::load(r)?;
                Leg::PayableInKind { rate, day_count, instrument }
            }
            "per time" => {
                let (amount, per) = <(Money, RatePeriod)>::load(r)?;
                Leg::PerTime { amount, per }
            }
            "indexed" => {
                let (series, base, current) = <(SeriesId, i64, i64)>::load(r)?;
                Leg::Indexed { series, base, current, leg: Box::new(Leg::load(r)?) }
            }
            "contingent" => {
                let (event, amount) = <(EventRef, ContingentAmount)>::load(r)?;
                Leg::Contingent { event, amount }
            }
            "delivery" => Leg::Delivery(Qty::load(r)?),
            "elective" => {
                let (side, schedule) = <(Side, Schedule)>::load(r)?;
                Leg::Elective { side, schedule, legs: Vec::load(r)? }
            }
            other => return Err(phx_store::LoadError::Invalid(format!("a leg of kind `{other}`"))),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Schedule {
    pub dates: ScheduleDates,
    pub count: Missing<u32>,
}

/// Where a claim ranks when its issuer fails, lower first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct Seniority(pub u8);

/// What secures a contract: the kind of thing, and where and what class it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Collateral {
    pub kind: u16,
    pub zone: Missing<ZoneId>,
    pub class: u16,
}

/// The rank of a contract's dues among its payer's dues on a day, lower paid first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct PaymentOrder(pub u8);

/// How a contract may end before its last date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Termination {
    None,
    /// The issuer may call it on its schedule's dates, at a price per unit of face in parts per million.
    Callable {
        schedule: Schedule,
        price_ppm: u32,
    },
    /// The holder may put it back on its schedule's dates, at a price per unit of face in parts per million.
    Putable {
        schedule: Schedule,
        price_ppm: u32,
    },
    /// The issuer may repay it at the present value of its remaining dues, discounted at the reference plus a spread.
    MakeWhole {
        series: SeriesId,
        spread: Rate,
    },
}

/// A conversion or write-down term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Conversion {
    /// Into shares of an instrument, at a ratio of shares per unit of face in parts per million.
    IntoShares { instrument: InstrumentId, ratio_ppm: u64 },
    /// Written down, or converted, when a named ratio of its issuer crosses a level.
    Trigger { ratio: u16, level_ppm: i64, write_down_ppm: u32 },
}

/// When a holder can see a default: payments missed beyond a grace period.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct DefaultDefinition {
    pub missed_payments: u16,
    pub grace_days: u16,
}

/// What a contract's legs are over: a printed series, or a published event concerning a party.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Underlying {
    Series(SeriesId),
    Event(EventRef),
}

/// A facility the issuer of a deposit agreed in advance: the balance may fall below nothing by up to `limit` for each
/// member of the row, and the negative balance is the loan, carrying its rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Facility {
    pub limit: Money,
    pub rate: Rate,
    pub day_count: DayCount,
}

/// A contract's terms: its legs placed on its schedule, and everything that says how it ranks, is secured, ends and
/// defaults. Two contracts with equal terms are identical.
#[clause("REG.5", "REG.8")]
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Terms {
    pub ccy: Ccy,
    pub legs: Vec<Leg>,
    pub schedule: Schedule,
    pub seniority: Seniority,
    pub collateral: Missing<Collateral>,
    pub payment_order: PaymentOrder,
    pub termination: Termination,
    pub conversion: Missing<Conversion>,
    pub default: DefaultDefinition,
    pub underlying: Missing<Underlying>,
    pub facility: Missing<Facility>,
    /// The insolvency procedure whose stay suspends the dues of the rows on lines of these terms while it is open.
    pub stay: Missing<u16>,
    /// What the contract fixes beyond its dues, by the places its line kind declares: an employment's occupation
    /// family, skill, hours, notice, severance, region and start band. Contracts differing in any are different.
    pub class: Vec<u32>,
}

impl Terms {
    /// The terms of an account or a claim with no dues of its own: no legs, and a schedule with no dates.
    #[must_use]
    pub fn account(ccy: Ccy, dates: ScheduleDates) -> Terms {
        Terms {
            ccy,
            legs: Vec::new(),
            schedule: Schedule { dates, count: Missing::Present(0) },
            seniority: Seniority(0),
            collateral: Missing::Absent,
            payment_order: PaymentOrder(0),
            termination: Termination::None,
            conversion: Missing::Absent,
            default: DefaultDefinition { missed_payments: 1, grace_days: 0 },
            underlying: Missing::Absent,
            facility: Missing::Absent,
            stay: Missing::Absent,
            class: Vec::new(),
        }
    }
}

/// What falls due: money, units of a unit, or units of an instrument paid in kind, or a loss for a named valuer to
/// value within its limit, less its deductible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Money(Money),
    Units(Qty),
    InKind { instrument: InstrumentId, units: i64 },
    Valued { valuer: u16, limit: Money, deductible: Money },
}

/// One due: which leg it comes from, in the terms' order, and its amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Due {
    pub leg: u16,
    pub amount: Amount,
}

/// The dues of a day, written into the caller's buffer, which never grows.
#[derive(Clone, Copy, Debug)]
pub struct DueBuf {
    dues: [Option<Due>; DUES_PER_DAY],
    len: usize,
}

impl Default for DueBuf {
    fn default() -> Self {
        DueBuf { dues: [None; DUES_PER_DAY], len: 0 }
    }
}

impl DueBuf {
    pub fn clear(&mut self) {
        self.len = 0;
    }

    fn push(&mut self, due: Due) {
        let Some(slot) = self.dues.get_mut(self.len) else {
            capacity_exceeded!("dues of one contract on one day", DUES_PER_DAY, self.len + 1);
        };
        *slot = Some(due);
        self.len += 1;
    }

    pub fn iter(&self) -> impl Iterator<Item = Due> + '_ {
        self.dues.iter().take(self.len).flatten().copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// What a due computation reads beyond the terms: the calendar, the principal still owed, and the decisions and
/// events it waits on.
pub struct DueState<'a> {
    pub calendar: &'a Calendar,
    pub outstanding: Money,
    /// Whether a side elected on a day, read from that side's decision record.
    pub elected: &'a dyn Fn(Side, Day) -> bool,
    /// Whether a published event happened on a day.
    pub occurred: &'a dyn Fn(EventRef, Day) -> bool,
    /// The day a declared state began and still lasts, if it does.
    pub in_state_since: &'a dyn Fn(u16, Day) -> Missing<Day>,
}

impl std::fmt::Debug for DueState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DueState").field("outstanding", &self.outstanding).finish_non_exhaustive()
    }
}

impl Schedule {
    /// Which date of the schedule a day is, counted from one; none if it is not one of its dates.
    #[must_use]
    pub fn date_index(&self, calendar: &Calendar, day: Day) -> Option<u32> {
        let anchor = calendar.day(self.dates.anchor)?;
        let elapsed = i64::from(day.get()) - i64::from(anchor.get());
        if elapsed <= 0 {
            return None;
        }
        // The dates are the anchor advanced by whole periods and moved to a business day, so the guess from the
        // elapsed time is within one date of the right one.
        let per = self.dates.period;
        let guess = if per.month_count() > 0 {
            let months = months_between(self.dates.anchor, calendar.date(day));
            months / i64::from(per.month_count())
        } else {
            elapsed / i64::from(per.day_count())
        };
        let last = match self.count {
            Missing::Present(n) => Some(i64::from(n)),
            Missing::Absent => None,
        };
        (guess - 1..=guess + 1).find_map(|k| {
            let k = u32::try_from(k).ok().filter(|k| *k >= 1)?;
            let within = last.is_none_or(|l| i64::from(k) <= l);
            (within && self.dates.nth(calendar, k) == day).then_some(k)
        })
    }

    /// The day of the schedule's `k`-th date, the anchor's being the zeroth, moved to a business day.
    pub fn day(&self, calendar: &Calendar, k: u32) -> Day {
        self.dates.nth(calendar, k)
    }

    /// The `k`-th date as the schedule states it, before any move to a business day: interest accrues between these,
    /// so a date falling on a holiday moves the payment and never the amount.
    pub fn stated(&self, k: u32) -> Date {
        advance(self.dates.anchor, self.dates.period, k, self.dates.eom)
    }
}

fn months_between(from: Date, to: Date) -> i64 {
    (i64::from(to.year()) - i64::from(from.year())) * i64::from(MONTHS_PER_YEAR) + i64::from(to.month())
        - i64::from(from.month())
}

/// A scheduled date's accrual period: the stated dates it runs between.
#[derive(Clone, Copy, Debug)]
struct Accrual {
    k: u32,
    from: Date,
    to: Date,
}

/// The rate in force on a day from a step schedule: the last step on or before it.
fn stepped(steps: &[(Day, Rate)], day: Day) -> Option<Rate> {
    steps.iter().filter(|(from, _)| *from <= day).map(|(_, r)| *r).next_back()
}

fn add_rates(a: Rate, b: Rate) -> Rate {
    if a.per() != b.per() {
        phx_num::violation!(clause = "TIME.4", "rates of two periods added");
    }
    let Some(raw) = a.raw().checked_add(b.raw()) else {
        phx_num::violation!(clause = "Law 7", "a rate overflows", a = a.raw(), b = b.raw());
    };
    Rate::new(raw, a.per())
}

/// The principal repaid on the `k`-th of `n` dates: equal parts, the remainder on the last, so the parts sum to the
/// principal exactly.
fn linear_part(principal: Money, k: u32, n: u32) -> Money {
    let parts = i64::from(n);
    let each = principal.amt() / parts;
    if k == n {
        Money::new(principal.amt() - each * (parts - 1), principal.ccy())
    } else {
        Money::new(each, principal.ccy())
    }
}

/// Scales a money amount by `current / base`, rounded once.
fn index(amount: Money, current: i64, base: i64) -> Money {
    let scaled = phx_num::div_round(i128::from(amount.amt()) * i128::from(current), i128::from(base), Round::HalfEven);
    let Ok(amt) = i64::try_from(scaled) else {
        phx_num::violation!(clause = "Law 7", "an indexed amount overflows", amt = amount.amt());
    };
    Money::new(amt, amount.ccy())
}

/// A per-time amount over an accrual period, by the time that period spans in the amount's own unit of time.
fn per_time(amount: Money, per: RatePeriod, period: Accrual) -> Money {
    let elapsed = match per {
        RatePeriod::Day => DayFraction::new(actual_days(period.from, period.to), 1, RatePeriod::Day),
        RatePeriod::Month => DayFraction::new(months_between(period.from, period.to), 1, RatePeriod::Month),
        RatePeriod::Year => day_fraction(period.from, period.to, DayCount::Act365F),
    };
    let paid = phx_num::div_round(
        i128::from(amount.amt()) * i128::from(elapsed.num()),
        i128::from(elapsed.den()),
        Round::HalfEven,
    );
    let Ok(amt) = i64::try_from(paid) else {
        phx_num::violation!(clause = "Law 7", "a per-time amount overflows", amt = amount.amt());
    };
    Money::new(amt, amount.ccy())
}

/// A contingent leg's amount on a day, if its event occurred then, or, for a benefit while a state lasts, on a date of
/// its schedule once the waiting period since the state began has passed.
fn contingent(
    event: EventRef,
    amount: &ContingentAmount,
    date: Option<Accrual>,
    day: Day,
    state: &DueState<'_>,
) -> Option<Amount> {
    match amount {
        ContingentAmount::Fixed(m) => (state.occurred)(event, day).then_some(Amount::Money(*m)),
        ContingentAmount::ValuedLoss { valuer, limit, deductible } => (state.occurred)(event, day)
            .then_some(Amount::Valued { valuer: *valuer, limit: *limit, deductible: *deductible }),
        ContingentAmount::WhileState { benefit, waiting, state: s } => {
            date?;
            let Missing::Present(since) = (state.in_state_since)(*s, day) else { return None };
            (state.calendar.plus(since, *waiting) <= day).then_some(Amount::Money(*benefit))
        }
    }
}

/// When a leg is asked for its due: the day, and the schedule's date that falls on it with the period it closes.
#[derive(Clone, Copy, Debug)]
struct When {
    date: Option<Accrual>,
    day: Day,
}

/// A leg's due on a day: on a date of its schedule for the period that date closes, or on a day of its own (an event
/// or an election); written to `out`.
fn leg_due(
    leg: &Leg,
    at: u16,
    terms: &Terms,
    when: When,
    state: &DueState<'_>,
    out: &mut DueBuf,
    scale: Option<(i64, i64)>,
) {
    let When { date, day } = when;
    let money = |m: Money| scale.map_or(m, |(current, base)| index(m, current, base));
    let n = match terms.schedule.count {
        Missing::Present(n) => Some(n),
        Missing::Absent => None,
    };
    match leg {
        Leg::FixedAmount(amount) => {
            if date.is_some() {
                out.push(Due { leg: at, amount: Amount::Money(money(*amount)) });
            }
        }
        Leg::Principal { amount, repayment } => {
            let (Some(Accrual { k, .. }), Some(n)) = (date, n) else { return };
            let part = match repayment {
                Repayment::Bullet => (k == n).then_some(*amount),
                Repayment::Linear => Some(linear_part(*amount, k, n)),
            };
            if let Some(p) = part {
                out.push(Due { leg: at, amount: Amount::Money(money(p)) });
            }
        }
        Leg::RateOnNotional { reference, day_count } => {
            let Some(period) = date else { return };
            let rate = match reference {
                Reference::Fixed(r) => *r,
                Reference::Floating(f) => add_rates(f.fixing.rate, f.spread),
            };
            let f = day_fraction(period.from, period.to, *day_count);
            out.push(Due {
                leg: at,
                amount: Amount::Money(money(accrue(state.outstanding, rate, f, Round::HalfEven))),
            });
        }
        Leg::StepSchedule { steps, day_count } => {
            let Some(period) = date else { return };
            let Some(rate) = state.calendar.day(period.from).and_then(|start| stepped(steps, start)) else {
                phx_num::violation!(
                    clause = "REG.5",
                    "a step schedule with no rate for a period it pays",
                    k = period.k
                );
            };
            let f = day_fraction(period.from, period.to, *day_count);
            out.push(Due {
                leg: at,
                amount: Amount::Money(money(accrue(state.outstanding, rate, f, Round::HalfEven))),
            });
        }
        Leg::PayableInKind { rate, day_count, instrument } => {
            let Some(period) = date else { return };
            let f = day_fraction(period.from, period.to, *day_count);
            let units = money(accrue(state.outstanding, *rate, f, Round::HalfEven)).amt();
            out.push(Due { leg: at, amount: Amount::InKind { instrument: *instrument, units } });
        }
        Leg::PerTime { amount, per } => {
            let Some(period) = date else { return };
            out.push(Due { leg: at, amount: Amount::Money(money(per_time(*amount, *per, period))) });
        }
        Leg::Indexed { base, current, leg: inner, .. } => {
            leg_due(inner, at, terms, when, state, out, Some((*current, *base)));
        }
        Leg::Contingent { event, amount } => {
            if let Some(a) = contingent(*event, amount, date, day, state) {
                let a = match a {
                    Amount::Money(m) => Amount::Money(money(m)),
                    other => other,
                };
                out.push(Due { leg: at, amount: a });
            }
        }
        Leg::Delivery(qty) => {
            if date.is_some() {
                out.push(Due { leg: at, amount: Amount::Units(*qty) });
            }
        }
        Leg::Elective { side, schedule, legs } => {
            let Some(k) = schedule.date_index(state.calendar, day) else { return };
            if !(state.elected)(*side, day) {
                return;
            }
            let own = Some(Accrual { k, from: schedule.stated(k - 1), to: schedule.stated(k) });
            for inner in legs {
                leg_due(inner, at, terms, When { date: own, day }, state, out, scale);
            }
        }
    }
}

/// A leg's due on a date of its schedule as far as the terms and the date fix it: an amount, or a rate and the
/// period's fraction to accrue on what a row owes, scaled where the leg is indexed, paid in kind where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Planned {
    Fixed(Amount),
    Accrue { rate: Rate, fraction: DayFraction, scale: Option<(i64, i64)>, in_kind: Option<InstrumentId> },
}

/// A contract's dues on one day, fixed once for every row of its line: each leg's part the terms and the date decide,
/// in the terms' order, so a row's dues are a few products of its balance. A leg that waits on an event or an
/// election makes the plan `general`, and each row's dues are reckoned whole.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DuePlan {
    legs: Vec<(u16, Planned)>,
    pub general: bool,
}

/// The plan of a contract's dues on a day whose date index in its schedule is known, as `due_at` would reckon them
/// for any balance.
#[clause("REG.5", "REG.11")]
#[must_use]
pub fn due_plan(terms: &Terms, k: Option<u32>, day: Day, calendar: &Calendar) -> DuePlan {
    let schedule = &terms.schedule;
    let date = k.map(|k| Accrual { k, from: schedule.stated(k - 1), to: schedule.stated(k) });
    let mut plan = DuePlan::default();
    for (i, leg) in terms.legs.iter().enumerate() {
        let Ok(at) = u16::try_from(i) else {
            capacity_exceeded!("legs of one contract", u16::MAX, i);
        };
        plan_leg(leg, at, terms, (date, day, calendar), &mut plan, None);
    }
    plan
}

fn plan_leg(
    leg: &Leg,
    at: u16,
    terms: &Terms,
    (date, day, calendar): (Option<Accrual>, Day, &Calendar),
    plan: &mut DuePlan,
    scale: Option<(i64, i64)>,
) {
    let money = |m: Money| scale.map_or(m, |(current, base)| index(m, current, base));
    let n = match terms.schedule.count {
        Missing::Present(n) => Some(n),
        Missing::Absent => None,
    };
    let mut fixed = |a: Amount| plan.legs.push((at, Planned::Fixed(a)));
    match leg {
        Leg::FixedAmount(amount) => {
            if date.is_some() {
                fixed(Amount::Money(money(*amount)));
            }
        }
        Leg::Principal { amount, repayment } => {
            let (Some(Accrual { k, .. }), Some(n)) = (date, n) else { return };
            let part = match repayment {
                Repayment::Bullet => (k == n).then_some(*amount),
                Repayment::Linear => Some(linear_part(*amount, k, n)),
            };
            if let Some(p) = part {
                fixed(Amount::Money(money(p)));
            }
        }
        Leg::RateOnNotional { reference, day_count } => {
            let Some(period) = date else { return };
            let rate = match reference {
                Reference::Fixed(r) => *r,
                Reference::Floating(f) => add_rates(f.fixing.rate, f.spread),
            };
            let fraction = day_fraction(period.from, period.to, *day_count);
            plan.legs.push((at, Planned::Accrue { rate, fraction, scale, in_kind: None }));
        }
        Leg::StepSchedule { steps, day_count } => {
            let Some(period) = date else { return };
            let Some(rate) = calendar.day(period.from).and_then(|start| stepped(steps, start)) else {
                phx_num::violation!(
                    clause = "REG.5",
                    "a step schedule with no rate for a period it pays",
                    k = period.k
                );
            };
            let fraction = day_fraction(period.from, period.to, *day_count);
            plan.legs.push((at, Planned::Accrue { rate, fraction, scale, in_kind: None }));
        }
        Leg::PayableInKind { rate, day_count, instrument } => {
            let Some(period) = date else { return };
            let fraction = day_fraction(period.from, period.to, *day_count);
            plan.legs.push((at, Planned::Accrue { rate: *rate, fraction, scale, in_kind: Some(*instrument) }));
        }
        Leg::PerTime { amount, per } => {
            let Some(period) = date else { return };
            fixed(Amount::Money(money(per_time(*amount, *per, period))));
        }
        Leg::Indexed { base, current, leg: inner, .. } => {
            plan_leg(inner, at, terms, (date, day, calendar), plan, Some((*current, *base)));
        }
        Leg::Delivery(qty) => {
            if date.is_some() {
                fixed(Amount::Units(*qty));
            }
        }
        Leg::Contingent { .. } | Leg::Elective { .. } => plan.general = true,
    }
}

/// The dues a plan comes to on what a row owes, written into `out`, as `due_at` reckons them for a plan that is not
/// `general`. It allocates nothing.
pub fn due_by_plan(plan: &DuePlan, outstanding: Money, out: &mut DueBuf) {
    out.clear();
    for (leg, planned) in &plan.legs {
        let amount = match *planned {
            Planned::Fixed(a) => a,
            Planned::Accrue { rate, fraction, scale, in_kind } => {
                let accrued = accrue(outstanding, rate, fraction, Round::HalfEven);
                let m = scale.map_or(accrued, |(current, base)| index(accrued, current, base));
                match in_kind {
                    Some(instrument) => Amount::InKind { instrument, units: m.amt() },
                    None => Amount::Money(m),
                }
            }
        };
        out.push(Due { leg: *leg, amount });
    }
}

/// The dues of a contract on a day, written into `out`: every leg on a date of the schedule, each contingent leg on
/// its event's day and each elective leg on a date its side elects. It is pure and allocates nothing.
#[clause("REG.5", "REG.11")]
pub fn due_on(terms: &Terms, day: Day, state: &DueState<'_>, out: &mut DueBuf) {
    due_at(terms, terms.schedule.date_index(state.calendar, day), day, state, out);
}

/// The dues of a contract on a day whose date index in its schedule is already known — `None` when the day is none
/// of its dates — so the schedule is never searched for the day.
#[clause("REG.5", "REG.11")]
pub fn due_at(terms: &Terms, k: Option<u32>, day: Day, state: &DueState<'_>, out: &mut DueBuf) {
    out.clear();
    let schedule = &terms.schedule;
    let date = k.map(|k| Accrual { k, from: schedule.stated(k - 1), to: schedule.stated(k) });
    for (i, leg) in terms.legs.iter().enumerate() {
        let Ok(at) = u16::try_from(i) else {
            capacity_exceeded!("legs of one contract", u16::MAX, i);
        };
        leg_due(leg, at, terms, When { date, day }, state, out, None);
    }
}

/// Whether every floating or indexed leg reads a series some market prints, as a floating reference must.
#[must_use]
pub fn unprinted_series(terms: &Terms, printed: &dyn Fn(SeriesId) -> bool) -> Vec<SeriesId> {
    fn walk(leg: &Leg, printed: &dyn Fn(SeriesId) -> bool, out: &mut Vec<SeriesId>) {
        match leg {
            Leg::RateOnNotional { reference: Reference::Floating(f), .. } if !printed(f.series) => out.push(f.series),
            Leg::Indexed { series, leg, .. } => {
                if !printed(*series) {
                    out.push(*series);
                }
                walk(leg, printed, out);
            }
            Leg::Elective { legs, .. } => legs.iter().for_each(|l| walk(l, printed, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    terms.legs.iter().for_each(|l| walk(l, printed, &mut out));
    if let Missing::Present(Underlying::Series(s)) = terms.underlying
        && !printed(s)
    {
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_core::calendar::Calendar;
    use phx_core::calendar::bizday::BusinessDayConvention;
    use phx_core::calendar::daycount::DayCount;
    use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
    use phx_core::calendar::rules::{CountryRules, WeekendRule};
    use phx_id::{CountryId, Date, Day, InstrumentId, SeriesId, Weekday};
    use phx_num::{Ccy, Missing, Money, Rate, RatePeriod};

    use super::{
        Amount, DefaultDefinition, DueBuf, DueState, Fixing, Floating, Leg, PaymentOrder, Reference, Repayment,
        Schedule, Seniority, Side, Termination, Terms, due_on,
    };

    const EUR: Ccy = Ccy::new(0);
    /// Rates are parts in 10^12 a year.
    const PERCENT: i64 = 10_000_000_000;

    fn calendar() -> Calendar {
        let rules =
            CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
        Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2020).unwrap()
    }

    fn schedule(months: u16, count: u32) -> Schedule {
        Schedule {
            dates: ScheduleDates {
                anchor: Date::new(2026, 1, 15).unwrap(),
                period: Period::months(months).unwrap(),
                eom: EndOfMonth::Plain,
                convention: BusinessDayConvention::Following,
                country: CountryId::new(0),
            },
            count: Missing::Present(count),
        }
    }

    fn terms(legs: Vec<Leg>, schedule: Schedule) -> Terms {
        Terms {
            ccy: EUR,
            legs,
            schedule,
            seniority: Seniority(0),
            collateral: Missing::Absent,
            payment_order: PaymentOrder(0),
            termination: Termination::None,
            conversion: Missing::Absent,
            default: DefaultDefinition { missed_payments: 1, grace_days: 30 },
            underlying: Missing::Absent,
            facility: Missing::Absent,
            stay: Missing::Absent,
            class: Vec::new(),
        }
    }

    fn dues(t: &Terms, cal: &Calendar, day: Day, outstanding: i64, elect: bool) -> Vec<Amount> {
        let elected = move |_: Side, _: Day| elect;
        let state = DueState {
            calendar: cal,
            outstanding: Money::new(outstanding, EUR),
            elected: &elected,
            occurred: &|_, _| false,
            in_state_since: &|_, _| Missing::Absent,
        };
        let mut out = DueBuf::default();
        due_on(t, day, &state, &mut out);
        out.iter().map(|d| d.amount).collect()
    }

    fn rate(percent: i64) -> Rate {
        Rate::new(percent * PERCENT, RatePeriod::Year)
    }

    #[test]
    fn due_on_fixed_coupon_bond() {
        let cal = calendar();
        let s = schedule(12, 3);
        let bond = terms(
            vec![
                Leg::RateOnNotional { reference: Reference::Fixed(rate(5)), day_count: DayCount::Thirty360Bond },
                Leg::Principal { amount: Money::new(1_000_000, EUR), repayment: Repayment::Bullet },
            ],
            s,
        );
        let first = s.day(&cal, 1);
        assert_eq!(dues(&bond, &cal, first, 1_000_000, false), vec![Amount::Money(Money::new(50_000, EUR))]);
        let last = s.day(&cal, 3);
        assert_eq!(
            dues(&bond, &cal, last, 1_000_000, false),
            vec![Amount::Money(Money::new(50_000, EUR)), Amount::Money(Money::new(1_000_000, EUR))],
            "the last coupon and the principal"
        );
        assert!(dues(&bond, &cal, first.succ(), 1_000_000, false).is_empty(), "nothing between dates");
    }

    #[test]
    fn due_on_floating_uses_fixing_day() {
        let cal = calendar();
        let s = schedule(6, 4);
        let fixing = Fixing { rate: rate(3), on: s.day(&cal, 0) };
        let note = terms(
            vec![Leg::RateOnNotional {
                reference: Reference::Floating(Floating {
                    series: SeriesId::new(1),
                    spread: rate(1),
                    reset: Period::months(6).unwrap(),
                    fixing,
                }),
                day_count: DayCount::Thirty360Bond,
            }],
            s,
        );
        assert_eq!(
            dues(&note, &cal, s.day(&cal, 1), 1_000_000, false),
            vec![Amount::Money(Money::new(20_000, EUR))],
            "the fixing standing on its day, plus the spread, for half a year"
        );
    }

    #[test]
    fn step_up_and_pik() {
        let cal = calendar();
        let s = schedule(12, 2);
        let step_day = s.day(&cal, 1);
        let t = terms(
            vec![
                Leg::StepSchedule {
                    steps: vec![(s.day(&cal, 0), rate(2)), (step_day, rate(4))],
                    day_count: DayCount::Thirty360Bond,
                },
                Leg::PayableInKind {
                    rate: rate(1),
                    day_count: DayCount::Thirty360Bond,
                    instrument: InstrumentId::new(7),
                },
            ],
            s,
        );
        assert_eq!(
            dues(&t, &cal, s.day(&cal, 1), 1_000_000, false),
            vec![
                Amount::Money(Money::new(20_000, EUR)),
                Amount::InKind { instrument: InstrumentId::new(7), units: 10_000 }
            ]
        );
        assert_eq!(
            dues(&t, &cal, s.day(&cal, 2), 1_000_000, false).first(),
            Some(&Amount::Money(Money::new(40_000, EUR))),
            "the step in force from the period's start"
        );
    }

    #[test]
    fn a_due_plan_reckons_what_due_at_does() {
        use super::{Due, due_at, due_by_plan, due_plan};
        let cal = calendar();
        let s = schedule(3, 8);
        let t = terms(
            vec![
                Leg::RateOnNotional { reference: Reference::Fixed(rate(5)), day_count: DayCount::Thirty360Bond },
                Leg::StepSchedule {
                    steps: vec![(s.day(&cal, 0), rate(2)), (s.day(&cal, 4), rate(4))],
                    day_count: DayCount::Act365F,
                },
                Leg::PayableInKind { rate: rate(1), day_count: DayCount::Act365F, instrument: InstrumentId::new(7) },
                Leg::Principal { amount: Money::new(1_000_003, EUR), repayment: Repayment::Linear },
                Leg::FixedAmount(Money::new(250, EUR)),
                Leg::PerTime { amount: Money::new(12_000, EUR), per: RatePeriod::Year },
                Leg::Indexed {
                    series: SeriesId::new(1),
                    base: 100,
                    current: 107,
                    leg: Box::new(Leg::RateOnNotional {
                        reference: Reference::Fixed(rate(3)),
                        day_count: DayCount::Thirty360Bond,
                    }),
                },
            ],
            s,
        );
        for k in 1..=8 {
            let day = s.day(&cal, k);
            let plan = due_plan(&t, Some(k), day, &cal);
            assert!(!plan.general);
            for owed in [0, 1, 999_999, 1_000_003, 73_512_918] {
                let state = DueState {
                    calendar: &cal,
                    outstanding: Money::new(owed, EUR),
                    elected: &|_, _| false,
                    occurred: &|_, _| false,
                    in_state_since: &|_, _| Missing::Absent,
                };
                let (mut whole, mut planned) = (DueBuf::default(), DueBuf::default());
                due_at(&t, Some(k), day, &state, &mut whole);
                due_by_plan(&plan, Money::new(owed, EUR), &mut planned);
                let (a, b): (Vec<Due>, Vec<Due>) = (whole.iter().collect(), planned.iter().collect());
                assert_eq!(a, b, "date {k}, owed {owed}");
            }
        }
    }

    #[test]
    fn amortising_schedule_sums_to_principal() {
        let cal = calendar();
        let s = schedule(1, 7);
        let loan = terms(vec![Leg::Principal { amount: Money::new(1_000_003, EUR), repayment: Repayment::Linear }], s);
        let total: i64 = (1..=7)
            .flat_map(|k| dues(&loan, &cal, s.day(&cal, k), 0, false))
            .map(|a| match a {
                Amount::Money(m) => m.amt(),
                _ => 0,
            })
            .sum();
        assert_eq!(total, 1_000_003, "the parts sum to the principal exactly");
    }

    #[test]
    fn elective_leg_pays_only_on_election() {
        let cal = calendar();
        let s = schedule(12, 5);
        let convertible = terms(
            vec![Leg::Elective {
                side: Side::Asset,
                schedule: schedule(12, 5),
                legs: vec![Leg::FixedAmount(Money::new(900, EUR))],
            }],
            s,
        );
        assert!(dues(&convertible, &cal, s.day(&cal, 2), 0, false).is_empty(), "nothing unless elected");
        assert_eq!(dues(&convertible, &cal, s.day(&cal, 2), 0, true), vec![Amount::Money(Money::new(900, EUR))]);
    }
}
