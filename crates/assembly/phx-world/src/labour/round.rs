//! The day's round at 5c, after the visits, in order: the offers made the day before answered by their applicants;
//! the applications sent the day before met by their employers at the meeting's chance and selected; the searchers'
//! applications to the vacancies standing at the start of the day; and the employers whose production schedule came
//! due today posting, withdrawing and laying off. A match takes three days at least, as real hiring does.

use std::collections::{BTreeMap, BTreeSet};

use if_labour::class;
use if_labour::decisions::{AcceptIn, Applicant, SearchIn, SelectIn};
use if_labour::kind::LabourKind;
use if_labour::law::Law;
use phx_core::SubStep;
use phx_id::{CountryId, Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_rand::{Draws, Subject, SubjectTag};
use phx_store::SystemBacking;

use super::book::{Application, Hire, Offer};
use crate::world::World;

/// A searching person as the round reads it: its agent, its place, its twins, its region and country, its skill,
/// its experience, its occupation and its last wage point.
#[derive(Clone, Copy, Debug)]
struct Seeker {
    party: PartyId,
    person: u32,
    unit: u32,
    region: u32,
    country: CountryId,
    skill: u32,
    experience: u32,
    occupation: u32,
    last: u32,
}

impl World {
    /// 5c: the day's round of labour, a step of each part.
    #[clause("LAB.4", "LAB.5", "LAB.7", "LAB.8", "TIME.10")]
    pub(crate) fn labour_round(&mut self, day: Day) {
        let Some(kind) = self.labour.kind else { return };
        let book = &mut self.labour.book;
        let live: BTreeSet<PartyId> = book.vacancies.iter().map(|v| v.employer).collect();
        let gone: BTreeSet<PartyId> = live.into_iter().filter(|p| !self.live(*p)).collect();
        self.labour.book.vacancies.retain(|v| !gone.contains(&v.employer));
        self.answer_offers(day, &kind);
        self.select_applicants(day, &kind);
        self.search(day, &kind);
        self.post(day);
        let held: BTreeSet<u32> = self.labour.book.offers.iter().map(|o| o.vacancy).collect();
        self.labour.book.vacancies.retain(|v| v.open > 0 || held.contains(&v.id));
    }

    /// A stream the labour kind names, opened for a subject on the day.
    fn labour_draws(&self, name: &str, subject: Subject, day: Day) -> Draws {
        let Some(stream) = self.streams.named(name) else {
            violation!(clause = "CHN.1", "labour drawing from a stream never declared");
        };
        self.streams.open(&stream, subject, day, SubStep::S5c.ordinal())
    }

    /// The searching persons of an agent, each with what the round reads of it.
    fn seekers(&self, kind: &LabourKind, party: PartyId) -> Vec<Seeker> {
        let Some(place) = self.labour_kind_place(kind) else { return Vec::new() };
        let Some(decl) = self.population.kinds.get(place).map(|k| &k.decl) else { return Vec::new() };
        let (_, slot) = self.books.parties.row(party);
        let table = Population::table::<SystemBacking>(self.books.parties.cells(), place);
        let Missing::Present(at) = decl.sited_by else { return Vec::new() };
        let region = table.attr(slot, at);
        let Some(country) = self.region_country(region) else { return Vec::new() };
        let law = super::law_of(&self.labour.laws, country);
        let unit = table.multiplicity(slot).get();
        let date = self.calendar.date(self.today);
        (0_u32..)
            .zip(table.persons(slot))
            .filter_map(|(i, w)| {
                let p = phx_pop::person::unpack(decl, *w);
                if p.attr(kind.state) != Some(class::SEARCHING) {
                    return None;
                }
                let skill =
                    p.attr(kind.education).and_then(|e| law.education_skill.get(usize::try_from(e).ok()?)).copied()?;
                let experience = u32::try_from(p.age_on(date)).ok()?;
                Some(Seeker {
                    party,
                    person: i,
                    unit,
                    region,
                    country,
                    skill,
                    experience,
                    occupation: p.attr(kind.occupation)?,
                    last: p.attr(kind.last_point)?,
                })
            })
            .collect()
    }

    /// The country a region lies in, by its market zone.
    pub(crate) fn region_country(&self, region: u32) -> Option<CountryId> {
        let zone = usize::try_from(region).ok().and_then(|r| self.goods_frame.market_zones.get(r))?;
        let Missing::Present(zone) = zone else { return None };
        match self.geo().zone_country(*zone) {
            Missing::Present(c) => Some(c),
            Missing::Absent => None,
        }
    }

    /// The monthly wage at a point, by the labour kind's arithmetic.
    pub(crate) fn wage_at(&self, law: &Law, point: i64) -> f64 {
        match self.labour.kind {
            Some(k) => (k.wage_at)(law, point),
            None => violation!(clause = "REP.34", "a wage point read in a world with no labour"),
        }
    }

    /// A searcher's reservation: its law's share of its last wage.
    fn reservation(&self, s: &Seeker) -> f64 {
        let law = super::law_of(&self.labour.laws, s.country);
        law.reservation_share * self.wage_at(law, i64::from(s.last))
    }

    /// The offers made the day before answered: each accepted becomes a hire at the next start of work, its vacancy's
    /// fill recorded; each refused returns its members to its vacancy.
    fn answer_offers(&mut self, day: Day, kind: &LabourKind) {
        let offers = std::mem::take(&mut self.labour.book.offers);
        for o in offers {
            let Some(v) = self.labour.book.vacancies.iter().find(|v| v.id == o.vacancy).cloned() else { continue };
            let seeker = if self.live(o.applicant) {
                self.seekers(kind, o.applicant).into_iter().find(|s| s.person == o.person)
            } else {
                None
            };
            let Some(s) = seeker else {
                self.return_jobs(o.vacancy, o.unit);
                continue;
            };
            let law = super::law_of(&self.labour.laws, s.country);
            let mut d = self.labour_draws(kind.taste_stream, Subject::new(SubjectTag::Party, o.applicant.get()), day);
            let taste = phx_rand::gumbel(&mut d, 0.0, 1.0) - phx_rand::gumbel(&mut d, 0.0, 1.0);
            let input = AcceptIn {
                wage: self.wage_at(law, v.point),
                reservation: self.reservation(&s),
                taste,
                wage_weight: law.wage_weight,
            };
            let decider = self.queue.decider(o.applicant);
            let queued = match decider {
                phx_core::decisions::Decider::Player { .. } => self.queue.take(o.applicant, kind.accept.name),
                phx_core::decisions::Decider::Rule => None,
            };
            let accepts = phx_core::decisions::dispatch(kind.accept, decider, queued.as_deref(), &input);
            if accepts != Some(true) {
                self.return_jobs(o.vacancy, o.unit);
                continue;
            }
            self.labour.day.acceptances += 1;
            let Some(stood) = self.calendar.days_between(v.first, day) else {
                violation!(clause = "LAB.2", "a vacancy posted after its offer was answered", vacancy = v.id);
            };
            self.labour.day.match_days += u64::from(stood);
            self.record_fill(&v, stood);
            let class = self.class_of(&v);
            self.labour.book.hires.push(Hire {
                employer: v.employer,
                employee: o.applicant,
                person: o.person,
                unit: o.unit,
                country: v.country,
                class,
                point: v.point,
                stood,
            });
            self.labour.day.matches += 1;
        }
    }

    /// Members an offer held returned to its vacancy.
    fn return_jobs(&mut self, vacancy: u32, unit: u32) {
        if let Some(v) = self.labour.book.vacancies.iter_mut().find(|v| v.id == vacancy) {
            v.open += unit;
        }
    }

    /// An employer's fill of an occupation: the point it filled at and the days its vacancy stood.
    fn record_fill(&mut self, v: &super::book::Vacancy, days: u32) {
        let fills = &mut self.labour.book.fills;
        match fills.iter_mut().find(|f| f.employer == v.employer && f.occupation == v.occupation) {
            Some(f) => {
                f.point = v.point;
                f.days = days;
            }
            None => {
                fills.push(super::book::Fill { employer: v.employer, occupation: v.occupation, point: v.point, days });
            }
        }
    }

    /// The class of the contracts a vacancy's hires join: its occupation, skill, hours and region, the country's
    /// notice and severance, and the band the hire begins in.
    fn class_of(&self, v: &super::book::Vacancy) -> Vec<u32> {
        let law = super::law_of(&self.labour.laws, CountryId::new(v.country));
        let year = i64::from(self.calendar.date(self.today).year());
        let band = i64::from(law.band_years);
        let Ok(first) = u32::try_from(year - year.rem_euclid(band)) else {
            violation!(clause = "REP.3", "a start band before the calendar's years", year = year);
        };
        let mut c = vec![0; class::PLACES];
        for (i, value) in [
            (class::OCCUPATION, v.occupation),
            (class::SKILL, v.skill),
            (class::HOURS, v.hours),
            (class::NOTICE, law.notice_days),
            (class::SEVERANCE, law.severance_days_a_year),
            (class::REGION, v.region),
            (class::BAND, first),
        ] {
            if let Some(slot) = c.get_mut(i) {
                *slot = value;
            }
        }
        c
    }

    /// The applications sent the day before met by their employers, each at the meeting's chance, and each vacancy's
    /// met applicants selected up to its jobs open; the chosen offered the job, the rest to apply again.
    fn select_applicants(&mut self, day: Day, kind: &LabourKind) {
        let apps = std::mem::take(&mut self.labour.book.applications);
        let mut by_vacancy: BTreeMap<u32, Vec<Application>> = BTreeMap::new();
        for a in apps {
            let mut d = self.labour_draws(kind.meeting_stream, Subject::new(SubjectTag::Party, a.applicant.get()), day);
            let country = self.labour.book.vacancies.iter().find(|v| v.id == a.vacancy).map(|v| v.country);
            let Some(country) = country else { continue };
            let law = super::law_of(&self.labour.laws, CountryId::new(country));
            if phx_rand::open_unit(&mut d) < law.seen_chance {
                by_vacancy.entry(a.vacancy).or_default().push(a);
            }
        }
        for (id, met) in by_vacancy {
            let Some(open) = self.labour.book.vacancies.iter().find(|v| v.id == id).map(|v| v.open) else { continue };
            let mut lots = self.labour_draws(kind.lot_stream, Subject::new(SubjectTag::Market, u64::from(id)), day);
            let applicants: Vec<Applicant> = met
                .iter()
                .map(|a| Applicant {
                    skill: a.skill,
                    experience: a.experience,
                    lot: phx_rand::uniform::below_u64(&mut lots, u64::MAX),
                    unit: a.unit,
                })
                .collect();
            let chosen = (kind.select)(&SelectIn { applicants, open });
            for k in chosen {
                let Some(a) = usize::try_from(k).ok().and_then(|k| met.get(k)) else { continue };
                if let Some(v) = self.labour.book.vacancies.iter_mut().find(|v| v.id == id) {
                    v.open -= a.unit;
                }
                self.labour.book.offers.push(Offer {
                    vacancy: id,
                    applicant: a.applicant,
                    person: a.person,
                    unit: a.unit,
                    made: day,
                });
                self.labour.day.offers += 1;
            }
        }
    }
}

impl World {
    /// The searchers' applications: each searching person of each searching agent, not already waiting on an
    /// application or an offer, sees the vacancies standing in its region in its occupation, at a skill it has, with
    /// jobs open for its twins, draws a taste for each and applies to those its rule chooses, a round's share of a
    /// week's applications. An agent none of whose persons search leaves the searchers.
    #[clause("LAB.5", "LAB.8", "REP.22")]
    fn search(&mut self, day: Day, kind: &LabourKind) {
        let waiting: BTreeSet<(PartyId, u32)> = self
            .labour
            .book
            .offers
            .iter()
            .map(|o| (o.applicant, o.person))
            .chain(self.labour.book.hires.iter().map(|h| (h.employee, h.person)))
            .collect();
        let mut standing: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        for (i, v) in self.labour.book.vacancies.iter().enumerate() {
            if v.open > 0 {
                standing.entry((v.region, v.occupation)).or_default().push(i);
            }
        }
        let searchers: Vec<PartyId> = self.labour.searchers.iter().copied().collect();
        for party in searchers {
            if !self.live(party) {
                self.labour.searchers.remove(&party);
                continue;
            }
            let seekers = self.seekers(kind, party);
            if seekers.is_empty() {
                self.labour.searchers.remove(&party);
                continue;
            }
            self.labour.day.searching_groups += 1;
            let mut d = self.labour_draws(kind.taste_stream, Subject::new(SubjectTag::Party, party.get()), day);
            for s in seekers.iter().filter(|s| !waiting.contains(&(s.party, s.person))) {
                self.apply_round(
                    day,
                    kind,
                    s,
                    standing.get(&(s.region, s.occupation)).map_or(&[][..], Vec::as_slice),
                    &mut d,
                );
            }
        }
    }

    /// One searching person's applications this round.
    fn apply_round(&mut self, day: Day, kind: &LabourKind, s: &Seeker, standing: &[usize], d: &mut Draws) {
        let law = super::law_of(&self.labour.laws, s.country).clone();
        let seen: Vec<usize> = standing
            .iter()
            .copied()
            .filter(|i| self.labour.book.vacancies.get(*i).is_some_and(|v| v.skill <= s.skill && v.open >= s.unit))
            .collect();
        self.labour.day.vacancies_visible += phx_rand::float::len_u64(seen.len());
        if seen.is_empty() {
            return;
        }
        // A week's applications spread over its days: the whole part every day, one more at the chance of the rest.
        let a_day = law.applications_a_week / crate::consts::DAYS_A_WEEK;
        let Some(whole) = phx_rand::float::floor_to_u64(a_day).and_then(|n| u32::try_from(n).ok()) else {
            violation!(clause = "LAB.16", "a round's applications beyond counting");
        };
        let rest = a_day - f64::from(whole);
        let applications = if phx_rand::open_unit(d) < rest { whole + 1 } else { whole };
        let wages: Vec<f64> = seen
            .iter()
            .filter_map(|i| self.labour.book.vacancies.get(*i))
            .map(|v| self.wage_at(&law, v.point))
            .collect();
        let tastes: Vec<f64> = seen.iter().map(|_| phx_rand::gumbel(d, 0.0, 1.0)).collect();
        let input =
            SearchIn { wages, tastes, reservation: self.reservation(s), applications, wage_weight: law.wage_weight };
        let decider = self.queue.decider(s.party);
        let queued = match decider {
            phx_core::decisions::Decider::Player { .. } => self.queue.take(s.party, kind.search.name),
            phx_core::decisions::Decider::Rule => None,
        };
        let Some(chosen) = phx_core::decisions::dispatch(kind.search, decider, queued.as_deref(), &input) else {
            return;
        };
        for k in chosen {
            let Some(v) =
                usize::try_from(k).ok().and_then(|k| seen.get(k)).and_then(|i| self.labour.book.vacancies.get(*i))
            else {
                continue;
            };
            let vacancy = v.id;
            self.labour.book.applications.push(Application {
                vacancy,
                applicant: s.party,
                person: s.person,
                unit: s.unit,
                skill: s.skill,
                experience: s.experience,
                sent: day,
            });
            self.labour.day.applications += 1;
        }
    }
}
