//! The opening's households: each region's people drawn household by household from its country's declared
//! distributions, gathered by key and landed as cells, region by region.

use std::collections::BTreeMap;

use if_pop::consts::FIRST_BIRTH_YEAR;
use if_pop::{
    ADULT_COUNTS, ADULT_GROUPS, CHILD_COUNTS, CHILD_GROUPS, FEMALE, HEAD_AGE, HOUSEHOLD, LIFE, MALE, PARTNER_AGE,
    PARTNERS, REGION, SCHOOLING,
};
use phx_core::register::values::{Distribution, Partition, Table2};
use phx_core::{
    Contribution, Opening, OpeningCountry, OpeningPhase, PARTIES, PrimDecl, Register, StreamDef, ValueType, Weight,
    apportion, joint, opening_subject,
};
use phx_id::{CountryId, LineId};
use phx_ledger::algebra::Side;
use phx_ledger::books::Books;
use phx_ledger::opening::key;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_pop::check::LineKinks;
use phx_pop::key::KeyRecord;
use phx_pop::kind::PopKindDecl;
use phx_pop::landing::{Drawn, TenB, land_drawn};
use phx_pop::population::{PopKind, Population};
use phx_pop::profile::{Profile, ProfileLayout};
use phx_rand::float::{floor_to_i64, from_i64, from_u64, len_u64};
use phx_rand::{AliasTable, Draws, open_unit};
use phx_store::SystemBacking;

use crate::consts::{BANDS, CHILDREN, MEMBER_COLUMNS, OLD_AGE, OLDER, OTHER, PARTNER, PERCENT, WORKING_AGE};
use crate::{CompositionStream, EducationStream, HealthStream, Prims, RegionsStream};

const HOUSEHOLDS: &str = "DEM.households";

/// A declared table's value at a point, its decimals undone.
fn value(table: &Table2, decl: &PrimDecl, row: i64, column: i64) -> f64 {
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

/// Outcomes drawn with probability proportional to their weights.
struct Pick<T> {
    table: AliasTable,
    outcomes: Vec<T>,
}

impl<T: Copy> Pick<T> {
    fn new(weighted: Vec<(T, f64)>) -> Pick<T> {
        if weighted.iter().all(|(_, w)| *w <= 0.0) {
            violation!(clause = "GEN.2", "a household draw with nothing to draw from");
        }
        let weights: Vec<f64> = weighted.iter().map(|(_, w)| *w).collect();
        Pick { table: AliasTable::new(&weights), outcomes: weighted.into_iter().map(|(t, _)| t).collect() }
    }

    fn draw(&self, d: &mut Draws) -> T {
        let Some(t) = self.outcomes.get(self.table.draw(d)) else {
            violation!(clause = "CHN.2", "an alias draw beyond its outcomes");
        };
        *t
    }
}

/// A person as drawn: the role's place in the household, age in whole years at the snapshot, and sex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Head,
    Partner,
    Adult,
    Child,
}

#[derive(Clone, Copy, Debug)]
struct Person {
    place: Place,
    age: u32,
    sex: u32,
}

/// A country's distributions as the draw reads them: its people by age and sex, raked to its drawn shares under 15
/// and over 65; adults, women, mothers of children under the age of majority and older persons to draw from; each
/// mother's chance of a living child at each single age; the chance of lasting disability by age and sex; education
/// by age band and sex; households by type at its drawn fertility, and whom each type holds.
struct Country<'a> {
    year: i32,
    oldest: u32,
    majority: u32,
    adults: Pick<(u32, u32)>,
    women: Pick<u32>,
    mothers: Pick<u32>,
    older: Pick<(u32, u32)>,
    first_mother: u32,
    child_chance: Vec<Vec<f64>>,
    disabled: Vec<[f64; 2]>,
    education_rows: Vec<i64>,
    education: [Vec<AliasTable>; 2],
    types: Pick<usize>,
    members: Vec<[bool; MEMBER_COLUMNS]>,
    gap: &'a Distribution,
    male_share: f64,
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

