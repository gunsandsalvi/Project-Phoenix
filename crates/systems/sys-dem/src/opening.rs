//! The opening's households: each region's persons drawn by age and sex from its country's declared distributions,
//! formed into households, and each household made an agent, region by region; under twins one agent for every
//! `REP.multiplicity` households.

use if_pop::{EDUCATION, EDUCATION_UNRECORDED, FEMALE, HEALTH, HOUSEHOLD, MALE, REGION, SEX};
use phx_core::calendar::daycount::actual_days;
use phx_core::register::values::{Distribution, Table2, TypeSet};
use phx_core::{
    Adjustment, CONTRACTS, Contribution, DECLARATIONS, Household, Opening, OpeningCountry, OpeningPhase, Person,
    PrimDecl, Register, StreamDef, ValueType, apportion, opening_subject,
};
use phx_id::{CountryId, Date};
use phx_ledger::books::Books;
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_ledger::opening::key;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_pop::kind::PopKindDecl;
use phx_pop::person::pack;
use phx_pop::population::Population;
use phx_rand::float::{floor_to_i64, from_i64, from_u64, len_u64};
use phx_rand::{AliasTable, Draws, below_u64, open_unit};
use phx_store::SystemBacking;

use crate::compose::{self, Member, Pick, Place, Pool, Rules, Type};
use crate::consts::{
    BANDS, CHILDREN_COLUMN, GAP_TYPES, MEMBER_COLUMNS, OLD_AGE, OLDER, OTHER, PARTNER_COLUMN, PERCENT, REGION_WAVE,
    WORKING_AGE,
};
use crate::household::Role;
use crate::lines::Drawer;
use crate::{CompositionStream, EducationStream, HealthStream, MeansStream, PersonsStream, Prims, RegionsStream};

const HOUSEHOLDS: &str = "DEM.households";

/// What the households' opening instructions are for: capital on both sides, since they open the books.
const REASON: ReasonDecl = ReasonDecl {
    name: "DEM opening",
    order: 0,
    paid: Effect::Equity,
    received: Effect::Equity,
    held: phx_num::Missing::Absent,
};

