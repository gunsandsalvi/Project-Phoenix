//! Labour matching on the core, a step a day. Open vacancies stand in a buffer grouped by region and occupation, each
//! group by the skill it asks, least first, so a searcher's reach is a prefix of its group. Each searcher draws its
//! applications from its own stream, without replacement, from the logit over the weight of each reachable vacancy's
//! wage's log among those paying above its reservation. The next day each employer meets its applicants at the
//! meeting's chance and selects among them up to its jobs open, and the day after each person answers its offers,
//! taking the best-paid it accepts; the others' jobs return to their vacancies. The hires go on to become employment
//! contracts where the world applies them. The world hands in only vacancies whose employer lives and seekers not
//! already waiting on an offer or a hire.

use phx_exec::Pool;
use phx_id::PartyKey;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;
use phx_rand::uniform::open_unit;

use crate::consts::SEARCH_CHUNK;

/// An employer's posted jobs: the region and occupation family they are in, the skill they ask, the monthly wage at
/// their point, and the jobs still open.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Vacancy {
    pub employer: PartyKey,
    pub region: u32,
    pub occupation: u32,
    pub skill: u32,
    pub wage: f64,
    pub open: u32,
}

/// A person searching for work: its household and its identity, the identity its draws are made for, where it looks,
/// the skill and years of experience it brings, and the least monthly wage it takes.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Seeker {
    pub household: PartyKey,
    pub person: u64,
    pub subject: u64,
    pub region: u32,
    pub occupation: u32,
    pub skill: u32,
    pub experience: u32,
    pub reservation: f64,
}

/// A seeker's application to a vacancy, carrying what its employer reads of it.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Application {
    pub vacancy: u32,
    pub seeker: Seeker,
}

/// An applicant as an employer ranks it: its skill, its experience and its lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applicant {
    pub skill: u32,
    pub experience: u32,
    pub lot: u64,
}

/// The open vacancies by region and occupation, each group by skill asked, least first, then by their place.
#[derive(Clone, Debug, Default)]
pub struct Standing {
    order: Vec<u32>,
    groups: Vec<((u32, u32), usize)>,
}

impl Standing {
    /// The vacancies with jobs open, grouped.
    #[must_use]
    pub fn new(vacancies: &[Vacancy]) -> Standing {
        let mut order: Vec<u32> = (0_u32..).zip(vacancies).filter(|(_, v)| v.open > 0).map(|(i, _)| i).collect();
        let at = |i: &u32| vacancies.get(usize::try_from(*i).unwrap_or(usize::MAX));
        order.sort_by_key(|i| at(i).map(|v| (v.region, v.occupation, v.skill, *i)));
        let mut groups: Vec<((u32, u32), usize)> = Vec::new();
        for (k, i) in order.iter().enumerate() {
            let Some(v) = at(i) else { continue };
            if groups.last().is_none_or(|(g, _)| *g != (v.region, v.occupation)) {
                groups.push(((v.region, v.occupation), k));
            }
        }
        Standing { order, groups }
    }

    /// The vacancies in a searcher's reach: its region's and occupation's, asking no more skill than it has.
    #[must_use]
    pub fn in_reach<'a>(&'a self, vacancies: &[Vacancy], s: &Seeker) -> &'a [u32] {
        let key = (s.region, s.occupation);
        let Ok(g) = self.groups.binary_search_by_key(&key, |(k, _)| *k) else { return &[] };
        let from = self.groups.get(g).map_or(0, |(_, f)| *f);
        let to = self.groups.get(g + 1).map_or(self.order.len(), |(_, t)| *t);
        let group = self.order.get(from..to).unwrap_or(&[]);
        let asks = |i: &u32| vacancies.get(usize::try_from(*i).unwrap_or(usize::MAX)).map_or(u32::MAX, |v| v.skill);
        let n = group.partition_point(|i| asks(i) <= s.skill);
        group.get(..n).unwrap_or(&[])
    }
}

