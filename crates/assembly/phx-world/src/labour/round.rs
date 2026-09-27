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

/// A searching person as the round reads it: its agent, its place, its region and country, its skill,
/// its experience, its occupation and its last wage point.
#[derive(Clone, Copy, Debug)]
struct Seeker {
    party: PartyId,
    person: u32,
    region: u32,
    country: CountryId,
    skill: u32,
    experience: u32,
    occupation: u32,
    last: u32,
}

/// What the searchers' round reads alike for every searcher: the persons already waiting on an offer or a hire, the
/// open vacancies by region and occupation, each vacancy's monthly wage and whether its employer is hiring.
type Shared<'a> = (&'a BTreeSet<(PartyId, u32)>, &'a BTreeMap<(u32, u32), Vec<usize>>, &'a [Option<f64>], &'a [bool]);

/// A searching agent's round as read on the pool.
enum Search {
    /// Dead, or with no person searching: it leaves the searchers.
    Gone,
    /// The player's agent, whose choices come from its queue on the calling thread.
    Player(Vec<Seeker>),
    /// The applications its persons' rule sends, and the vacancies they saw.
    Sent { applications: Vec<Application>, visible: u64 },
}

impl World {
    /// 5c: the day's round of labour, a step of each part.
    #[clause("LAB.4", "LAB.5", "LAB.7", "LAB.8", "LAB.15", "TIME.10")]
    pub(crate) fn labour_round(&mut self, day: Day) {
        let Some(kind) = self.labour.kind else { return };
        let book = &mut self.labour.book;
        let live: BTreeSet<PartyId> = book.vacancies.iter().map(|v| v.employer).collect();
        let gone: BTreeSet<PartyId> = live.into_iter().filter(|p| !self.live(*p)).collect();
        self.labour.book.keep(|v| !gone.contains(&v.employer));
        self.answer_offers(day, &kind);
        self.select_applicants(day, &kind);
        self.search(day, &kind);
        self.post(day);
        let held: BTreeSet<u32> = self.labour.book.offers.iter().map(|o| o.vacancy).collect();
        self.labour.book.keep(|v| v.open > 0 || held.contains(&v.id));
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

    /// The offers made the day before answered, each person's together: of those it would accept, the one paying most
    /// becomes a hire at the next start of work, its vacancy's fill recorded, ties to the vacancy posted first; every
    /// other returns its members to its vacancy, since a person takes one job.
    fn answer_offers(&mut self, day: Day, kind: &LabourKind) {
        let mut by_person: BTreeMap<(PartyId, u32), Vec<Offer>> = BTreeMap::new();
        for o in std::mem::take(&mut self.labour.book.offers) {
            by_person.entry((o.applicant, o.person)).or_default().push(o);
        }
        for ((applicant, person), offers) in by_person {
            let seeker = if self.live(applicant) {
                self.seekers(kind, applicant).into_iter().find(|s| s.person == person)
            } else {
                None
            };
            let Some(s) = seeker else {
                for o in &offers {
                    self.return_jobs(o.vacancy);
                }
                continue;
            };
            let law = super::law_of(&self.labour.laws, s.country);
            let mut d = self.labour_draws(kind.taste_stream, Subject::new(SubjectTag::Party, applicant.get()), day);
            let mut best: Option<(f64, u32)> = None;
            for o in &offers {
                let Some(v) = self.labour.book.vacancy(o.vacancy) else { continue };
                let wage = self.wage_at(law, v.point);
                let taste = phx_rand::gumbel(&mut d, 0.0, 1.0) - phx_rand::gumbel(&mut d, 0.0, 1.0);
                let input = AcceptIn { wage, reservation: self.reservation(&s), taste, wage_weight: law.wage_weight };
                let decider = self.queue.decider(applicant);
                let queued = match decider {
                    phx_core::decisions::Decider::Player { .. } => self.queue.take(applicant, kind.accept.name),
                    phx_core::decisions::Decider::Rule => None,
                };
                let accepts = phx_core::decisions::dispatch(kind.accept, decider, queued.as_deref(), &input);
                let better = |(w, id): (f64, u32)| wage.total_cmp(&w).then_with(|| id.cmp(&v.id)).is_gt();
                if accepts == Some(true) && best.is_none_or(better) {
                    best = Some((wage, v.id));
                }
            }
            for o in offers {
                let taken = best.is_some_and(|(_, id)| id == o.vacancy);
                let v = self.labour.book.vacancy(o.vacancy).cloned();
                let (true, Some(v)) = (taken, v) else {
                    self.return_jobs(o.vacancy);
                    continue;
                };
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
                    country: v.country,
                    class,
                    point: v.point,
                    stood,
                });
                self.labour.day.matches += 1;
            }
        }
    }

    /// A household's persons moved to new places, or gone: its applications, offers and hires follow each person to
    /// its place, and a gone person's are dropped, an offer's members returned to its vacancy.
    pub(crate) fn labour_renumber(&mut self, party: PartyId, places: &[Option<usize>]) {
        let place = |person: u32| {
            usize::try_from(person)
                .ok()
                .and_then(|i| places.get(i).copied().flatten())
                .and_then(|n| u32::try_from(n).ok())
        };
        let book = &mut self.labour.book;
        book.applications.retain_mut(|a| {
            if a.applicant != party {
                return true;
            }
            place(a.person).map(|p| a.person = p).is_some()
        });
        book.hires.retain_mut(|h| {
            if h.employee != party {
                return true;
            }
            place(h.person).map(|p| h.person = p).is_some()
        });
        let mut returned: Vec<u32> = Vec::new();
        book.offers.retain_mut(|o| {
            if o.applicant != party {
                return true;
            }
            if let Some(p) = place(o.person) {
                o.person = p;
                true
            } else {
                returned.push(o.vacancy);
                false
            }
        });
        for vacancy in returned {
            self.return_jobs(vacancy);
        }
    }

    /// The job an offer held returned to its vacancy.
    fn return_jobs(&mut self, vacancy: u32) {
        if let Some(v) = self.labour.book.vacancy_mut(vacancy) {
            v.open += 1;
        }
    }

    /// An employer's fill of an occupation: the point it filled at and the days its vacancy stood.
    fn record_fill(&mut self, v: &super::book::Vacancy, days: u32) {
        self.labour.book.fills.insert((v.employer, v.occupation), super::book::Fill { point: v.point, days });
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
            let country = self.labour.book.vacancy(a.vacancy).map(|v| v.country);
            let Some(country) = country else { continue };
            let law = super::law_of(&self.labour.laws, CountryId::new(country));
            if phx_rand::open_unit(&mut d) < law.seen_chance {
                by_vacancy.entry(a.vacancy).or_default().push(a);
            }
        }
        for (id, met) in by_vacancy {
            let Some(open) = self.labour.book.vacancy(id).map(|v| v.open) else { continue };
            let mut lots = self.labour_draws(kind.lot_stream, Subject::new(SubjectTag::Market, u64::from(id)), day);
            let applicants: Vec<Applicant> = met
                .iter()
                .map(|a| Applicant {
                    skill: a.skill,
                    experience: a.experience,
                    lot: phx_rand::uniform::below_u64(&mut lots, u64::MAX),
                })
                .collect();
            let chosen = (kind.select)(&SelectIn { applicants, open });
            for k in chosen {
                let Some(a) = usize::try_from(k).ok().and_then(|k| met.get(k)) else { continue };
                if let Some(v) = self.labour.book.vacancy_mut(id) {
                    v.open -= 1;
                }
                self.labour.book.offers.push(Offer {
                    vacancy: id,
                    applicant: a.applicant,
                    person: a.person,
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
    /// jobs open, draws a taste for each and applies to those
    /// its rule chooses, a round's share of a week's applications. An agent none of whose persons search leaves the
    /// searchers.
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
        // Each open vacancy's monthly wage, reckoned once for every searcher who sees it: its law's wage at its point.
        let wages: Vec<Option<f64>> = self
            .labour
            .book
            .vacancies
            .iter()
            .map(|v| {
                let law = self.region_country(v.region).map(|c| super::law_of(&self.labour.laws, c));
                law.filter(|_| v.open > 0).map(|law| self.wage_at(law, v.point))
            })
            .collect();
        let hiring: Vec<bool> =
            self.labour.book.vacancies.iter().map(|v| v.open > 0 && self.live(v.employer)).collect();
        let searchers: Vec<PartyId> = self.labour.searchers.iter().copied().collect();
        let shared = (&waiting, &standing, &wages[..], &hiring[..]);
        let shards = crate::consts::SEARCH_SHARDS;
        let read = phx_exec::pool::map(self.books.pool(), shards, |k| {
            crate::shard::part(&searchers, shards, k)
                .iter()
                .map(|p| self.searched(day, kind, *p, shared))
                .collect::<Vec<_>>()
        });
        for (party, out) in searchers.into_iter().zip(read.into_iter().flatten()) {
            match out {
                Search::Gone => {
                    self.labour.searchers.remove(&party);
                }
                Search::Player(seekers) => {
                    self.labour.day.searching_groups += 1;
                    let mut d = self.labour_draws(kind.taste_stream, Subject::new(SubjectTag::Party, party.get()), day);
                    for s in seekers.iter().filter(|s| !waiting.contains(&(s.party, s.person))) {
                        let Some(standing) = standing.get(&(s.region, s.occupation)) else { continue };
                        let (visible, prepared) = self.prepared(s, (standing, &wages, &hiring), &mut d);
                        self.labour.day.vacancies_visible += visible;
                        let Some((seen, input)) = prepared else { continue };
                        let decider = self.queue.decider(s.party);
                        let queued = self.queue.take(s.party, kind.search.name);
                        let chosen = phx_core::decisions::dispatch(kind.search, decider, queued.as_deref(), &input);
                        let sent = self.sent(day, s, &seen, chosen.unwrap_or_default());
                        self.push_applications(sent);
                    }
                }
                Search::Sent { applications, visible } => {
                    self.labour.day.searching_groups += 1;
                    self.labour.day.vacancies_visible += visible;
                    self.push_applications(applications);
                }
            }
        }
    }

    /// A searching agent's round read without touching the world: gone, left to its player's queue, or the
    /// applications its persons' rule sends and the vacancies they saw.
    fn searched(
        &self,
        day: Day,
        kind: &LabourKind,
        party: PartyId,
        (waiting, standing, wages, hiring): Shared<'_>,
    ) -> Search {
        if !self.live(party) {
            return Search::Gone;
        }
        let seekers = self.seekers(kind, party);
        if seekers.is_empty() {
            return Search::Gone;
        }
        if let phx_core::decisions::Decider::Player { .. } = self.queue.decider(party) {
            return Search::Player(seekers);
        }
        let mut d = self.labour_draws(kind.taste_stream, Subject::new(SubjectTag::Party, party.get()), day);
        let mut applications = Vec::new();
        let mut visible = 0;
        for s in seekers.iter().filter(|s| !waiting.contains(&(s.party, s.person))) {
            let Some(standing) = standing.get(&(s.region, s.occupation)) else { continue };
            let (seen_count, prepared) = self.prepared(s, (standing, wages, hiring), &mut d);
            visible += seen_count;
            let Some((seen, input)) = prepared else { continue };
            let chosen = phx_core::decisions::dispatch(kind.search, phx_core::decisions::Decider::Rule, None, &input);
            applications.extend(self.sent(day, s, &seen, chosen.unwrap_or_default()));
        }
        Search::Sent { applications, visible }
    }

    /// The day's applications entered in the book, in the order they were sent.
    fn push_applications(&mut self, sent: Vec<Application>) {
        self.labour.day.applications += phx_rand::float::len_u64(sent.len());
        self.labour.book.applications.extend(sent);
    }

    /// One searching person's round up to its decision: the vacancies it sees, and, when it sees any, the rule's
    /// input for them, its count of applications and its tastes drawn.
    fn prepared(
        &self,
        s: &Seeker,
        (standing, wage_of, hiring): (&[usize], &[Option<f64>], &[bool]),
        d: &mut Draws,
    ) -> (u64, Option<(Vec<usize>, SearchIn)>) {
        let law = super::law_of(&self.labour.laws, s.country);
        let seen: Vec<usize> = standing
            .iter()
            .copied()
            .filter(|i| {
                hiring.get(*i).copied().unwrap_or(false)
                    && self.labour.book.vacancies.get(*i).is_some_and(|v| v.skill <= s.skill)
            })
            .collect();
        let visible = phx_rand::float::len_u64(seen.len());
        if seen.is_empty() {
            return (visible, None);
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
            .map(|i| match wage_of.get(*i).copied().flatten() {
                Some(w) => w,
                None => violation!(clause = "REP.34", "an open vacancy seen with no wage", vacancy = *i),
            })
            .collect();
        let tastes: Vec<f64> = seen.iter().map(|_| phx_rand::gumbel(d, 0.0, 1.0)).collect();
        let input =
            SearchIn { wages, tastes, reservation: self.reservation(s), applications, wage_weight: law.wage_weight };
        (visible, Some((seen, input)))
    }

    /// The applications a person's choice of the vacancies it saw sends.
    fn sent(&self, day: Day, s: &Seeker, seen: &[usize], chosen: Vec<u32>) -> Vec<Application> {
        chosen
            .into_iter()
            .filter_map(|k| {
                usize::try_from(k).ok().and_then(|k| seen.get(k)).and_then(|i| self.labour.book.vacancies.get(*i))
            })
            .map(|v| Application {
                vacancy: v.id,
                applicant: s.party,
                person: s.person,
                skill: s.skill,
                experience: s.experience,
                sent: day,
            })
            .collect()
    }
}
