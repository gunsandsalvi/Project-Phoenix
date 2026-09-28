//! Terms split in two: a shape, the terms with each amount a contract fixes for itself taken out, which every contract
//! of that kind of terms shares; and each contract's own amounts, kept in its own words. A shape's dues on a date are
//! planned once for all its contracts, so a contract's due is its amounts and balance read and a few products taken.

use phx_core::calendar::Calendar;
use phx_id::Day;
use phx_macros::clause;
use phx_num::{DayFraction, Missing, Money, violation};

use super::{
    Accrual, Amount, Due, DueBuf, DuePlan, Leg, Planned, Repayment, Terms, index, linear_part, per_time_by,
    per_time_fraction, plan_leg, planned_amount,
};

/// How a date takes a contract's own amount: whole, as its part of a principal repaid in equal parts, or over the
/// time the period spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Whole,
    Linear { k: u32, n: u32 },
    PerTime(DayFraction),
}

/// A leg's due on a date: planned as the terms plan it, from the balance or fixed in the shape; or the contract's
/// own amount at a place, taken as the date takes it and scaled where the leg is indexed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Plan(Planned),
    Own { place: u8, part: Part, scale: Option<(i64, i64)> },
}

/// A shape's dues on one date, for every contract of the shape. A leg that waits on an event or an election makes it
/// `general`, and each contract's dues are reckoned whole from its terms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShapePlan {
    legs: Vec<(u16, Step)>,
    pub general: bool,
}

/// A contract's terms as its shape and its own amounts, in the order its legs are walked; equal shapes are one kind
/// of terms, whatever the amounts.
#[clause("REG.5", "REG.8")]
#[must_use]
pub fn shape_of(terms: &Terms) -> (Terms, Vec<Money>) {
    let mut shape = terms.clone();
    let mut own = Vec::new();
    for leg in &mut shape.legs {
        take(leg, &mut own);
    }
    (shape, own)
}

fn take(leg: &mut Leg, own: &mut Vec<Money>) {
    match leg {
        Leg::FixedAmount(m) | Leg::Principal { amount: m, .. } | Leg::PerTime { amount: m, .. } => {
            own.push(*m);
            *m = Money::new(0, m.ccy());
        }
        Leg::Indexed { leg, .. } => take(leg, own),
        _ => {}
    }
}

/// The terms a shape and a contract's own amounts make, as `shape_of` split them.
#[must_use]
pub fn terms_of(shape: &Terms, own: &[Money]) -> Terms {
    let mut terms = shape.clone();
    let mut at = 0;
    for leg in &mut terms.legs {
        give(leg, own, &mut at);
    }
    if at != own.len() {
        violation!(clause = "REG.5", "a contract's amounts that its terms' shape does not read", given = own.len());
    }
    terms
}

fn give(leg: &mut Leg, own: &[Money], at: &mut usize) {
    match leg {
        Leg::FixedAmount(m) | Leg::Principal { amount: m, .. } | Leg::PerTime { amount: m, .. } => {
            let Some(v) = own.get(*at) else {
                violation!(clause = "REG.5", "a contract without the amount its terms' shape reads", place = *at);
            };
            *m = *v;
            *at += 1;
        }
        Leg::Indexed { leg, .. } => give(leg, own, at),
        _ => {}
    }
}

/// The plan of a shape's dues on a day whose date index in its schedule is known, for every contract of the shape.
#[clause("REG.5", "REG.11")]
#[must_use]
pub fn shape_plan(shape: &Terms, k: Option<u32>, day: Day, calendar: &Calendar) -> ShapePlan {
    let schedule = &shape.schedule;
    let date = k.map(|k| Accrual { k, from: schedule.stated(k - 1), to: schedule.stated(k) });
    let mut plan = ShapePlan::default();
    let mut place = 0_u8;
    for (i, leg) in shape.legs.iter().enumerate() {
        let Ok(at) = u16::try_from(i) else {
            phx_num::capacity_exceeded!("legs of one contract", u16::MAX, i);
        };
        plan_shape_leg(leg, at, shape, (date, day, calendar), (&mut plan, &mut place), None);
    }
    plan
}