/// The vacancies a seeker applies to among those it weighs, each with its pull: drawn one by one without replacement,
/// each at a draw of the unit interval, with chances in proportion to their pulls; as many as it has draws, or all it
/// weighs.
#[clause("LAB.5", "LAB.8", "REP.22")]
#[must_use]
pub fn pick(weighed: &[(u32, f64)], draws: &[f64]) -> Vec<u32> {
    let mut reach = weighed.to_vec();
    let mut out = Vec::new();
    for u in draws {
        let total: f64 = reach.iter().map(|(_, w)| w).sum();
        if reach.is_empty() || total.is_nan() || total <= 0.0 {
            break;
        }
        let u = u * total;
        let mut acc = 0.0;
        // A draw at the very top falls to the last.
        let mut at = reach.len() - 1;
        for (k, (_, w)) in reach.iter().enumerate() {
            acc += w;
            if u < acc {
                at = k;
                break;
            }
        }
        let (vacancy, _) = reach.remove(at);
        out.push(vacancy);
    }
    out
}

/// The searchers' round: each seeker sends a round's share of a week's applications — the whole part, and one more
/// at the chance of the rest — to vacancies in its reach paying above its reservation, chosen by `choose` from their
/// pulls, their wage to the power of `wage_weight`, and a draw for each application; drawn one by one without
/// replacement in proportion to the pulls, this is the logit over the weight of the wage's log with a standard Gumbel
/// taste for each. Each seeker draws from `draws` of its subject; the applications are in the seekers' order, the same
/// for any workers.
#[clause("LAB.5", "LAB.8", "REP.22")]
pub fn search(
    pool: Option<&Pool>,
    (vacancies, standing): (&[Vacancy], &Standing),
    seekers: &[Seeker],
    (wage_weight, a_day): (f64, f64),
    (draws, choose): (
        &(impl Fn(u64) -> Draws + Sync),
        &(impl Fn(&Seeker, Vec<(u32, f64)>, Vec<f64>) -> Vec<u32> + Sync),
    ),
) -> Vec<Application> {
    if a_day.is_nan() || a_day < 0.0 || a_day.is_infinite() {
        violation!(clause = "LAB.16", "a round's applications neither none nor a count");
    }
    let Some(whole) = phx_rand::float::floor_to_u64(a_day) else {
        violation!(clause = "LAB.16", "a round's applications beyond counting");
    };
    let rest = a_day - phx_rand::float::from_u64(whole);
    // A vacancy's pull is its wage to the weight, reckoned once for every seeker who sees it.
    let pull: Vec<f64> = vacancies.iter().map(|v| libm::pow(v.wage, wage_weight)).collect();
    let chunks: Vec<&[Seeker]> = seekers.chunks(SEARCH_CHUNK).collect();
    phx_exec::pool::map(pool, chunks.len(), |c| {
        let mut out = Vec::new();
        for s in chunks.get(c).copied().unwrap_or(&[]) {
            let mut d = draws(s.subject);
            let sends = whole + u64::from(open_unit(&mut d) < rest);
            let reach: Vec<(u32, f64)> = standing
                .in_reach(vacancies, s)
                .iter()
                .filter_map(|i| {
                    let at = usize::try_from(*i).ok()?;
                    let (v, w) = (vacancies.get(at)?, pull.get(at)?);
                    (v.wage > s.reservation).then_some((*i, *w))
                })
                .collect();
            if reach.is_empty() || sends == 0 {
                continue;
            }
            let units: Vec<f64> = (0..sends).map(|_| open_unit(&mut d)).collect();
            out.extend(choose(s, reach, units).into_iter().map(|vacancy| Application { vacancy, seeker: *s }));
        }
        out
    })
    .concat()
}