/// The households' declarations in the books: their opening's reason.
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "household declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let _ = phx_ledger::books::of(opening).ledger.reasons.declare(REASON);
    }
}

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
/// education by age band and sex; and the types' shares, for the report.
struct Country {
    year: i32,
    year_days: u64,
    passed_days: u64,
    people: [Vec<f64>; 2],
    rules: Rules,
    disabled: Vec<[f64; 2]>,
    education_rows: Vec<i64>,
    education: [Vec<AliasTable>; 2],
    shares: Vec<f64>,
    wealth: Distribution,
    income: Distribution,
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
fn disabled(p: &Prims, register: &Register, id: CountryId, oldest: i64) -> Vec<[f64; 2]> {
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

/// Households by type at the country's drawn fertility and whom each type holds besides its head: the types with
/// children and those without, each by its share, and every type's share.
fn types(p: &Prims, register: &Register, c: &OpeningCountry) -> (Pick<Type>, Pick<Type>, Vec<f64>) {
    let (table, decl) = (p.types.get(register, c.id), p.types.decl(register));
    let ln_fertility = libm::log(derived(c, "GEN.fertility"));
    let (Some(intercept), Some(slope)) = (table.columns().first(), table.columns().get(1)) else {
        violation!(clause = "GEN.2", "household types without an intercept and a slope");
    };
    let (m, md) = (p.members.shared(register), p.members.decl(register));
    let mut with = Vec::new();
    let mut without = Vec::new();
    let mut shares = Vec::new();
    for (r, index) in table.rows().iter().zip(0_usize..) {
        let share = libm::exp(value(table, decl, *r, *intercept) + value(table, decl, *r, *slope) * ln_fertility);
        let holds = |col: usize| {
            let Ok(at) = i64::try_from(col) else { violation!(clause = "GEN.2", "a members column beyond counting") };
            value(m, md, *r, at) > 0.0
        };
        let t = Type { index, partner: holds(PARTNER_COLUMN), older: holds(OLDER), other: holds(OTHER) };
        if holds(CHILDREN_COLUMN) {
            with.push((t, share));
        } else {
            without.push((t, share));
        }
        shares.push(share);
    }
    if MEMBER_COLUMNS != m.columns().len() {
        violation!(clause = "GEN.2", "a members table of other columns than a type's members");
    }
    let total: f64 = shares.iter().sum();
    (Pick::new(with), Pick::new(without), shares.into_iter().map(|s| s / total).collect())
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
        let (with_children, without, shares) = types(p, register, c);
        let (education_rows, education) = education(p, register, id);
        let rules = Rules {
            majority,
            ages: age(oldest) + 1,
            old_age: age(OLD_AGE),
            first_mother,
            chances,
            first_gap,
            gaps,
            with_children,
            without,
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
            shares,
            wealth: p.wealth.get(register, id).clone(),
            income: p.income.get(register, id).clone(),
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

/// Each country's households, drawn region by region: the country's people apportioned among its regions by their
/// land, one in `REP.multiplicity` of each region's persons drawn by single age and sex, and households formed from them
/// until none is left — families while a child is left, then households of adults — each person's health drawn by age
/// and sex and each adult's education by age band and sex. Each household is an agent of that many twins.
#[clause("GEN.2", "GEN.3", "POP.1", "POP.2", "REP.25", "REP.26", "REP.40")]
#[derive(Debug)]
pub struct Households {
    pub prims: Prims,
}

/// What the draw made, for the report: households, persons, persons in each of the derived shares' age bands,
/// persons disabled, households of each type drawn, and children raised by an adult who is not their mother; each
/// counted once for the twins its agent stands for.
#[derive(Clone, Debug, Default)]
struct Tally {
    households: u64,
    persons: u64,
    bands: [u64; BANDS],
    disabled: u64,
    types: Vec<u64>,
    raised: u64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.households += other.households;
        self.persons += other.persons;
        self.disabled += other.disabled;
        self.raised += other.raised;
        for (a, b) in self.bands.iter_mut().zip(other.bands) {
            *a += b;
        }
        if self.types.len() < other.types.len() {
            self.types.resize(other.types.len(), 0);
        }
        for (a, b) in self.types.iter_mut().zip(&other.types) {
            *a += b;
        }
    }
}

/// Where a region's households go: the books they open in, the population's kind and table, the day and the twins
/// each household agent stands for.
struct Into<'a> {
    books: &'a mut Books,
    kind: &'a PopKindDecl,
    at: usize,
    day: phx_id::Day,
    twins: u32,
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
/// persons and the means it was drawn.
struct Formed {
    subject: phx_rand::Subject,
    raised: bool,
    kind: usize,
    h: Household,
    drawn: (f64, f64),
}

/// One region's households formed from one twin-th of its persons, each household's draws keyed by the region and
/// its place in it, so a region is drawn alone; with the persons it counts disabled and by age band.
fn draw_region(
    country: &Country,
    opening_ctx: &phx_core::OpeningCtx<'_>,
    (region, people): (u32, u64),
    (kind, twins): (&PopKindDecl, u32),
) -> (Vec<Formed>, Tally) {
    let twins = u64::from(twins);
    let mut lot = opening_ctx.draws(&PersonsStream::DECL, opening_subject(region, 0));
    let [women, men] = &country.people;
    let weights: Vec<f64> = women.iter().chain(men).copied().collect();
    let mut counts = compose::apportion(people / twins, &weights, &mut lot);
    let men_counts = counts.split_off(women.len());
    let mut pool = Pool::of(counts, men_counts);
    let mut tally = Tally { types: vec![0; country.shares.len()], ..Tally::default() };
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
            tally.disabled += twins * u64::from(p.attr(HEALTH.name) == Some(if_pop::DISABLED));
            if let Some(b) = tally.bands.get_mut(band(i64::from(m.age))) {
                *b += twins;
            }
            persons.push(p);
        }
        let mut attrs = vec![0_u32; kind.attrs.len()];
        if let Some(r) = attrs.get_mut(region_at) {
            *r = region;
        }
        let names: Vec<(&'static str, u32)> = kind.attrs.iter().zip(&attrs).map(|(a, v)| (a.item.name, *v)).collect();
        let h = Household { attrs: names, persons };
        let mut means = opening_ctx.draws(&MeansStream::DECL, subject);
        let drawn = (
            country.wealth.draw(&mut means) / country.wealth.mean(),
            country.income.draw(&mut means) / country.income.mean(),
        );
        out.push(Formed { subject, raised: formed.raised, kind: formed.kind.index, h, drawn });
        let Some(next) = ordinal.checked_add(1) else {
            capacity_exceeded!("households of a region", u32::MAX, ordinal);
        };
        ordinal = next;
    }
    (out, tally)
}

/// A region's drawn households made agents in their order, each with its lines drawn onto the books.
fn book_region(
    formed: Vec<Formed>,
    opening_ctx: &phx_core::OpeningCtx<'_>,
    into: &mut Into<'_>,
    drawer: &mut Drawer,
    tally: &mut Tally,
) {
    let twins = u64::from(into.twins);
    for Formed { subject, raised, kind, mut h, drawn } in formed {
        let held = drawer.household(into.books, &mut h, ((opening_ctx, subject), drawn));
        let attrs: Vec<u32> = into.kind.attrs.iter().map(|a| h.attr(a.item.name)).collect();
        let words: Vec<u64> = h.persons.iter().map(|p| pack(into.kind, p)).collect();
        let (tables, directory, space) = into.books.parties.cells_mut();
        let table = Population::table_mut::<SystemBacking>(tables, into.at);
        let (slot, party) =
            phx_pop::table::begin(table, directory, space, (into.day, phx_core::Weight::new(into.twins), &attrs));
        table.set_persons(slot, &words);
        table.set_attachments(slot, &held.attachments);
        drawer.agent(party, twins, held);
        tally.persons += twins * len_u64(h.persons.len());
        tally.households += twins;
        tally.raised += twins * u64::from(raised);
        if let Some(t) = tally.types.get_mut(kind) {
            *t += twins;
        }
    }
}

/// The household kind's place among the population's kinds.
fn household_kind(population: &Population) -> usize {
    let Some(k) = population.kinds.iter().position(|k| k.decl.kind == HOUSEHOLD) else {
        violation!(clause = "POP.2", "a world that keeps no household kind");
    };
    k
}

impl Contribution for Households {
    fn name(&self) -> &'static str {
        "households"
    }
    fn phase(&self) -> OpeningPhase {
        CONTRACTS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[HOUSEHOLDS]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[HOUSEHOLDS]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let Opening { ctx, day, date, calendar, register, countries, report, books, population, attachments } = opening;
        let Some(books) = books.downcast_mut::<Books>() else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's books");
        };
        let Some(population) = population.downcast_mut::<Population>() else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's population");
        };
        let at = household_kind(population);
        let twins = population.representation.multiplicity;
        let reason = books.ledger.reasons.named(REASON.name);
        let Some(kind) = population.kinds.get(at).map(|k| k.decl.clone()) else {
            violation!(clause = "POP.2", "a world that keeps no household kind");
        };
        for c in *countries {
            let country = Country::of(&self.prims, register, c, *date);
            let mut drawer = Drawer::new(attachments, books, register, (calendar, *day), c, twins);
            let tiles: Vec<u64> = c.regions.iter().map(|(_, tiles)| len_u64(tiles.len())).collect();
            let mut lot = ctx.draws(&RegionsStream::DECL, opening_subject(u32::from(c.id.get()), 0));
            let shares = apportion(c.people, &tiles, &mut lot);
            let mut tally = Tally::default();
            let mut whole = 0_u64;
            let regions: Vec<(u32, u64)> = c.regions.iter().map(|(r, _)| *r).zip(shares).collect();
            // A wave of regions drawn at once on the books' workers, then booked in the regions' order.
            for wave in regions.chunks(REGION_WAVE) {
                let drawn =
                    books.on_pool(wave.len(), |i| wave.get(i).map(|r| draw_region(&country, ctx, *r, (&kind, twins))));
                for ((_, people), (formed, counted)) in wave.iter().zip(drawn.into_iter().flatten()) {
                    whole += people - people % u64::from(twins);
                    tally.add(&counted);
                    let mut into = Into { books, kind: &kind, at, day: *day, twins };
                    book_region(formed, ctx, &mut into, &mut drawer, &mut tally);
                }
            }
            let mut lot = ctx.draws(&RegionsStream::DECL, opening_subject(u32::from(c.id.get()), 1));
            drawer.close(books, (register, reason), &mut lot, report);
            if tally.households == 0 {
                violation!(clause = "GEN.3", "a country whose people make no household", country = c.id.get());
            }
            if whole != c.people {
                report.adjustments.push(Adjustment {
                    what: format!("country {}: persons, each region's a whole number for {twins} twins", c.id.get()),
                    drawn: i128::from(c.people),
                    set: i128::from(whole),
                });
            }
            population.count(at, (tally.households, 0), (tally.persons, 0));
            report.distributions.push((key(HOUSEHOLDS, c.id), describe(c, &country, &tally, twins)));
        }
    }
}