fn plan_shape_leg(
    leg: &Leg,
    at: u16,
    terms: &Terms,
    when: (Option<Accrual>, Day, &Calendar),
    (plan, place): (&mut ShapePlan, &mut u8),
    scale: Option<(i64, i64)>,
) {
    let (date, _, _) = when;
    let n = match terms.schedule.count {
        Missing::Present(n) => Some(n),
        Missing::Absent => None,
    };
    let mut own = |part: Option<Part>, plan: &mut ShapePlan| {
        if let Some(part) = part {
            plan.legs.push((at, Step::Own { place: *place, part, scale }));
        }
        let Some(next) = place.checked_add(1) else {
            phx_num::capacity_exceeded!("a contract's own amounts", u8::MAX, *place);
        };
        *place = next;
    };
    match leg {
        Leg::FixedAmount(_) => own(date.map(|_| Part::Whole), plan),
        Leg::Principal { repayment, .. } => {
            let part = match (date, n) {
                (Some(Accrual { k, .. }), Some(n)) => match repayment {
                    Repayment::Bullet => (k == n).then_some(Part::Whole),
                    Repayment::Linear => Some(Part::Linear { k, n }),
                },
                _ => None,
            };
            own(part, plan);
        }
        Leg::PerTime { per, .. } => own(date.map(|period| Part::PerTime(per_time_fraction(*per, period))), plan),
        Leg::Indexed { base, current, leg: inner, .. } => {
            plan_shape_leg(inner, at, terms, when, (plan, place), Some((*current, *base)));
        }
        other => {
            let mut sub = DuePlan::default();
            plan_leg(other, at, terms, when, &mut sub, scale);
            plan.general |= sub.general;
            plan.legs.extend(sub.legs.into_iter().map(|(a, p)| (a, Step::Plan(p))));
        }
    }
}

/// A contract's dues by its shape's plan, from its own amounts and what it owes, written into `out`, as `due_at`
/// reckons them for its terms; the plan must not be `general`. It allocates nothing.
#[clause("REG.5", "REG.11")]
pub fn due_by_shape(plan: &ShapePlan, own: &[Money], outstanding: Money, out: &mut DueBuf) {
    out.clear();
    for (leg, step) in &plan.legs {
        let amount = match *step {
            Step::Plan(p) => planned_amount(p, outstanding),
            Step::Own { place, part, scale } => {
                let Some(m) = own.get(usize::from(place)) else {
                    violation!(clause = "REG.5", "a contract without the amount its terms' shape reads", place = place);
                };
                let m = match part {
                    Part::Whole => *m,
                    Part::Linear { k, n } => linear_part(*m, k, n),
                    Part::PerTime(f) => per_time_by(*m, f),
                };
                Amount::Money(scale.map_or(m, |(current, base)| index(m, current, base)))
            }
        };
        out.push(Due { leg: *leg, amount });
    }
}

#[cfg(test)]
mod tests {
    use phx_core::calendar::Calendar;
    use phx_core::calendar::bizday::BusinessDayConvention;
    use phx_core::calendar::daycount::DayCount;
    use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
    use phx_core::calendar::rules::{CountryRules, WeekendRule};
    use phx_id::{CountryId, Date, InstrumentId, SeriesId, Weekday};
    use phx_num::{Ccy, Missing, Money, Qty, Rate, RatePeriod, UnitId};

    use super::super::{
        DefaultDefinition, DueBuf, DueState, Leg, PaymentOrder, Reference, Repayment, Schedule, Seniority, Termination,
        Terms, due_at,
    };
    use super::{due_by_shape, shape_of, shape_plan, terms_of};

    const EUR: Ccy = Ccy::new(0);
    /// A rate of one percent, in parts in 10^12 a year.
    const PERCENT: i64 = 10_000_000_000;

