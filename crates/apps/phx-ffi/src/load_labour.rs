//! The bench's labour round: vacancies posted by firms, a day's searchers drawn from the households' persons, and each
//! day's step of the matching — yesterday's offers answered, yesterday's applications met and selected, today's
//! applications drawn — a filled vacancy posted again by another firm, so the book keeps its size.

use phx_exec::{Pool, mix64};
use phx_id::{PartyKey, Slot};
use phx_market::hiring::{Applicant, Application, Seeker, Standing, Vacancy, answer, search, select};
use phx_rand::{Draws, Seed, StreamKey, Subject, SubjectTag, stream_key};

use super::{index_u64, to_u32, to_usize};

/// Occupation families and skill levels the book spreads over.
const OCCUPATIONS: u32 = 10;
const SKILLS: u32 = 4;
/// Monthly wages: the least and its spread; a searcher's reservation is a share of a wage it last earned.
const WAGE_FLOOR: u64 = 1_800;
const WAGE_SPREAD: u64 = 1_600;
const RESERVATION_SHARE: f64 = 0.8;
/// Jobs a vacancy opens with, at most.
const JOBS: u64 = 3;
/// The weight of a wage's log, the applications a searcher sends a day and the chance an application is met.
const WAGE_WEIGHT: f64 = 2.0;
const APPLICATIONS_A_DAY: f64 = 0.6;
const SEEN_CHANCE: f64 = 0.7;
/// Years of experience a searcher brings, at most.
const EXPERIENCE: u64 = 40;

pub(super) struct LabourLoad {
    firm: u8,
    firms: u32,
    household: u8,
    households: u32,
    regions: u32,
    vacancies: Vec<Vacancy>,
    applications: Vec<Application>,
    offers: Vec<Application>,
    posted: u64,
    taste: StreamKey,
    meeting: StreamKey,
    lot: StreamKey,
}

impl LabourLoad {
    /// A book of `n` vacancies over the firms, spread over the regions, occupations and skills.
    pub fn new((firm, firms): (u8, u32), (household, households): (u8, u32), regions: u32, n: u64) -> LabourLoad {
        let mut l = LabourLoad {
            firm,
            firms,
            household,
            households,
            regions,
            vacancies: Vec::new(),
            applications: Vec::new(),
            offers: Vec::new(),
            posted: 0,
            taste: stream_key(Seed::new(21), "LAB.match_taste"),
            meeting: stream_key(Seed::new(22), "LAB.meeting"),
            lot: stream_key(Seed::new(23), "LAB.lot"),
        };
        l.vacancies = (0..n).map(|_| l.post()).collect();
        l
    }

    /// The next vacancy posted: a firm, its region, an occupation, a skill, a wage and its jobs.
    fn post(&mut self) -> Vacancy {
        self.posted += 1;
        let h = mix64(self.posted);
        let f = to_u32(h % u64::from(self.firms));
        Vacancy {
            employer: PartyKey::new(self.firm, Slot::new(f)),
            region: f % self.regions,
            occupation: to_u32((h >> 16) % u64::from(OCCUPATIONS)),
            skill: to_u32((h >> 24) % u64::from(SKILLS)),
            wage: phx_rand::float::from_u64(WAGE_FLOOR + (h >> 32) % WAGE_SPREAD),
            open: to_u32(1 + (h >> 8) % JOBS),
        }
    }

    /// The day's step for `count` searchers; returns the searchers, applications, offers and hires it handled.
    pub fn day(&mut self, pool: &Pool, day: u32, count: u64) -> u64 {
        let taste = self.taste;
        let accepts = |o: &Application, v: &Vacancy| {
            let mut d = Draws::new(taste, Subject::new(SubjectTag::Party, o.seeker.subject), day, 1);
            let t = phx_rand::gumbel(&mut d, 0.0, 1.0) - phx_rand::gumbel(&mut d, 0.0, 1.0);
            WAGE_WEIGHT * (v.wage / o.seeker.reservation).ln() + t > 0.0
        };
        let offers = std::mem::take(&mut self.offers);
        let hires = answer(&mut self.vacancies, &offers, accepts);
        let meeting = self.meeting;
        let met = |a: &Application| {
            let mut d = Draws::new(meeting, Subject::new(SubjectTag::Party, a.seeker.subject), day, 0);
            phx_rand::open_unit(&mut d) < SEEN_CHANCE
        };
        let lot = self.lot;
        let lots = |v: u32| Draws::new(lot, Subject::new(SubjectTag::Market, u64::from(v)), day, 0);
        let by_skill = |a: &[Applicant], open: u32| {
            let mut order: Vec<u32> = (0..to_u32(index_u64(a.len()))).collect();
            order.sort_by_key(|k| {
                a.get(to_usize(u64::from(*k)))
                    .map(|x| (std::cmp::Reverse(x.skill), std::cmp::Reverse(x.experience), x.lot))
            });
            order.truncate(to_usize(u64::from(open)));
            order
        };
        let applications = std::mem::take(&mut self.applications);
        self.offers = select(&mut self.vacancies, &applications, met, (lots, by_skill));
        // A vacancy filled is posted again, by another firm, once no offer holds one of its jobs.
        let mut held = vec![false; self.vacancies.len()];
        for o in &self.offers {
            if let Some(h) = held.get_mut(to_usize(u64::from(o.vacancy))) {
                *h = true;
            }
        }
        for at in 0..self.vacancies.len() {
            if self.vacancies.get(at).is_some_and(|v| v.open == 0) && !held.get(at).copied().unwrap_or(true) {
                let fresh = self.post();
                if let Some(v) = self.vacancies.get_mut(at) {
                    *v = fresh;
                }
            }
        }
        let standing = Standing::new(&self.vacancies);
        let seekers: Vec<Seeker> = (0..count)
            .map(|i| {
                let h = mix64(u64::from(day) << 32 | i);
                let slot = to_u32(h % u64::from(self.households));
                Seeker {
                    household: PartyKey::new(self.household, Slot::new(slot)),
                    person: (h >> 40) % 2,
                    subject: h & SUBJECT_BITS,
                    region: slot % self.regions,
                    occupation: to_u32((h >> 16) % u64::from(OCCUPATIONS)),
                    skill: to_u32((h >> 24) % u64::from(SKILLS)),
                    experience: to_u32((h >> 32) % EXPERIENCE),
                    reservation: RESERVATION_SHARE * phx_rand::float::from_u64(WAGE_FLOOR + (h >> 20) % WAGE_SPREAD),
                }
            })
            .collect();
        let draws = |subject: u64| Draws::new(taste, Subject::new(SubjectTag::Party, subject), day, 0);
        self.applications =
            search(Some(pool), (&self.vacancies, &standing), &seekers, (WAGE_WEIGHT, APPLICATIONS_A_DAY), &draws);
        count + index_u64(applications.len() + offers.len() + hires.len())
    }
}

/// A subject's identity bits: the sixty below its tag.
const SUBJECT_BITS: u64 = (1 << 60) - 1;
