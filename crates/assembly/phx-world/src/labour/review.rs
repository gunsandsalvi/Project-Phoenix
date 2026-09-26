//! The employers' reviews of their contracts' wages, a pay round a year at a phase each employer draws: at 5c the
//! employer offers each of its lines a point, and at the next 4a each employee answers, staying at the offer or at
//! its counter the work still pays for, or quitting to search. A new wage is a new line, which its members join.

use if_labour::class;
use if_labour::decisions::{AnswerIn, Need, ReviewIn};
use if_labour::law::Law;
use phx_core::SubStep;
use phx_core::calendar::period::Period;
use phx_id::{CountryId, Day, PartyId};
use phx_ledger::algebra::Side;
use phx_ledger::apply::ApplyAt;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};

use super::book::Review;
use super::post::Staff;
use crate::world::World;

impl World {
    /// Whether an employer's review is due today, its next set a review period on; an employer seen the first time
    /// draws the day in its first period it reviews on, so pay rounds are staggered.
    fn review_due(&mut self, day: Day, employer: PartyId, law: &Law) -> bool {
        let Some(kind) = self.labour.kind else { return false };
        let Some(period) = u16::try_from(law.review_months).ok().and_then(Period::months) else {
            violation!(clause = "LAB.17", "a review period beyond a period", months = law.review_months);
        };
        match self.labour.book.reviews.get(&employer).copied() {
            Some(due) if due <= day => {
                let mut next = self.calendar.plus(due, period);
                while next <= day {
                    next = self.calendar.plus(next, period);
                }
                self.labour.book.reviews.insert(employer, next);
                true
            }
            Some(_) => false,
            None => {
                let end = self.calendar.plus(day, period);
                let Some(days) = self.calendar.days_between(day, end) else { return false };
                let Some(stream) = self.streams.named(kind.review_stream) else {
                    violation!(clause = "CHN.1", "reviews phased by a stream never declared");
                };
                let mut d = self.streams.open(
                    &stream,
                    Subject::new(SubjectTag::Party, employer.get()),
                    day,
                    SubStep::S5c.ordinal(),
                );
                let Some(phase) =
                    u16::try_from(phx_rand::uniform::below_u64(&mut d, u64::from(days))).ok().and_then(Period::days)
                else {
                    violation!(clause = "LAB.17", "a review's phase beyond a period", days = days);
                };
                self.labour.book.reviews.insert(employer, self.calendar.plus(day, phase));
                false
            }
        }
    }

    /// An employer's review, when due: for each of its lines, the point it offers, the lesser of the point nearest
    /// what a month of the job's hours brings in at its price and the point its fills show the market pays, never
    /// below the law's least; applied at the next start of work.
    #[clause("LAB.17", "LAB.9", "LAB.11")]
    pub(super) fn review(
        &mut self,
        day: Day,
        (employer, country, twins): (PartyId, CountryId, u32),
        (law, price): (&Law, f64),
        staff: &[Staff],
        needs: &[Need],
    ) {
        let Some(kind) = self.labour.kind else { return };
        if staff.is_empty() || !self.review_due(day, employer, law) {
            return;
        }
        for s in staff {
            let Some(n) = needs.iter().find(|n| n.occupation == s.occupation && n.hours_a_unit > 0.0) else {
                continue;
            };
            let month = f64::from(s.hours) * law.weeks_a_month;
            let Some(revenue) = (kind.point_near)(law, price / n.hours_a_unit * month) else { continue };
            let share = f64::from(s.hours) / f64::from(law.full_time_hours);
            let least = match law.minimum_monthly {
                Missing::Present(m) => match (kind.least_point)(law, m * share) {
                    Some(p) => Missing::Present(p),
                    None => violation!(clause = "LAB.11", "a minimum wage beyond the wage points"),
                },
                Missing::Absent => Missing::Absent,
            };
            let input = ReviewIn {
                current: i64::from(self.line_point(s.line)),
                revenue,
                market: self.offer_point(law, (employer, s.occupation), staff, s.hours),
                least,
            };
            let offer = (kind.review)(&input);
            self.labour.book.reviewing.push(Review {
                employer,
                country: country.get(),
                line: s.line,
                count: s.members * twins,
                offer,
                most: revenue,
            });
        }
    }

