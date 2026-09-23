use phx_id::{Date, Weekday, civil_from_days, days_from_civil};
use phx_macros::clause;

use crate::consts::{
    COMMON_YEAR, EASTER_CENTURY, EASTER_F, EASTER_FOUR, EASTER_G_DIV, EASTER_GOLDEN, EASTER_H, EASTER_L, EASTER_M,
    EASTER_MONTH, MAX_NTH,
};

/// The weekdays a country's banks and markets close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekendRule {
    pub days: Vec<Weekday>,
}

/// How a holiday's date is found in a year.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HolidayRule {
    Fixed {
        name: String,
        month: u8,
        day: u8,
    },
    /// The `n`-th such weekday of the month; `n` = −1 is the last.
    NthWeekday {
        name: String,
        month: u8,
        weekday: Weekday,
        n: i8,
    },
    /// Days from Easter Sunday, negative before it.
    EasterOffset {
        name: String,
        days: i32,
    },
    /// The named holiday, when it falls on one of these weekdays, is also kept on the next day that is neither a
    /// weekend day nor another holiday; substitutes resolve in declared order.
    Substitute {
        name: String,
        of: String,
        when_on: Vec<Weekday>,
    },
}

impl HolidayRule {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            HolidayRule::Fixed { name, .. }
            | HolidayRule::NthWeekday { name, .. }
            | HolidayRule::EasterOffset { name, .. }
            | HolidayRule::Substitute { name, .. } => name,
        }
    }
}

/// A country's business days: its weekdays outside the weekend, less its holidays.
#[clause("TIME.2", "TIME.13")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountryRules {
    pub weekend: WeekendRule,
    pub holidays: Vec<HolidayRule>,
}

/// Easter Sunday of a Gregorian year, by the anonymous computus of Meeus, Jones and Butcher.
#[must_use]
pub fn easter_sunday(year: i32) -> Option<Date> {
    let golden = year.rem_euclid(EASTER_GOLDEN);
    let (century, year_in_century) = (year.div_euclid(EASTER_CENTURY), year.rem_euclid(EASTER_CENTURY));
    let (century_quarter, century_rest) = (century / EASTER_FOUR, century % EASTER_FOUR);
    let [f_add, f_div] = EASTER_F;
    let lunar = (century + f_add) / f_div;
    let solar = (century - lunar + 1) / EASTER_G_DIV;
    let [h_add, h_mod] = EASTER_H;
    let epact = (EASTER_GOLDEN * golden + century - century_quarter - solar + h_add).rem_euclid(h_mod);
    let (year_quarter, year_rest) = (year_in_century / EASTER_FOUR, year_in_century % EASTER_FOUR);
    let [l_add, l_mod] = EASTER_L;
    let weekday_shift = (l_add + 2 * century_rest + 2 * year_quarter - epact - year_rest).rem_euclid(l_mod);
    let [m_h, m_l, m_div] = EASTER_M;
    let correction = (golden + m_h * epact + m_l * weekday_shift) / m_div;
    let [m_mul, m_add, m_div31] = EASTER_MONTH;
    let t = epact + weekday_shift - m_mul * correction + m_add;
    let (month, day) = (u8::try_from(t / m_div31).ok()?, u8::try_from(t % m_div31 + 1).ok()?);
    Date::new(year, month, day)
}

fn nth_weekday(year: i32, month: u8, weekday: Weekday, n: i8) -> Option<Date> {
    let last = Date::days_in_month(year, month)?;
    let mut days = (1..=last).filter_map(|d| Date::new(year, month, d)).filter(|d| d.weekday() == weekday);
    if n == -1 { days.next_back() } else { usize::try_from(n - 1).ok().and_then(|k| days.nth(k)) }
}

fn serial(date: Date) -> i64 {
    days_from_civil(date)
}

impl CountryRules {
    /// A refusal naming what is wrong: a weekend of every day, an impossible date, a substitute for a holiday not
    /// declared before it, two rules of one name.
    ///
    /// # Errors
    /// When the rules cannot describe a calendar.
    pub fn validate(&self) -> Result<(), String> {
        if WEEK.iter().all(|d| self.weekend.days.contains(d)) {
            return Err("a weekend of every day leaves no business day".to_owned());
        }
        let mut seen: Vec<&str> = Vec::new();
        for rule in &self.holidays {
            if seen.contains(&rule.name()) {
                return Err(format!("two holiday rules named `{}`", rule.name()));
            }
            match rule {
                HolidayRule::Fixed { month, day, .. } => {
                    // A fixed day must exist every year, so 29 February is refused.
                    if Date::new(COMMON_YEAR, *month, *day).is_none() {
                        return Err(format!("`{}` names no date every year has", rule.name()));
                    }
                }
                HolidayRule::NthWeekday { month, n, .. } => {
                    if Date::days_in_month(COMMON_YEAR, *month).is_none() || !(*n == -1 || (1..=MAX_NTH).contains(n)) {
                        return Err(format!("`{}` names a weekday no month has every year", rule.name()));
                    }
                }
                HolidayRule::EasterOffset { .. } => {}
                HolidayRule::Substitute { of, .. } => {
                    let base = self.holidays.iter().take_while(|r| r.name() != rule.name()).find(|r| r.name() == of);
                    if !matches!(base, Some(r) if !matches!(r, HolidayRule::Substitute { .. })) {
                        return Err(format!("`{}` substitutes `{of}`, which no earlier rule declares", rule.name()));
                    }
                }
            }
            seen.push(rule.name());
        }
        Ok(())
    }

