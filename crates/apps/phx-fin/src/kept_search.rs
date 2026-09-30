//! Today's hiring search (`phx_market::hiring::search`): the design point's vacancies standing by region, occupation
//! and skill, and a day's searchers each applying among those in its reach.

use std::collections::BTreeMap;

use phx_exec::Pool;
use phx_id::{PartyKey, Slot};
use phx_market::hiring::{Seeker, Standing, Vacancy, pick, search};
use phx_rand::uniform::{below_u64, open_unit};
use phx_rand::{Draws, Subject, SubjectTag};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{BASE, count, day_of, slots, wide};
use crate::measure::Measures;
use crate::{DayType, FinError};

/// The kinds of the fill: the households searching and the employers.
const HOUSEHOLDS: u8 = 1;
const EMPLOYERS: u8 = 2;
/// The occupation families, the ten major groups of the classification the world's labour uses; the skill levels a
/// job asks and a person has.
const OCCUPATIONS: u64 = 10;
const SKILLS: u64 = 4;
/// A vacancy pays up to this many a month, with up to this many jobs open; a searcher takes at least up to a half of
/// the top wage, sends one application a day and one more at even odds, and weighs a wage by its first power.
const WAGE: f64 = 5_000.0;
const OPEN: u64 = 3;
const RESERVATION: f64 = 0.5;
const APPLICATIONS_A_DAY: f64 = 1.5;
const WAGE_WEIGHT: f64 = 1.0;

/// The vacancies, their standing, and each day type's searchers.
#[derive(Debug, Default)]
pub struct Search {
    vacancies: Vec<Vacancy>,
    standing: Standing,
    seekers: BTreeMap<DayType, Vec<Seeker>>,
    regions: u64,
    streams: Option<Streams>,
    /// The last day's applications.
    pub applications: u64,
}

impl Search {
    /// `[store] vacancies`, each in a region, occupation and skill drawn for it, with a wage and jobs drawn for it.
    ///
    /// # Errors
    /// A design point without the counts.
    pub fn fill(&mut self, design: &Design, streams: &Streams) -> Result<u64, FinError> {
        let n = count(&design.store, "vacancies", "store")?;
        let regions = count(&design.store, "regions", "store")?;
        let mut d = streams.draws("kept.search", 0, 0);
        self.vacancies = (0..n)
            .map(|i| {
                Ok(Vacancy {
                    employer: PartyKey::new(EMPLOYERS, Slot::new(slots(i)?)),
                    region: slots(below_u64(&mut d, regions))?,
                    occupation: slots(below_u64(&mut d, OCCUPATIONS))?,
                    skill: slots(below_u64(&mut d, SKILLS))?,
                    wage: open_unit(&mut d) * WAGE,
                    open: slots(below_u64(&mut d, OPEN) + 1)?,
                })
            })
            .collect::<Result<_, FinError>>()?;
        self.standing = Standing::new(&self.vacancies);
        (self.regions, self.streams) = (regions, Some(*streams));
        Ok(n)
    }

    fn make(&self, day: DayType, searches: u64) -> Result<Vec<Seeker>, FinError> {
        let streams = self.streams.ok_or_else(|| FinError("searchers made before the fill".to_owned()))?;
        let mut d = streams.draws("kept.search", 1, day_of(day)?);
        (0..searches)
            .map(|i| {
                Ok(Seeker {
                    household: PartyKey::new(HOUSEHOLDS, Slot::new(slots(i)?)),
                    person: i,
                    subject: i,
                    region: slots(below_u64(&mut d, self.regions))?,
                    occupation: slots(below_u64(&mut d, OCCUPATIONS))?,
                    skill: slots(below_u64(&mut d, SKILLS))?,
                    experience: slots(below_u64(&mut d, OCCUPATIONS))?,
                    reservation: open_unit(&mut d) * RESERVATION * WAGE,
                })
            })
            .collect()
    }

    /// A day's `searches` searchers each sending its applications.
    ///
    /// # Errors
    /// A search before the fill.
    pub fn day(
        &mut self,
        day: DayType,
        counts: &BTreeMap<String, u64>,
        m: &mut Measures<'_>,
        pool: Option<&Pool>,
    ) -> Result<(), FinError> {
        // A day no one searches, as on a closed day, sends nothing.
        let Some(&searches) = counts.get("searches") else { return Ok(()) };
        if !self.seekers.contains_key(&day) {
            let made = self.make(day, searches)?;
            self.seekers.insert(day, made);
        }
        let streams = self.streams.ok_or_else(|| FinError("a search before the fill".to_owned()))?;
        let key = streams.key("kept.taste");
        let draws = move |subject: u64| Draws::new(key, Subject::new(SubjectTag::Party, subject), 0, 0);
        let choose = |_: &Seeker, reach: Vec<(u32, f64)>, units: Vec<f64>| pick(&reach, &units);
        let seekers = self.seekers.get(&day).map_or(&[][..], Vec::as_slice);
        let (vacancies, standing) = (&self.vacancies, &self.standing);
        let sent = m.read(BASE, "search", searches, || {
            search(pool, (vacancies, standing), seekers, (WAGE_WEIGHT, APPLICATIONS_A_DAY), (&draws, &choose))
        });
        self.applications = wide(sent.len());
        Ok(())
    }

    /// A digest of the vacancies, the same for any workers.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let words = self.vacancies.iter().flat_map(|v| [u64::from(v.region), u64::from(v.skill), v.wage.to_bits()]);
        words.fold(0, |a, w| phx_exec::mix::mix64(a ^ w))
    }

    /// The bytes the vacancies and their standing hold.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        let per = size_of::<Vacancy>() + size_of::<u32>();
        wide(self.vacancies.len() * per)
    }
}