/// Mothers of children under the age of majority, each drawn by her age's women and her chance of a child, and her
/// chance of a living child at each single age.
fn mothers(
    p: &Prims,
    register: &Register,
    id: CountryId,
    people: &People<'_>,
    majority: u32,
) -> (Pick<u32>, u32, Vec<Vec<f64>>) {
    let (minor, decl) = (p.minor_children.get(register, id), p.minor_children.decl(register));
    if len_u64(minor.columns().len()) < u64::from(majority) {
        violation!(clause = "GEN.2", "an age of majority beyond the children's ages the kin table holds");
    }
    let ages: Vec<i64> = minor.rows().iter().copied().filter(|a| *a >= i64::from(majority)).collect();
    let Some(first) = ages.first().copied() else {
        violation!(clause = "GEN.2", "no mother of age in the kin table", country = id.get());
    };
    let chances: Vec<Vec<f64>> =
        ages.iter().map(|a| (0..i64::from(majority)).map(|k| value(minor, decl, *a, k)).collect()).collect();
    if chances.iter().flatten().any(|m| !(0.0..1.0).contains(m)) {
        violation!(clause = "GEN.2", "a mother's chance of a child at one age outside a probability");
    }
    let weighted = ages
        .iter()
        .zip(&chances)
        .map(|(a, ms)| {
            let none: f64 = ms.iter().map(|m| 1.0 - m).product();
            (age(*a), people.at(*a, FEMALE) * (1.0 - none))
        })
        .collect();
    (Pick::new(weighted), age(first), chances)
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

/// Households by type at the country's drawn fertility, and whom each type holds besides its head.
fn types(p: &Prims, register: &Register, c: &OpeningCountry) -> (Pick<usize>, Vec<[bool; MEMBER_COLUMNS]>) {
    let (table, decl) = (p.types.get(register, c.id), p.types.decl(register));
    let ln_fertility = libm::log(derived(c, "GEN.fertility"));
    let (Some(intercept), Some(slope)) = (table.columns().first(), table.columns().get(1)) else {
        violation!(clause = "GEN.2", "household types without an intercept and a slope");
    };
    let weighted = table
        .rows()
        .iter()
        .zip(0_usize..)
        .map(|(r, i)| {
            (i, libm::exp(value(table, decl, *r, *intercept) + value(table, decl, *r, *slope) * ln_fertility))
        })
        .collect();
    let (m, md) = (p.members.shared(register), p.members.decl(register));
    let members = table
        .rows()
        .iter()
        .map(|r| {
            let mut row = [false; MEMBER_COLUMNS];
            for (col, at) in row.iter_mut().zip(0_i64..) {
                *col = value(m, md, *r, at) > 0.0;
            }
            row
        })
        .collect();
    (Pick::new(weighted), members)
}

impl<'a> Country<'a> {
    fn of(p: &Prims, register: &'a Register, c: &OpeningCountry, year: i32) -> Country<'a> {
        let id = c.id;
        let people = People::of(p, register, c);
        let Some(oldest) = people.ages().last().copied() else {
            violation!(clause = "GEN.2", "an age standard of no ages");
        };
        let Ok(majority) = u32::try_from(p.majority.get(register, id).get()) else {
            violation!(clause = "POP.16", "an age of majority beyond counting", country = id.get());
        };
        let adult_ages: Vec<i64> = people.ages().iter().copied().filter(|a| *a >= i64::from(majority)).collect();
        let adults =
            Pick::new(adult_ages.iter().flat_map(|a| SEXES.map(|s| ((age(*a), s), people.at(*a, s)))).collect());
        let women = Pick::new(adult_ages.iter().map(|a| (age(*a), people.at(*a, FEMALE))).collect());
        let older = Pick::new(
            people
                .ages()
                .iter()
                .filter(|a| **a >= OLD_AGE)
                .flat_map(|a| SEXES.map(|s| ((age(*a), s), people.at(*a, s))))
                .collect(),
        );
        let (mothers, first_mother, child_chance) = mothers(p, register, id, &people, majority);
        let (education_rows, education) = education(p, register, id);
        let (types, members) = types(p, register, c);
        let ratio = p.sex_ratio.get(register, id).to_f64();
        Country {
            year,
            oldest: age(oldest),
            majority,
            adults,
            women,
            mothers,
            older,
            first_mother,
            child_chance,
            disabled: disabled(p, register, id, oldest),
            education_rows,
            education,
            types,
            members,
            gap: p.partner_gap.get(register, id),
            male_share: ratio / (PERCENT + ratio),
        }
    }

    /// A woman's partner's age: hers and the drawn gap, redrawn until it is an adult's age the tables hold.
    fn partner_of(&self, woman: u32, d: &mut Draws) -> u32 {
        loop {
            let Some(gap) = floor_to_i64(libm::round(self.gap.draw(d))) else {
                violation!(clause = "GEN.2", "a partner's age gap that is no number");
            };
            let a = i64::from(woman) + gap;
            if a >= i64::from(self.majority) && a <= i64::from(self.oldest) {
                return age(a);
            }
        }
    }

    /// A mother's children under the age of majority, by single age: each age a child with its chance, the whole
    /// set redrawn until it holds one, so a mother drawn as one has a child.
    fn children_of(&self, mother: u32, d: &mut Draws, out: &mut Vec<Person>) {
        let Some(chances) = self.child_chance.get(phx_rand::float::index(u64::from(mother - self.first_mother))) else {
            violation!(clause = "GEN.2", "a mother older than the kin table", age = mother);
        };
        let start = out.len();
        while out.len() == start {
            for (k, m) in (0_u32..).zip(chances) {
                if open_unit(d) < *m {
                    let sex = if open_unit(d) < self.male_share { MALE } else { FEMALE };
                    out.push(Person { place: Place::Child, age: k, sex });
                }
            }
        }
    }

    /// One household's persons: its type, and whom the type holds besides the head.
    fn household(&self, d: &mut Draws, out: &mut Vec<Person>) {
        out.clear();
        let t = self.types.draw(d);
        let Some(m) = self.members.get(t) else { violation!(clause = "GEN.2", "a household type with no members") };
        let has = |col: usize| m.get(col).copied().unwrap_or(false);
        if has(CHILDREN) {
            let mother = self.mothers.draw(d);
            self.children_of(mother, d, out);
            if has(PARTNER) {
                let man = self.partner_of(mother, d);
                out.push(Person { place: Place::Head, age: man, sex: MALE });
                out.push(Person { place: Place::Partner, age: mother, sex: FEMALE });
            } else {
                out.push(Person { place: Place::Head, age: mother, sex: FEMALE });
            }
        } else if has(PARTNER) {
            let woman = self.women.draw(d);
            let man = self.partner_of(woman, d);
            out.push(Person { place: Place::Head, age: man, sex: MALE });
            out.push(Person { place: Place::Partner, age: woman, sex: FEMALE });
        } else {
            let (a, sex) = self.adults.draw(d);
            out.push(Person { place: Place::Head, age: a, sex });
        }
        if has(OLDER) {
            let (a, sex) = self.older.draw(d);
            out.push(Person { place: Place::Adult, age: a, sex });
        }
        if has(OTHER) {
            let (a, sex) = self.adults.draw(d);
            out.push(Person { place: Place::Adult, age: a, sex });
        }
    }

    fn disabled(&self, p: &Person, d: &mut Draws) -> u32 {
        let Some(q) = self.disabled.get(phx_rand::float::index(u64::from(p.age))).and_then(|q| q.get(index(p.sex)))
        else {
            violation!(clause = "GEN.2", "a person older than the disability table", age = p.age);
        };
        u32::from(open_unit(d) < *q)
    }

    fn education(&self, p: &Person, d: &mut Draws) -> u32 {
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

    fn birth_year(&self, p: &Person) -> u32 {
        let Ok(born) = u32::try_from(i64::from(self.year) - i64::from(p.age) - i64::from(FIRST_BIRTH_YEAR)) else {
            violation!(clause = "POP.1", "a person born before the first birth year the profile holds", age = p.age);
        };
        born
    }
}

fn index(v: u32) -> usize {
    phx_rand::float::index(u64::from(v))
}

/// Where the household kind keeps each of the persons' key attributes and profile groups.
struct Layout {
    region: usize,
    head_age: usize,
    partners: usize,
    partner_age: usize,
    adults: Vec<usize>,
    children: Vec<usize>,
    head: (usize, usize),
    partner: (usize, usize),
    adult_groups: Vec<(usize, usize)>,
    child_groups: Vec<usize>,
}

impl Layout {
    fn of(kind: &PopKindDecl) -> Layout {
        let attr = |name: &str| {
            let Some(i) = kind.key_attrs.iter().position(|a| a.item.name == name) else {
                violation!(clause = "REP.19", "a household key attribute the kind does not hold");
            };
            i
        };
        let group = |name: &str| {
            let Some(i) = kind.groups.iter().position(|g| g.name == name) else {
                violation!(clause = "REP.32", "a household profile group the kind does not hold");
            };
            i
        };
        let pair = |(l, s): &(phx_core::GroupDecl, phx_core::GroupDecl)| (group(l.name), group(s.name));
        let mut adult_groups: Vec<(usize, usize)> = ADULT_GROUPS.iter().map(pair).collect();
        let others = adult_groups.split_off(2);
        let (Some(head), Some(partner)) = (adult_groups.first().copied(), adult_groups.get(1).copied()) else {
            violation!(clause = "REP.26", "a household without its head's and partner's groups");
        };
        Layout {
            region: attr(REGION.name),
            head_age: attr(HEAD_AGE.name),
            partners: attr(PARTNERS.name),
            partner_age: attr(PARTNER_AGE.name),
            adults: ADULT_COUNTS.iter().map(|a| attr(a.name)).collect(),
            children: CHILD_COUNTS.iter().map(|a| attr(a.name)).collect(),
            head,
            partner,
            adult_groups: others,
            child_groups: CHILD_GROUPS.iter().map(|g| group(g.name)).collect(),
        }
    }
}

/// The households' rows are none yet, so no line's kink can part them.
struct NoRows;

impl LineKinks for NoRows {
    fn points(&self, _: LineId, _: Side) -> Vec<i64> {
        Vec::new()
    }
}

/// Each country's households, drawn region by region: the country's people apportioned among its regions by their
/// land, then households drawn until a region's persons reach its share, each from its own draws. A household's type
/// is drawn at the country's fertility; a family's mother by her age's people and chance of a child, her children
/// under the age of majority by single age, her partner at the drawn gap; a lone head or another adult by the
/// adults' ages, an older relative by the old's. Each person's health is drawn by age and sex, each adult's education
/// by age band and sex. The households of one key are gathered into one part and landed.
#[clause("GEN.2", "GEN.3", "POP.1", "POP.2", "REP.25", "REP.26")]
#[derive(Debug)]
pub struct Households {
    pub prims: Prims,
}

/// What the draw made, for the report: households, persons, persons in each of the derived shares' age bands, and
/// persons disabled.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    households: u64,
    persons: u64,
    bands: [u64; BANDS],
    disabled: u64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.households += other.households;
        self.persons += other.persons;
        self.disabled += other.disabled;
        for (a, b) in self.bands.iter_mut().zip(other.bands) {
            *a += b;
        }
    }
}

/// What one region's draw made.
struct Region {
    drawn: Vec<Drawn>,
    tally: Tally,
}

/// One region's households drawn and gathered by key.
fn draw_region(
    country: &Country<'_>,
    opening_ctx: &phx_core::OpeningCtx<'_>,
    (region, people): (u32, u64),
    kind: &PopKindDecl,
    classes: &Partition,
) -> Region {
    let layout = Layout::of(kind);
    let profiles = ProfileLayout::new(&kind.groups);
    let class = |a: u32| {
        let at = classes.bounds.partition_point(|b| *b <= i64::from(a));
        let Some(c) = at.checked_sub(1).and_then(|c| u32::try_from(c).ok()) else {
            violation!(clause = "REP.25", "an age below the first age class", age = a);
        };
        c
    };
    let mut cells: BTreeMap<KeyRecord, (u32, Profile)> = BTreeMap::new();
    let (mut tally, mut ordinal) = (Tally::default(), 0_u32);
    let mut members: Vec<Person> = Vec::new();
    while tally.persons < people {
        let subject = opening_subject(region, ordinal);
        let mut d = opening_ctx.draws(&CompositionStream::DECL, subject);
        let mut health = opening_ctx.draws(&HealthStream::DECL, subject);
        let mut school = opening_ctx.draws(&EducationStream::DECL, subject);
        country.household(&mut d, &mut members);
        let mut record = KeyRecord::default();
        let set = |r: &mut KeyRecord, attr: usize, v: u32| kind.key.set(r, attr, v);
        set(&mut record, layout.region, region);
        let mut adults = vec![0_u32; layout.adults.len()];
        let mut children = vec![0_u32; layout.children.len()];
        let mut values: Vec<(usize, u32)> = Vec::with_capacity(members.len() * 2);
        for p in &members {
            let disabled = country.disabled(p, &mut health);
            tally.disabled += u64::from(disabled);
            if let Some(b) = tally.bands.get_mut(band(i64::from(p.age))) {
                *b += 1;
            }
            let life = joint(LIFE, &[country.birth_year(p), p.sex, disabled]);
            let c = class(p.age);
            let (life_group, school_group) = match p.place {
                Place::Head => {
                    set(&mut record, layout.head_age, c);
                    (layout.head.0, Some(layout.head.1))
                }
                Place::Partner => {
                    set(&mut record, layout.partners, 1);
                    set(&mut record, layout.partner_age, c);
                    (layout.partner.0, Some(layout.partner.1))
                }
                Place::Adult => {
                    let (Some(n), Some(g)) = (adults.get_mut(index(c)), layout.adult_groups.get(index(c))) else {
                        violation!(clause = "REP.26", "an adult's age class beyond the household's roles");
                    };
                    *n += 1;
                    (g.0, Some(g.1))
                }
                Place::Child => {
                    let (Some(n), Some(g)) = (children.get_mut(index(c)), layout.child_groups.get(index(c))) else {
                        violation!(clause = "REP.26", "a child's age class beyond the household's roles");
                    };
                    *n += 1;
                    (*g, None)
                }
            };
            values.push((life_group, life));
            if let Some(g) = school_group {
                values.push((g, joint(SCHOOLING, &[country.education(p, &mut school)])));
            }
        }
        for (attr, n) in layout.adults.iter().zip(&adults) {
            set(&mut record, *attr, *n);
        }
        for (attr, n) in layout.children.iter().zip(&children) {
            set(&mut record, *attr, *n);
        }
        let (count, profile) = cells.entry(record).or_insert_with(|| (0, Profile::empty(&profiles)));
        *count += 1;
        for (g, v) in values {
            profile.add(&profiles, g, v, 1);
        }
        tally.persons += len_u64(members.len());
        tally.households += 1;
        let Some(next) = ordinal.checked_add(1) else {
            capacity_exceeded!("households of a region", u32::MAX, ordinal);
        };
        ordinal = next;
    }
    let drawn =
        cells.into_iter().map(|(key, (weight, profile))| Drawn { key, weight: Weight::new(weight), profile }).collect();
    Region { drawn, tally }
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
        PARTIES
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
        let Opening { ctx, day, date, register, countries, report, books, population, .. } = opening;
        let Some(books) = books.downcast_mut::<Books>() else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's books");
        };
        let Some(population) = population.downcast_mut::<Population>() else {
            violation!(clause = "GEN.3", "an opening handed something other than the world's population");
        };
        let at = household_kind(population);
        let classes = self.prims.age_classes.shared(register);
        for c in *countries {
            let country = Country::of(&self.prims, register, c, date.year());
            let tiles: Vec<u64> = c.regions.iter().map(|(_, tiles)| len_u64(tiles.len())).collect();
            let mut lot = ctx.draws(&RegionsStream::DECL, opening_subject(u32::from(c.id.get()), 0));
            let shares = apportion(c.people, &tiles, &mut lot);
            let (mut tally, mut cells) = (Tally::default(), 0_u64);
            for ((region, _), people) in c.regions.iter().zip(shares) {
                let Some(kd) = population.kinds.get_mut(at) else {
                    violation!(clause = "POP.2", "a world that keeps no household kind");
                };
                let drawn = draw_region(&country, ctx, (*region, people), &kd.decl, classes);
                let landed = land(books, kd, at, *day, drawn.drawn);
                tally.add(&drawn.tally);
                cells += landed;
            }
            if tally.households == 0 {
                violation!(clause = "GEN.3", "a country whose people make no household", country = c.id.get());
            }
            population.count(at, tally.households, 0);
            report.distributions.push((key(HOUSEHOLDS, c.id), describe(c, &tally, cells)));
        }
    }
}