    /// The best monthly wage among the vacancies an employee of a class can see: its region's, of its occupation,
    /// open to its skill.
    fn best_vacancy(&self, law: &Law, class_of: &[u32]) -> Missing<f64> {
        let at = |i| class_of.get(i).copied();
        let (Some(region), Some(occupation), Some(skill)) =
            (at(class::REGION), at(class::OCCUPATION), at(class::SKILL))
        else {
            return Missing::Absent;
        };
        let best = self
            .labour
            .book
            .vacancies
            .iter()
            .filter(|v| v.region == region && v.occupation == occupation && v.skill <= skill && v.open > 0)
            .map(|v| v.point)
            .reduce(|a, b| if b > a { b } else { a });
        match best {
            Some(p) => Missing::Present(self.wage_at(law, p)),
            None => Missing::Absent,
        }
    }

    /// A review applied at 4a: each employee drawn from the employer's side answers, and stays at the point the two
    /// conclude, its members joining that point's line, or quits to search. Nothing moves when the rule keeps a line
    /// as it pays.
    #[clause("LAB.17", "LAB.6", "LAB.9", "REP.23")]
    pub(super) fn apply_review(&mut self, day: Day, r: &Review) {
        let Some(kind) = self.labour.kind else { return };
        if !self.live(r.employer) {
            return;
        }
        let (place, slot) = self.books.parties.row(r.employer);
        let found = phx_ledger::rows::find(self.books.parties.holder(place), slot, r.line, Side::Liability);
        let Some(held) = found.map(|v| v.row.count) else { return };
        let count = if r.count < held { r.count } else { held };
        let current = i64::from(self.line_point(r.line));
        let law = super::law_of(&self.labour.laws, CountryId::new(r.country)).clone();
        let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(r.line)).clone();
        let input = AnswerIn {
            offer: r.offer,
            reservation: law.reservation_share * self.wage_at(&law, current),
            best: self.best_vacancy(&law, &terms.class),
            outlook: law.price_outlook,
            ratio: law.point_ratio,
        };
        let conclude = |answer: Option<i64>| match answer {
            Some(a) => (kind.conclude)(r.offer, a, r.most),
            // An employee who says nothing works on at the offer.
            None => Missing::Present(r.offer),
        };
        let ruled =
            conclude(phx_core::decisions::dispatch(kind.answer, phx_core::decisions::Decider::Rule, None, &input));
        if ruled == Missing::Present(current) {
            return;
        }
        self.labour.day.reviewed += u64::from(count);
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        let Some(stream) = self.streams.named(kind.layoff_stream) else {
            violation!(clause = "CHN.1", "reviews drawn from a stream never declared");
        };
        let mut d = self.streams.open(
            &stream,
            Subject::new(SubjectTag::Line, u64::from(r.line.get())),
            day,
            SubStep::S4a.ordinal(),
        );
        let taken = match self.books.members_leave(
            (r.employer, r.line, Side::Liability),
            count,
            m,
            &mut d,
            self.audit.stream(),
        ) {
            Ok(t) => t,
            Err(f) => violation!(clause = "LAB.17", "a review that did not leave its line", party = f.party.get()),
        };
        let Missing::Present(reason) =
            self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.renegotiated))
        else {
            violation!(clause = "LAB.17", "reviews under a reason never declared");
        };
        for (party, k) in taken {
            let decider = self.queue.decider(party);
            let outcome = match decider {
                phx_core::decisions::Decider::Player { .. } => {
                    let queued = self.queue.take(party, kind.answer.name);
                    conclude(phx_core::decisions::dispatch(kind.answer, decider, queued.as_deref(), &input))
                }
                phx_core::decisions::Decider::Rule => ruled,
            };
            let persons = self.detach(&[(party, r.line, Side::Asset, k)], &mut d);
            match outcome {
                Missing::Present(point) => {
                    let line = self.employment_line(r.country, &terms.class, point);
                    if self
                        .books
                        .members_join((party, line, Side::Asset), r.employer, k, (reason, m), self.audit.stream())
                        .is_err()
                    {
                        violation!(clause = "LAB.17", "a review that did not join its line", party = party.get());
                    }
                    let Some(recorded) = u32::try_from(point).ok().filter(|p| *p < class::WAGE_POINTS) else {
                        phx_num::capacity_exceeded!("a wage point a person can record", class::WAGE_POINTS, point);
                    };
                    for (p, person) in persons {
                        self.attach(p, person, line);
                        self.set_person(p, person, &[(kind.last_point, recorded)]);
                    }
                    if point > current {
                        self.labour.day.raised += u64::from(k);
                    } else if point < current {
                        self.labour.day.cut += u64::from(k);
                    }
                }
                Missing::Absent => {
                    let Some(last) = u32::try_from(current).ok() else { continue };
                    for (p, person) in persons {
                        self.set_person(p, person, &[(kind.state, class::SEARCHING), (kind.last_point, last)]);
                        self.labour.searchers.insert(p);
                    }
                    self.labour.day.quits += u64::from(k);
                }
            }
        }
    }
}
