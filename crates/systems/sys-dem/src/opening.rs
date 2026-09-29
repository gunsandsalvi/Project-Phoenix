//! The opening's households: each region's persons drawn by age and sex from its country's declared distributions,
//! formed into households, and each household made an agent, region by region.

use if_pop::{EDUCATION, EDUCATION_UNRECORDED, FEMALE, HEALTH, MALE, REGION, SEX};
use phx_core::calendar::daycount::actual_days;
use phx_core::register::values::{Distribution, Table2, TypeSet};
use phx_core::{
    Household, OpeningCountry, Person, PrimDecl, Register, StreamDef, ValueType, apportion, opening_subject,
};
use phx_id::{CountryId, Date};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_pop::kind::PopKindDecl;
use phx_rand::float::{floor_to_i64, from_i64, len_u64};
use phx_rand::{AliasTable, Draws, below_u64, open_unit};

use crate::compose::{self, Member, Pick, Place, Pool, Rules, Type};
use crate::consts::{
    BANDS, CHILDREN_COLUMN, GAP_TYPES, MEMBER_COLUMNS, OLD_AGE, OLDER, OTHER, PARTNER_COLUMN, PERCENT, WORKING_AGE,
};
use crate::household::Role;
use crate::{CompositionStream, EducationStream, HealthStream, MeansStream, PersonsStream, Prims, RegionsStream};

/// A declared table's value at a point, its decimals undone.
pub(crate) fn value(table: &Table2, decl: &PrimDecl, row: i64, column: i64) -> f64 {
    let ValueType::Table2 { row_exp, column_exp, exp } = decl.value else {
        violation!(clause = "NUM.3", "a table read from a primitive that is not one");
    };
    let scaled = |x: i64, e: u8| {
        let Some(s) = i64::try_from(phx_num::price::pow10(e)).ok().and_then(|p| x.checked_mul(p)) else {
            capacity_exceeded!("a table's point in its decimals", i64::MAX, x);
        };
        s
    };
    let Ok(raw) = table.at(scaled(row, row_exp), scaled(column, column_exp)) else {
        violation!(clause = "GEN.2", "a household table that does not cover a point", row = row, column = column);
    };
    from_i64(raw) / libm::pow(phx_core::consts::DECIMAL_RADIX_F64, f64::from(exp))
}

/// A country's distributions as the draw reads them: its people by single age and sex, raked to its drawn shares
/// under 15 and over 65; the rules households are formed by; the chance of lasting disability by age and sex;
/// education by age band and sex; and the means' distributions.
struct Country {
    year: i32,
    year_days: u64,
    passed_days: u64,
    people: [Vec<f64>; 2],
    rules: Rules,
    disabled: Vec<[f64; 2]>,
    education_rows: Vec<i64>,
    education: [Vec<AliasTable>; 2],
    wealth: Distribution,
}

fn derived(c: &OpeningCountry, name: &str) -> f64 {
    let Some(v) = c.derived(name) else {
        violation!(clause = "GEN.15", "a derived value the households read is missing", country = c.id.get());
    };
    v
}

fn age(a: i64) -> u32 {
    let Ok(a) = u32::try_from(a) else { violation!(clause = "GEN.2", "an age below nought", age = a) };
    a
}

/// A country's people by single age and sex, the standard raked to its drawn shares under 15 and over 65: each band
/// scaled to its share, keeping the standard's shape within it.
struct People<'a> {
    table: &'a Table2,
    decl: &'a PrimDecl,
    scale: [f64; BANDS],
}

fn band(a: i64) -> usize {
    usize::from(a >= WORKING_AGE) + usize::from(a >= OLD_AGE)
}

const SEXES: [u32; 2] = [FEMALE, MALE];