/// A region's households landed as cells in the household kind's table.
fn land(books: &mut Books, kd: &mut PopKind, at: usize, today: phx_id::Day, drawn: Vec<Drawn>) -> u64 {
    let Books { ledger, parties, .. } = books;
    let (cells, directory, space) = parties.cells_mut();
    let table = Population::table_mut::<SystemBacking>(cells, at);
    let PopKind { decl, keys, index, levels, place, .. } = kd;
    let mut ctx =
        TenB { ledger, table, place: *place, keys, directory, space, kind: decl, levels, kinks: &NoRows, today };
    land_drawn(&mut ctx, index, drawn).new_cells
}

fn describe(c: &OpeningCountry, t: &Tally, cells: u64) -> String {
    let share = |n: u64| PERCENT * from_u64(n) / from_u64(t.persons);
    let [under, _, over] = t.bands;
    format!(
        "country {}: {} households of {} persons (people {}, {:.2} a household) in {cells} cells; {:.1}% under 15 \
         (GEN.share_under_15 {:.1}%), {:.1}% 65 and over (GEN.share_65_plus {:.1}%), {:.1}% disabled. Drawn by type \
         (DEM.household_types at GEN.fertility, DEM.household_members), mothers and their children \
         (DEM.minor_children), partners (DEM.partner_age_gap), ages (DEM.age_standard raked to GEN.share_under_15 and \
         GEN.share_65_plus), health (DEM.disability_prevalence, DEM.disability_onset) and education \
         (DEM.education_female, DEM.education_male)",
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
    )
}