    fn calendar() -> Calendar {
        let rules =
            CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
        Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2020).unwrap()
    }

    fn terms(legs: Vec<Leg>, count: u32) -> Terms {
        let dates = ScheduleDates {
            anchor: Date::new(2026, 1, 31).unwrap(),
            period: Period::months(1).unwrap(),
            eom: EndOfMonth::Plain,
            convention: BusinessDayConvention::Following,
            country: CountryId::new(0),
        };
        Terms {
            ccy: EUR,
            legs,
            schedule: Schedule { dates, count: Missing::Present(count) },
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

    fn eur(a: i64) -> Money {
        Money::new(a, EUR)
    }

    #[test]
    fn compiled_program_matches_algebra() {
        let cal = calendar();
        let rate = Rate::new(3 * PERCENT + 7, RatePeriod::Year);
        let every = [
            vec![Leg::FixedAmount(eur(310_007))],
            vec![Leg::Principal { amount: eur(1_000_003), repayment: Repayment::Bullet }],
            vec![
                Leg::Principal { amount: eur(1_000_003), repayment: Repayment::Linear },
                Leg::RateOnNotional { reference: Reference::Fixed(rate), day_count: DayCount::Act365F },
            ],
            vec![
                Leg::Amortising,
                Leg::RateOnNotional { reference: Reference::Fixed(rate), day_count: DayCount::Act360 },
            ],
            vec![Leg::PerTime { amount: eur(9_001), per: RatePeriod::Day }],
            vec![Leg::PerTime { amount: eur(120_001), per: RatePeriod::Year }],
            vec![Leg::PerTime { amount: eur(4_003), per: RatePeriod::Month }],
            vec![Leg::Indexed {
                series: SeriesId::new(1),
                base: 1_000,
                current: 1_037,
                leg: Box::new(Leg::FixedAmount(eur(50_001))),
            }],
            vec![Leg::Indexed {
                series: SeriesId::new(1),
                base: 997,
                current: 1_013,
                leg: Box::new(Leg::RateOnNotional { reference: Reference::Fixed(rate), day_count: DayCount::Act365F }),
            }],
            vec![Leg::PayableInKind { rate, day_count: DayCount::Act365F, instrument: InstrumentId::new(4) }],
            vec![
                Leg::StepSchedule {
                    steps: vec![(cal.day(Date::new(2026, 1, 1).unwrap()).unwrap(), rate)],
                    day_count: DayCount::Act365F,
                },
                Leg::Delivery(Qty::new(12, UnitId::new(2))),
            ],
        ];
        let count = 7;
        for legs in every {
            let t = terms(legs, count);
            let (shape, own) = shape_of(&t);
            assert_eq!(terms_of(&shape, &own), t, "the shape and the amounts make the terms again");
            let (other, _) = shape_of(&terms(t.legs.iter().map(|l| scaled(l, 3)).collect(), count));
            assert_eq!(other, shape, "contracts differing only in their amounts share one shape");
            for k in 1..=count {
                let day = t.schedule.day(&cal, k);
                let plan = shape_plan(&shape, Some(k), day, &cal);
                assert!(!plan.general);
                for outstanding in [0, 1, 777_777, 12_345_678_901] {
                    let state = DueState {
                        calendar: &cal,
                        outstanding: eur(outstanding),
                        elected: &|_, _| false,
                        occurred: &|_, _| false,
                        in_state_since: &|_, _| Missing::Absent,
                    };
                    let (mut want, mut got) = (DueBuf::default(), DueBuf::default());
                    due_at(&t, Some(k), day, &state, &mut want);
                    due_by_shape(&plan, &own, eur(outstanding), &mut got);
                    assert_eq!(got.iter().collect::<Vec<_>>(), want.iter().collect::<Vec<_>>(), "{t:?} at {k}");
                }
            }
        }
    }

    /// A leg with its own amounts times `m`, its shape the same.
    fn scaled(leg: &Leg, m: i64) -> Leg {
        match leg {
            Leg::FixedAmount(a) => Leg::FixedAmount(eur(a.amt() * m)),
            Leg::Principal { amount, repayment } => {
                Leg::Principal { amount: eur(amount.amt() * m), repayment: *repayment }
            }
            Leg::PerTime { amount, per } => Leg::PerTime { amount: eur(amount.amt() * m), per: *per },
            Leg::Indexed { series, base, current, leg } => {
                Leg::Indexed { series: *series, base: *base, current: *current, leg: Box::new(scaled(leg, m)) }
            }
            other => other.clone(),
        }
    }
}