    #[must_use]
    pub fn is_weekend(&self, date: Date) -> bool {
        self.weekend.days.contains(&date.weekday())
    }

    /// The date a non-substitute rule gives in a year, or none for a substitute.
    fn base_date(rule: &HolidayRule, year: i32) -> Option<Date> {
        match rule {
            HolidayRule::Fixed { month, day, .. } => Date::new(year, *month, *day),
            HolidayRule::NthWeekday { month, weekday, n, .. } => nth_weekday(year, *month, *weekday, *n),
            HolidayRule::EasterOffset { days, .. } => {
                easter_sunday(year).map(|e| civil_from_days(serial(e) + i64::from(*days)))
            }
            HolidayRule::Substitute { .. } => None,
        }
    }

    /// The holidays of the years before, of and after `year` that the substitutes of `year`'s neighbours can reach,
    /// as sorted serials.
    fn holidays_around(&self, year: i32) -> Vec<i64> {
        let mut days: Vec<i64> = Vec::new();
        for y in [year - 1, year, year + 1] {
            let mut base: Vec<(&str, i64)> =
                self.holidays.iter().filter_map(|r| Self::base_date(r, y).map(|d| (r.name(), serial(d)))).collect();
            days.extend(base.iter().map(|(_, s)| *s));
            for rule in &self.holidays {
                let HolidayRule::Substitute { name, of, when_on } = rule else { continue };
                let Some(&(_, on)) = base.iter().find(|(n, _)| n == of) else { continue };
                if !when_on.contains(&civil_from_days(on).weekday()) {
                    continue;
                }
                let mut next = on + 1;
                while self.is_weekend(civil_from_days(next)) || days.contains(&next) {
                    next += 1;
                }
                days.push(next);
                base.push((name, next));
            }
        }
        days.sort_unstable();
        days.dedup();
        days
    }

    /// The holidays kept in `year`, in date order.
    #[must_use]
    pub fn holidays_in(&self, year: i32) -> Vec<Date> {
        self.holidays_around(year).into_iter().map(civil_from_days).filter(|d| d.year() == year).collect()
    }
}

/// Monday to Sunday.
pub const WEEK: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

#[cfg(test)]
mod tests {
    use phx_id::{Date, Weekday};

    use super::{CountryRules, HolidayRule, WeekendRule, easter_sunday};

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    #[test]
    fn easter_known_years() {
        for (y, m, d) in [(1961, 4, 2), (2000, 4, 23), (2008, 3, 23), (2024, 3, 31), (2038, 4, 25)] {
            assert_eq!(easter_sunday(y), Some(date(y, m, d)), "{y}");
        }
    }

    fn uk_christmas() -> CountryRules {
        let fixed = |name: &str, month, day| HolidayRule::Fixed { name: name.to_owned(), month, day };
        let sub = |name: &str, of: &str| HolidayRule::Substitute {
            name: name.to_owned(),
            of: of.to_owned(),
            when_on: vec![Weekday::Saturday, Weekday::Sunday],
        };
        CountryRules {
            weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
            holidays: vec![
                fixed("Christmas", 12, 25),
                fixed("Boxing Day", 12, 26),
                sub("Christmas substitute", "Christmas"),
                sub("Boxing Day substitute", "Boxing Day"),
                HolidayRule::NthWeekday { name: "Spring".to_owned(), month: 5, weekday: Weekday::Monday, n: -1 },
            ],
        }
    }

    #[test]
    fn substitutes_do_not_collide() {
        let rules = uk_christmas();
        rules.validate().unwrap();
        // 2021: Christmas on a Saturday, Boxing Day on a Sunday: Monday 27 and Tuesday 28.
        let h = rules.holidays_in(2021);
        assert!(h.contains(&date(2021, 12, 27)) && h.contains(&date(2021, 12, 28)));
        // 2022: Christmas on a Sunday, Boxing Day on Monday: the substitute is Tuesday 27.
        let h = rules.holidays_in(2022);
        assert!(h.contains(&date(2022, 12, 26)) && h.contains(&date(2022, 12, 27)) && !h.contains(&date(2022, 12, 28)));
        assert!(h.contains(&date(2022, 5, 30)), "the last Monday of May 2022");
    }

    #[test]
    fn rules_are_validated() {
        let mut rules = uk_christmas();
        rules.holidays.push(HolidayRule::Fixed { name: "Leap".to_owned(), month: 2, day: 29 });
        assert!(rules.validate().is_err());
        let mut rules = uk_christmas();
        rules.holidays.insert(
            0,
            HolidayRule::Substitute { name: "early".to_owned(), of: "Christmas".to_owned(), when_on: vec![] },
        );
        assert!(rules.validate().is_err(), "a substitute before its holiday");
        let rules = CountryRules { weekend: WeekendRule { days: super::WEEK.to_vec() }, holidays: vec![] };
        assert!(rules.validate().is_err());
    }
}