/// The employers' round over yesterday's applications: each is met at the chance `met` gives it; each vacancy's met
/// applicants, in the order they were sent, are handed with the vacancy and their lots, drawn from `lots` of the
/// vacancy, to `choose`,
/// which returns the places of those offered its jobs, no more than it has open. Returns the offers, in the vacancies'
/// order, each vacancy's jobs taken off as they are offered.
#[clause("LAB.7", "LAB.8")]
pub fn select(
    vacancies: &mut [Vacancy],
    applications: &[Application],
    met: impl Fn(&Application) -> bool,
    (lots, choose): (impl Fn(u32) -> Draws, impl Fn(&Vacancy, &[Applicant], u32) -> Vec<u32>),
) -> Vec<Application> {
    let mut seen: Vec<(u32, usize)> =
        applications.iter().enumerate().filter(|(_, a)| met(a)).map(|(k, a)| (a.vacancy, k)).collect();
    seen.sort_unstable();
    let mut offers = Vec::new();
    let mut from = 0;
    while let Some((vacancy, _)) = seen.get(from).copied() {
        let to = from + seen.get(from..).map_or(0, |r| r.partition_point(|(v, _)| *v == vacancy));
        let met: Vec<&Application> =
            seen.get(from..to).unwrap_or(&[]).iter().filter_map(|(_, k)| applications.get(*k)).collect();
        from = to;
        let Some(v) = vacancies.get_mut(usize::try_from(vacancy).unwrap_or(usize::MAX)) else {
            violation!(clause = "LAB.7", "an application to a vacancy that is not posted", vacancy = vacancy);
        };
        let mut d = lots(vacancy);
        let ranked: Vec<Applicant> = met
            .iter()
            .map(|a| Applicant {
                skill: a.seeker.skill,
                experience: a.seeker.experience,
                lot: phx_rand::uniform::below_u64(&mut d, u64::MAX),
            })
            .collect();
        let chosen = choose(v, &ranked, v.open);
        if chosen.len() > usize::try_from(v.open).unwrap_or(usize::MAX) {
            violation!(clause = "LAB.7", "more offered than a vacancy's jobs open", open = v.open);
        }
        for k in chosen {
            let Some(a) = met.get(usize::try_from(k).unwrap_or(usize::MAX)) else {
                violation!(clause = "LAB.7", "an offer to no applicant", applicant = k);
            };
            v.open -= 1;
            offers.push(**a);
        }
    }
    offers
}

/// The searchers' answers to yesterday's offers, each person's together: `accepts` tells whether it takes an offer, and
/// of those it takes the best-paid becomes a hire, ties to the vacancy listed first; every other offer's job returns
/// to its vacancy, since a person takes one job. Returns the hires, in the persons' order.
#[clause("LAB.5", "LAB.8")]
pub fn answer(
    vacancies: &mut [Vacancy],
    offers: &[Application],
    accepts: impl Fn(&Application, &Vacancy) -> bool,
) -> Vec<Application> {
    let mut by_person: Vec<(u32, u64, u32, usize)> = (0..offers.len())
        .filter_map(|k| offers.get(k).map(|o| (o.seeker.household.word(), o.seeker.person, o.vacancy, k)))
        .collect();
    by_person.sort_unstable();
    let mut hires = Vec::new();
    let mut from = 0;
    while let Some((household, person, _, _)) = by_person.get(from).copied() {
        let to = from
            + by_person.get(from..).map_or(0, |r| r.partition_point(|(h, p, _, _)| (*h, *p) == (household, person)));
        let mine = by_person.get(from..to).unwrap_or(&[]);
        from = to;
        let at = |v: u32| vacancies.get(usize::try_from(v).unwrap_or(usize::MAX)).copied();
        let mut best: Option<(f64, u32, usize)> = None;
        for (_, _, vacancy, k) in mine {
            let (Some(o), Some(v)) = (offers.get(*k), at(*vacancy)) else { continue };
            let better = |(w, id, _): (f64, u32, usize)| v.wage.total_cmp(&w).then(id.cmp(vacancy)).is_gt();
            if accepts(o, &v) && best.is_none_or(better) {
                best = Some((v.wage, *vacancy, *k));
            }
        }
        for (_, _, vacancy, k) in mine {
            if best.is_some_and(|(_, _, b)| b == *k) {
                if let Some(o) = offers.get(*k) {
                    hires.push(*o);
                }
            } else if let Some(v) = vacancies.get_mut(usize::try_from(*vacancy).unwrap_or(usize::MAX)) {
                v.open += 1;
            }
        }
    }
    hires
}

#[path = "hiring_tests.rs"]
mod tests;