impl<'a> People<'a> {
    fn of(p: &Prims, register: &'a Register, c: &OpeningCountry) -> People<'a> {
        let (table, decl) = (p.age_standard.get(register, c.id), p.age_standard.decl(register));
        let mut standard = [0.0_f64; BANDS];
        for a in table.rows() {
            for s in SEXES {
                if let Some(b) = standard.get_mut(band(*a)) {
                    *b += value(table, decl, *a, i64::from(s));
                }
            }
        }
        let under = derived(c, "GEN.share_under_15") / PERCENT;
        let over = derived(c, "GEN.share_65_plus") / PERCENT;
        let targets = [under, 1.0 - under - over, over];
        let mut scale = [0.0_f64; BANDS];
        for ((x, t), s) in scale.iter_mut().zip(targets).zip(standard) {
            *x = t / s;
        }
        People { table, decl, scale }
    }

    fn at(&self, a: i64, s: u32) -> f64 {
        let Some(scale) = self.scale.get(band(a)) else {
            violation!(clause = "GEN.2", "an age outside the bands the shares count", age = a);
        };
        value(self.table, self.decl, a, i64::from(s)) * scale
    }

    fn ages(&self) -> &[i64] {
        self.table.rows()
    }
}

/// Each woman's chance of a living child at each single age under the age of majority, by her age from the first the
/// kin table holds at or past majority.
fn chances(p: &Prims, register: &Register, id: CountryId, majority: u32) -> (u32, Vec<Vec<f64>>) {
    let (minor, decl) = (p.minor_children.get(register, id), p.minor_children.decl(register));
    if len_u64(minor.columns().len()) < u64::from(majority) {
        violation!(clause = "GEN.2", "an age of majority beyond the children's ages the kin table holds");
    }
    let ages: Vec<i64> = minor.rows().iter().copied().filter(|a| *a >= i64::from(majority)).collect();
    let Some(first) = ages.first().copied() else {
        violation!(clause = "GEN.2", "no mother of age in the kin table", country = id.get());
    };
    if ages.windows(2).any(|w| matches!(w, [a, b] if b - a != 1)) {
        violation!(clause = "GEN.2", "a kin table whose mothers' ages are not each year's", country = id.get());
    }
    let chances: Vec<Vec<f64>> =
        ages.iter().map(|a| (0..i64::from(majority)).map(|k| value(minor, decl, *a, k)).collect()).collect();
    if chances.iter().flatten().any(|m| !(0.0..1.0).contains(m)) {
        violation!(clause = "GEN.2", "a mother's chance of a child at one age outside a probability");
    }
    (age(first), chances)
}

/// The chance of each whole year of a partner's age above the woman's, from the drawn gap's types, and the first
/// year that has one.
fn gaps(gap: &Distribution) -> (i64, Vec<f64>) {
    let Ok(types) = TypeSet::build(gap, GAP_TYPES) else {
        violation!(clause = "NUM.4", "a partner gap that does not cut into types");
    };
    let scale = libm::pow(phx_core::consts::DECIMAL_RADIX_F64, f64::from(gap.exp));
    let years: Vec<(i64, u32)> = types
        .types()
        .iter()
        .map(|t| {
            let Some(y) = floor_to_i64(libm::round(from_i64(t.value) / scale)) else {
                violation!(clause = "GEN.2", "a partner's age gap that is no number");
            };
            (y, t.share_ppm)
        })
        .collect();
    // The types are the gap's quantiles in order, so the first and the last are its least and greatest.
    let (Some((first, _)), Some((last, _))) = (years.first().copied(), years.last().copied()) else {
        violation!(clause = "GEN.2", "a partner gap of no types");
    };
    let Ok(span) = usize::try_from(last - first) else { violation!(clause = "GEN.2", "a partner gap of no span") };
    let mut chances = vec![0.0_f64; span + 1];
    for (y, share) in years {
        if let Some(c) = usize::try_from(y - first).ok().and_then(|i| chances.get_mut(i)) {
            *c += f64::from(share);
        }
    }
    (first, chances)
}

/// The chance of lasting disability at each age by sex: the prevalence where it is observed, and below its first
/// band what the onset hazard gives from birth.
pub(crate) fn disabled(p: &Prims, register: &Register, id: CountryId, oldest: i64) -> Vec<[f64; 2]> {
    let (prevalence, pd) = (p.disability.get(register, id), p.disability.decl(register));
    let (onset, od) = (p.onset.get(register, id), p.onset.decl(register));
    let Some(first) = prevalence.rows().first().copied() else {
        violation!(clause = "GEN.2", "a disability table of no ages");
    };
    let chances: Vec<[f64; 2]> = (0..=oldest)
        .map(|a| {
            SEXES.map(|s| {
                if a >= first {
                    value(prevalence, pd, band_of(prevalence.rows(), a), i64::from(s))
                } else {
                    -libm::expm1(-value(onset, od, a, i64::from(s)) * from_i64(a))
                }
            })
        })
        .collect();
    if chances.iter().flatten().any(|q| !(0.0..=1.0).contains(q)) {
        violation!(clause = "GEN.2", "a chance of disability outside a probability", country = id.get());
    }
    chances
}

/// The first age of the band an age falls in, the last band open above.
fn band_of(firsts: &[i64], a: i64) -> i64 {
    let Some(first) = firsts.partition_point(|f| *f <= a).checked_sub(1).and_then(|i| firsts.get(i)) else {
        violation!(clause = "GEN.2", "an age below a table's first band", age = a);
    };
    *first
}

/// Education by age band for each sex, the bands' first ages the same for both.
fn education(p: &Prims, register: &Register, id: CountryId) -> (Vec<i64>, [Vec<AliasTable>; 2]) {
    let tables = p.education.map(|e| {
        let (t, d) = (e.get(register, id), e.decl(register));
        t.rows()
            .iter()
            .map(|r| AliasTable::new(&t.columns().iter().map(|col| value(t, d, *r, *col)).collect::<Vec<f64>>()))
            .collect::<Vec<AliasTable>>()
    });
    let [female, male] = p.education;
    let rows = female.get(register, id).rows().to_vec();
    if male.get(register, id).rows() != rows.as_slice() {
        violation!(clause = "GEN.2", "women's and men's education by different age bands");
    }
    (rows, tables)
}

/// Households by type at the country's drawn fertility and whom each type holds besides its head: every type, the
/// types with children and those without, each by its share; the types of one person and of a couple only; and
/// every type's share.
fn types(p: &Prims, register: &Register, c: &OpeningCountry) -> Types {
    let (table, decl) = (p.types.get(register, c.id), p.types.decl(register));
    let ln_fertility = libm::log(derived(c, "GEN.fertility"));
    let (Some(intercept), Some(slope)) = (table.columns().first(), table.columns().get(1)) else {
        violation!(clause = "GEN.2", "household types without an intercept and a slope");
    };
    let (m, md) = (p.members.shared(register), p.members.decl(register));
    let (mut all, mut with, mut without) = (Vec::new(), Vec::new(), Vec::new());
    for (r, index) in table.rows().iter().zip(0_usize..) {
        let share = libm::exp(value(table, decl, *r, *intercept) + value(table, decl, *r, *slope) * ln_fertility);
        let holds = |col: usize| {
            let Ok(at) = i64::try_from(col) else { violation!(clause = "GEN.2", "a members column beyond counting") };
            value(m, md, *r, at) > 0.0
        };
        let t = Type { index, holds: [PARTNER_COLUMN, CHILDREN_COLUMN, OLDER, OTHER].map(holds) };
        if t.children() {
            with.push((t, share));
        } else {
            without.push((t, share));
        }
        all.push((t, share));
    }
    if MEMBER_COLUMNS != m.columns().len() {
        violation!(clause = "GEN.2", "a members table of other columns than a type's members");
    }
    let only = |partner: bool| {
        let found =
            all.iter().map(|(t, _)| *t).find(|t| t.partner() == partner && !t.children() && !t.older() && !t.other());
        let Some(t) = found else {
            violation!(clause = "GEN.2", "no household type of one person or of a couple only");
        };
        t
    };
    let (alone, pair) = (only(false), only(true));
    Types { all: Pick::new(all), with: Pick::new(with), without: Pick::new(without), alone, pair }
}

/// The household types as the composition draws them.
struct Types {
    all: Pick<Type>,
    with: Pick<Type>,
    without: Pick<Type>,
    alone: Type,
    pair: Type,
}

/// A mother's expected living children at each single age from majority to the oldest, by her age from the first
/// the chances hold (`DEM.grown_children`).
fn grown(
    p: &Prims,
    register: &Register,
    id: CountryId,
    (majority, ages): (u32, u32),
    first: u32,
    mothers: usize,
) -> Vec<Vec<f64>> {
    let (table, decl) = (p.grown_children.get(register, id), p.grown_children.decl(register));
    if table.columns().first().is_none_or(|c| *c > i64::from(majority)) {
        violation!(clause = "GEN.2", "an age of majority before the grown children the kin table holds");
    }
    (first..)
        .take(mothers)
        .map(|mother| (majority..ages).map(|k| value(table, decl, i64::from(mother), i64::from(k))).collect())
        .collect()
}

impl Country {
    fn of(p: &Prims, register: &Register, c: &OpeningCountry, date: phx_id::Date) -> Country {
        let id = c.id;
        let people = People::of(p, register, c);
        let Some(oldest) = people.ages().last().copied() else {
            violation!(clause = "GEN.2", "an age standard of no ages");
        };
        if people.ages().iter().zip(0_i64..).any(|(a, i)| *a != i) {
            violation!(clause = "GEN.2", "an age standard not of each single age from nought", country = id.get());
        }
        let Ok(majority) = u32::try_from(p.majority.get(register, id).get()) else {
            violation!(clause = "POP.16", "an age of majority beyond counting", country = id.get());
        };
        let (first_mother, chances) = chances(p, register, id, majority);
        let (first_gap, gaps) = gaps(p.partner_gap.get(register, id));
        let Types { all, with: with_children, without, alone, pair } = types(p, register, c);
        let grown = grown(p, register, id, (majority, age(oldest) + 1), first_mother, chances.len());
        let (education_rows, education) = education(p, register, id);
        let rules = Rules {
            majority,
            ages: age(oldest) + 1,
            old_age: age(OLD_AGE),
            first_mother,
            chances,
            grown,
            first_gap,
            gaps,
            single_fathers: p.single_fathers.get(register, id).to_f64(),
            all,
            with_children,
            without,
            alone,
            pair,
        };
        let (year_days, passed_days) = year_days(date);
        Country {
            year: date.year(),
            year_days,
            passed_days,
            people: SEXES.map(|s| people.ages().iter().map(|a| people.at(*a, s)).collect()),
            rules,
            disabled: disabled(p, register, id, oldest),
            education_rows,
            education,
            wealth: p.wealth.get(register, id).clone(),
        }
    }