fn describe(c: &OpeningCountry, country: &Country, t: &Tally, twins: u32) -> String {
    let share = |n: u64| PERCENT * from_u64(n) / from_u64(t.persons);
    let [under, _, over] = t.bands;
    let types: Vec<String> = t
        .types
        .iter()
        .zip(&country.shares)
        .map(|(n, s)| format!("{:.1}% ({:.1}%)", PERCENT * from_u64(*n) / from_u64(t.households), PERCENT * s))
        .collect();
    format!(
        "country {}: {} households of {} persons (people {}, {:.2} a household) as agents of {twins} twins; {:.1}% \
         under 15 (GEN.share_under_15 {:.1}%), {:.1}% 65 and over (GEN.share_65_plus {:.1}%), {:.1}% disabled; \
         households by type as drawn (DEM.household_types at GEN.fertility) {}; {} children raised by an adult not \
         their mother. Persons by age and sex (DEM.age_standard raked to GEN.share_under_15 and GEN.share_65_plus), \
         formed into families by mothers' chances of children (DEM.minor_children), partners (DEM.partner_age_gap) \
         and whom each type holds (DEM.household_members); health (DEM.disability_prevalence, DEM.disability_onset) \
         and education (DEM.education_female, DEM.education_male)",
        c.id.get(),
        t.households,
        t.persons,
        c.people,
        from_u64(t.persons) / from_u64(t.households),
        share(under),
        derived(c, "GEN.share_under_15"),
        share(over),
        derived(c, "GEN.share_65_plus"),
        share(t.disabled),
        types.join(", "),
        t.raised,
    )
}
