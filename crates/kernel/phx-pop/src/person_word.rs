//! A person's packed word as fixed bit fields — its birth date, role, sex, health, education, labour state, last
//! occupation and wage point, and whether its life record is kept — and its skills by occupation family; each read by
//! shift and mask, each written one field at a time, nothing allocated.

use phx_id::Date;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{
    BIRTH_DAY_BITS, BIRTH_MONTH_BITS, BIRTH_YEAR_BITS, EDUCATION_BITS, HEALTH_BITS, LABOUR_BITS, OCCUPATION_BITS,
    POINT_BITS, ROLE_BITS, SEX_BITS, SKILL_BITS, SKILL_FAMILIES,
};

/// A field of a person's word: the bit it starts at and the bits it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub shift: u32,
    pub bits: u32,
}

const fn after(f: Field, bits: u32) -> Field {
    Field { shift: f.shift + f.bits, bits }
}

/// The birth date as its civil day, month and year, the year offset by half its range: an age is read against the day's
/// date with no conversion.
pub const BIRTH_DAY: Field = Field { shift: 0, bits: BIRTH_DAY_BITS };
pub const BIRTH_MONTH: Field = after(BIRTH_DAY, BIRTH_MONTH_BITS);
pub const BIRTH_YEAR: Field = after(BIRTH_MONTH, BIRTH_YEAR_BITS);
/// The role its kind declares: head, partner, adult, child.
pub const ROLE: Field = after(BIRTH_YEAR, ROLE_BITS);
pub const SEX: Field = after(ROLE, SEX_BITS);
pub const HEALTH: Field = after(SEX, HEALTH_BITS);
/// The highest education stage reached, and its field.
pub const EDUCATION: Field = after(HEALTH, EDUCATION_BITS);
pub const EDUCATION_FIELD: Field = after(EDUCATION, EDUCATION_BITS);
/// The labour state the labour law declares: not searching, searching, retired.
pub const LABOUR: Field = after(EDUCATION_FIELD, LABOUR_BITS);
/// The occupation family it last worked in, and its last job's wage point.
pub const OCCUPATION: Field = after(LABOUR, OCCUPATION_BITS);
pub const POINT: Field = after(OCCUPATION, POINT_BITS);
/// Whether its life record is kept: it held an office or founded a firm.
pub const LIFE_RECORD: Field = after(POINT, 1);

/// Every field within the word, the rest its reserve.
const _: () = assert!(LIFE_RECORD.shift + LIFE_RECORD.bits <= u64::BITS && BIRTH_YEAR.bits < u32::BITS);
/// The skills' families within their word.
const _: () = assert!(SKILL_FAMILIES * SKILL_BITS <= u32::BITS);

fn mask(bits: u32) -> u64 {
    if bits >= u64::BITS { u64::MAX } else { (1 << bits) - 1 }
}

/// A person's packed word.
#[clause("REP.25", "REP.26")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonWord(pub u64);

impl PersonWord {
    /// A person born on a date in a role, every other field nought.
    #[must_use]
    pub fn new(born: Date, role: u32) -> PersonWord {
        let year = born.year();
        let Some(bits) = year.checked_add(1 << (BIRTH_YEAR_BITS - 1)).and_then(|y| u32::try_from(y).ok()) else {
            capacity_exceeded!("a birth year", 1_i64 << (BIRTH_YEAR_BITS - 1), year);
        };
        PersonWord(0)
            .with(BIRTH_DAY, u32::from(born.day()))
            .with(BIRTH_MONTH, u32::from(born.month()))
            .with(BIRTH_YEAR, bits)
            .with(ROLE, role)
    }

    /// A field's value.
    #[must_use]
    #[inline]
    pub fn get(self, f: Field) -> u32 {
        // No field is wider than 32 bits, so its value fits.
        u32::try_from((self.0 >> f.shift) & mask(f.bits)).unwrap_or(u32::MAX)
    }

    /// The word with one field written; a value past its bits stops the run.
    #[must_use]
    pub fn with(self, f: Field, v: u32) -> PersonWord {
        let v = u64::from(v);
        if v > mask(f.bits) {
            capacity_exceeded!("a person's field", mask(f.bits), v);
        }
        PersonWord((self.0 & !(mask(f.bits) << f.shift)) | (v << f.shift))
    }

    /// Its birth year, held offset by half its field's range.
    #[must_use]
    pub fn birth_year(self) -> i32 {
        self.get(BIRTH_YEAR).cast_signed() - (1 << (BIRTH_YEAR_BITS - 1))
    }

    pub fn born(self) -> Date {
        let (Ok(month), Ok(day)) = (u8::try_from(self.get(BIRTH_MONTH)), u8::try_from(self.get(BIRTH_DAY))) else {
            violation!(clause = "REP.25", "a birth date's fields past their bits");
        };
        match Date::new(self.birth_year(), month, day) {
            Some(d) => d,
            None => violation!(clause = "REP.25", "a person's birth date no calendar holds"),
        }
    }

    /// Its age in whole years on a day, from the day's date as its facts hold it: one more on each birthday, a birthday
    /// on the 29th of February falling on the 1st of March in a common year.
    #[must_use]
    pub fn age_on(self, today: Date) -> i64 {
        let before = (u32::from(today.month()), u32::from(today.day())) < (self.get(BIRTH_MONTH), self.get(BIRTH_DAY));
        i64::from(today.year()) - i64::from(self.birth_year()) - i64::from(before)
    }
}

/// A person's skill level in each occupation family.
#[clause("POP.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Skills(pub u32);

impl Skills {
    fn at(family: u32) -> u32 {
        if family >= SKILL_FAMILIES {
            violation!(clause = "POP.1", "a skill of an occupation family past the person's", family = family);
        }
        family * SKILL_BITS
    }

    #[must_use]
    pub fn get(self, family: u32) -> u32 {
        (self.0 >> Skills::at(family)) & ((1 << SKILL_BITS) - 1)
    }

    /// The skills with one family's level written; a level past its bits stops the run.
    #[must_use]
    pub fn with(self, family: u32, level: u32) -> Skills {
        let (at, top) = (Skills::at(family), (1 << SKILL_BITS) - 1);
        if level > top {
            capacity_exceeded!("a skill level", top, level);
        }
        Skills((self.0 & !(top << at)) | (level << at))
    }
}