    fn disabled(&self, p: &Member, d: &mut Draws) -> u32 {
        let Some(q) = self.disabled.get(index(p.age)).and_then(|q| q.get(index(p.sex))) else {
            violation!(clause = "GEN.2", "a person older than the disability table", age = p.age);
        };
        u32::from(open_unit(d) < *q)
    }

    fn education(&self, p: &Member, d: &mut Draws) -> u32 {
        let row = self.education_rows.partition_point(|r| *r <= i64::from(p.age));
        let table = self.education.get(index(p.sex)).and_then(|rows| row.checked_sub(1).and_then(|r| rows.get(r)));
        let Some(table) = table else {
            violation!(clause = "GEN.2", "an adult younger than the education table", age = p.age);
        };
        let Ok(level) = u32::try_from(table.draw(d)) else {
            violation!(clause = "GEN.2", "an education level beyond counting");
        };
        level
    }

    /// A person's birth date from its age at the snapshot: its birthday falls on a day drawn evenly over the year,
    /// and the person was born this year less its age if that day has come, a year earlier if not.
    fn born(&self, p: &Member, d: &mut Draws) -> Date {
        let day_of_year = below_u64(d, self.year_days);
        let passed = day_of_year <= self.passed_days;
        let year = i64::from(self.year) - i64::from(p.age) - i64::from(!passed);
        let Ok(year) = i32::try_from(year) else { violation!(clause = "POP.1", "a birth year beyond the calendar") };
        let first = start(year);
        let Ok(offset) = i64::try_from(day_of_year) else { violation!(clause = "TIME.2", "a day beyond its year") };
        let born = phx_core::calendar::days_after(first, offset);
        // A birthday drawn on the 366th day falls on the last day of a common birth year.
        if born.year() == year { born } else { phx_core::calendar::days_after(start(year + 1), -1) }
    }
}

fn start(y: i32) -> Date {
    let Some(d) = Date::new(y, 1, 1) else { violation!(clause = "TIME.2", "a year with no first day") };
    d
}

/// The days of a date's year, and those of it before the date.
fn year_days(date: Date) -> (u64, u64) {
    let (first, next) = (start(date.year()), start(date.year() + 1));
    let whole = |n: i64| {
        let Ok(d) = u64::try_from(n) else {
            violation!(clause = "TIME.2", "a date before its year's first day", days = n);
        };
        d
    };
    (whole(actual_days(first, next)), whole(actual_days(first, date)))
}

fn index(v: u32) -> usize {
    phx_rand::float::index(u64::from(v))
}

/// A drawn person as its household holds it.
fn person(country: &Country, m: &Member, (d, health, school): (&mut Draws, &mut Draws, &mut Draws)) -> Person {
    let role = match m.place {
        Place::Head => Role::Head,
        Place::Partner => Role::Partner,
        Place::Adult => Role::Adult,
        Place::Child => Role::Child,
    };
    let disabled = country.disabled(m, health);
    let education = if m.place == Place::Child { EDUCATION_UNRECORDED } else { country.education(m, school) };
    Person {
        role: role.name(),
        born: country.born(m, d),
        attrs: vec![(SEX.name, m.sex), (HEALTH.name, disabled), (EDUCATION.name, education)],
        gone: false,
    }
}

/// A household drawn in a region, before the books hold it: the subject its draws are keyed by, what formed it, its
/// persons and the wealth it was drawn, as a multiple of the mean.
#[derive(Debug)]
pub struct Formed {
    pub subject: phx_rand::Subject,
    pub raised: bool,
    pub kind: usize,
    pub h: Household,
    pub wealth: f64,
}

/// A country's households drawn region by region, its persons apportioned over its regions by their land: each
/// region with its households in their order.
#[clause("GEN.2", "GEN.3", "POP.1", "POP.2", "PTY.2")]
#[must_use]
pub fn draw_country(
    p: &Prims,
    register: &Register,
    (opening_ctx, date): (&phx_core::OpeningCtx<'_>, phx_id::Date),
    c: &OpeningCountry,
    kind: &PopKindDecl,
) -> Vec<(u32, Vec<Formed>)> {
    let country = Country::of(p, register, c, date);
    let tiles: Vec<u64> = c.regions.iter().map(|(_, tiles)| len_u64(tiles.len())).collect();
    let mut lot = opening_ctx.draws(&RegionsStream::DECL, opening_subject(u32::from(c.id.get()), 0));
    let shares = apportion(c.people, &tiles, &mut lot);
    c.regions
        .iter()
        .map(|(r, _)| *r)
        .zip(shares)
        .map(|(region, people)| (region, draw_region(&country, opening_ctx, (region, people), kind)))
        .collect()
}

/// One region's households formed from its persons, each household's draws keyed by the region and
/// its place in it, so a region is drawn alone; with the persons it counts disabled and by age band.
fn draw_region(
    country: &Country,
    opening_ctx: &phx_core::OpeningCtx<'_>,
    (region, people): (u32, u64),
    kind: &PopKindDecl,
) -> Vec<Formed> {
    let mut lot = opening_ctx.draws(&PersonsStream::DECL, opening_subject(region, 0));
    let [women, men] = &country.people;
    let weights: Vec<f64> = women.iter().chain(men).copied().collect();
    let mut counts = compose::apportion(people, &weights, &mut lot);
    let men_counts = counts.split_off(women.len());
    let mut pool = Pool::of(counts, men_counts);
    let region_at = kind.attr(REGION.name).unwrap_or_else(|| {
        violation!(clause = "REP.41", "a household kind without its region");
    });
    let (mut ordinal, mut members, mut out) = (0_u32, Vec::new(), Vec::new());
    loop {
        let subject = opening_subject(region, ordinal);
        let mut d = opening_ctx.draws(&CompositionStream::DECL, subject);
        let Some(formed) = compose::household(&mut pool, &country.rules, &mut d, &mut members) else { break };
        let mut health = opening_ctx.draws(&HealthStream::DECL, subject);
        let mut school = opening_ctx.draws(&EducationStream::DECL, subject);
        let mut persons = Vec::with_capacity(members.len());
        for m in &members {
            let p = person(country, m, (&mut d, &mut health, &mut school));
            persons.push(p);
        }
        let mut attrs = vec![0_u32; kind.attrs.len()];
        if let Some(r) = attrs.get_mut(region_at) {
            *r = region;
        }
        let names: Vec<(&'static str, u32)> = kind.attrs.iter().zip(&attrs).map(|(a, v)| (a.item.name, *v)).collect();
        let h = Household { attrs: names, persons, positions: Vec::new() };
        let mut means = opening_ctx.draws(&MeansStream::DECL, subject);
        let wealth = country.wealth.draw(&mut means) / country.wealth.mean();
        out.push(Formed { subject, raised: formed.raised, kind: formed.kind.index, h, wealth });
        let Some(next) = ordinal.checked_add(1) else {
            capacity_exceeded!("households of a region", u32::MAX, ordinal);
        };
        ordinal = next;
    }
    out
}
