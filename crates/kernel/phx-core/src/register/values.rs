use phx_id::{Date, Weekday};
use phx_macros::clause;
use phx_num::{Amount, Count, Fixed, PointTable, QtyRaw, Rate, RatePeriod};
use phx_rand::{Draws, below_u64};
use toml::Value;

use crate::calendar::rules::{CountryRules, HolidayRule, WEEK, WeekendRule};
use crate::consts::{PARAM_EXP, PPM, RATE_EXP};
use crate::kinds::{Feature, LegalForm};
use crate::register::quantile;

/// What a declaration says its value is, with the decimal places each number is written to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    Fixed {
        exp: u8,
    },
    /// A fraction per the entry's period.
    Rate,
    /// Money in the smallest units `exp` decimal places below the currency's unit.
    Money {
        exp: u8,
    },
    Qty {
        exp: u8,
    },
    Count,
    Date,
    Table1 {
        axis_exp: u8,
        exp: u8,
    },
    Table2 {
        row_exp: u8,
        column_exp: u8,
        exp: u8,
    },
    /// A distribution across a kind, whose types carry values to `exp` places.
    Distribution {
        exp: u8,
    },
    PointTable {
        exp: u8,
    },
    Calendar,
    LegalForms,
}

/// What a table does with a point outside its axes: its declared rule, never an implicit one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    /// The nearest edge's value.
    Edge,
    Refuse,
}

/// A point below a table's first axis point or beyond its last, where the table refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutsideAxes;

/// Values at points of one axis; between points the value of the point at or below holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table1 {
    axis: Vec<i64>,
    values: Vec<i64>,
    outside: Outside,
}

/// The index of the axis point at or below `x`, or the edge the declared rule gives.
fn position(axis: &[i64], x: i64, outside: Outside) -> Result<usize, OutsideAxes> {
    let below = axis.partition_point(|a| *a <= x);
    let last = axis.len() - 1;
    let beyond = axis.last().is_some_and(|l| x > *l);
    match (below, beyond, outside) {
        (0, _, Outside::Refuse) | (_, true, Outside::Refuse) => Err(OutsideAxes),
        (0, _, Outside::Edge) => Ok(0),
        (_, true, Outside::Edge) => Ok(last),
        (b, false, _) => Ok(b - 1),
    }
}

impl Table1 {
    /// # Errors
    /// When `x` lies outside the axis and the table refuses there.
    pub fn at(&self, x: i64) -> Result<i64, OutsideAxes> {
        let i = position(&self.axis, x, self.outside)?;
        self.values.get(i).copied().ok_or(OutsideAxes)
    }
}

/// Values over two axes, row-major, each axis read as a `Table1`'s is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table2 {
    rows: Vec<i64>,
    columns: Vec<i64>,
    values: Vec<i64>,
    outside: Outside,
}

impl Table2 {
    /// # Errors
    /// When either point lies outside its axis and the table refuses there.
    pub fn at(&self, row: i64, column: i64) -> Result<i64, OutsideAxes> {
        let r = position(&self.rows, row, self.outside)?;
        let c = position(&self.columns, column, self.outside)?;
        self.values.get(r * self.columns.len() + c).copied().ok_or(OutsideAxes)
    }
}

/// A distribution's family and parameters, each written to twelve decimal places.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Family {
    Normal {
        mean: i64,
        sd: i64,
    },
    LogNormal {
        mu: i64,
        sigma: i64,
    },
    Pareto {
        x_min: i64,
        alpha: i64,
    },
    /// A log-normal body to `threshold` and a Pareto tail of index `alpha` above it, joined where they meet.
    LogNormalParetoTail {
        mu: i64,
        sigma: i64,
        threshold: i64,
        alpha: i64,
    },
    Gamma {
        shape: i64,
        scale: i64,
    },
    Beta {
        a: i64,
        b: i64,
    },
    Weibull {
        k: i64,
        lambda: i64,
    },
    /// Declared values, to the distribution's places, and their shares in parts per million.
    Discrete {
        values: Vec<i64>,
        shares_ppm: Vec<u32>,
    },
    /// A cumulative distribution linear between declared points: values in parameter places, cumulative shares in
    /// parts per million from 0 to 10^6.
    Empirical {
        values: Vec<i64>,
        cumulative_ppm: Vec<u32>,
    },
}

/// How a distribution is cut into a count of types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discretisation {
    /// Types of equal share, the remainder of 10^6 going one part to each of the first types; each type's value is
    /// the quantile at the middle of its share, rounded half to even to the distribution's places.
    EqualShares,
}

/// A difference across a kind, carried as types drawn once at each party's creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Distribution {
    pub family: Family,
    pub discretisation: Discretisation,
    pub exp: u8,
}

/// A type within a kind's type set.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(u16);

impl TypeId {
    pub const fn new(index: u16) -> TypeId {
        TypeId(index)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// One type: its share of the kind and the value it carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeShare {
    pub id: TypeId,
    pub share_ppm: u32,
    pub value: i64,
}

/// A kind's finite types and their shares, summing to 10^6 exactly.
#[clause("NUM.4")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeSet {
    types: Vec<TypeShare>,
}

impl TypeSet {
    /// # Errors
    /// When the shares do not sum to 10^6 exactly, or a share is zero, or there are no types.
    pub fn new(types: Vec<TypeShare>) -> Result<TypeSet, String> {
        let total: u64 = types.iter().map(|t| u64::from(t.share_ppm)).sum();
        if types.is_empty() || total != u64::from(PPM) || types.iter().any(|t| t.share_ppm == 0) {
            return Err(format!("{} types whose shares sum to {total} parts per million", types.len()));
        }
        Ok(TypeSet { types })
    }

    /// The types a distribution gives at a count, by its declared discretisation.
    ///
    /// # Errors
    /// When the count is zero or does not fit the distribution, or a quantile is not a finite number.
    #[clause("NUM.4")]
    pub fn build(distribution: &Distribution, count: u16) -> Result<TypeSet, String> {
        if count == 0 {
            return Err("a type set of no types".to_owned());
        }
        if let Family::Discrete { values, shares_ppm } = &distribution.family {
            if values.len() != usize::from(count) {
                return Err(format!("{} declared types cut into {count}", values.len()));
            }
            let types = (0..count).zip(values.iter().zip(shares_ppm)).map(|(i, (v, s))| TypeShare {
                id: TypeId(i),
                share_ppm: *s,
                value: *v,
            });
            return TypeSet::new(types.collect());
        }
        let Discretisation::EqualShares = distribution.discretisation;
        let (base, extra) = (PPM / u32::from(count), PPM % u32::from(count));
        let mut types = Vec::with_capacity(usize::from(count));
        let mut before = 0_u32;
        for i in 0..count {
            let share = base + u32::from(u32::from(i) < extra);
            let middle = (f64::from(before) + f64::from(share) / 2.0) / f64::from(PPM);
            let x = quantile::quantile(&distribution.family, middle)?;
            let value = quantile::to_places(x, distribution.exp)
                .ok_or_else(|| format!("type {i}'s value {x} is not a number to {} places", distribution.exp))?;
            types.push(TypeShare { id: TypeId(i), share_ppm: share, value });
            before += share;
        }
        TypeSet::new(types)
    }

    #[must_use]
    pub fn types(&self) -> &[TypeShare] {
        &self.types
    }
}

/// A party's type at its creation: a pick weighted by the types' shares.
#[clause("NUM.4")]
pub fn draw_type(set: &TypeSet, draws: &mut Draws) -> TypeId {
    let mut left = below_u64(draws, u64::from(PPM));
    for t in &set.types {
        let share = u64::from(t.share_ppm);
        if left < share {
            return t.id;
        }
        left -= share;
    }
    phx_num::violation!(clause = "NUM.4", "a type set whose shares fall short of a million", left = left);
}

/// A primitive's value, of the type its declaration names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimValue {
    Fixed { raw: i64, exp: u8 },
    Rate(Rate),
    Money(Amount),
    Qty(QtyRaw),
    Count(Count),
    Date(Date),
    Table1(Table1),
    Table2(Table2),
    Distribution(Distribution),
    PointTable(PointTable),
    Calendar(CountryRules),
    LegalForms(Vec<LegalForm>),
}

/// A decimal written in data as an integer scaled by 10^exp: exact, or refused. A float is read by its shortest
/// decimal form, which is what its author wrote.
fn decimal(v: &Value, exp: u8) -> Result<i64, String> {
    let text = match v {
        Value::Integer(i) => i.to_string(),
        Value::Float(f) if f.is_finite() => f.to_string(),
        Value::String(s) => s.clone(),
        other => return Err(format!("`{other}` is not a number")),
    };
    let (negative, digits) = text.strip_prefix('-').map_or((false, text.as_str()), |d| (true, d));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let fraction = fraction.trim_end_matches('0');
    if whole.is_empty() || !whole.bytes().chain(fraction.bytes()).all(|b| b.is_ascii_digit()) {
        return Err(format!("`{text}` is not a decimal"));
    }
    let places = usize::from(exp);
    if fraction.len() > places {
        return Err(format!("`{text}` has more than {exp} decimal places"));
    }
    let scaled = format!("{whole}{fraction}{}", "0".repeat(places - fraction.len()));
    let magnitude: i64 = scaled.parse().map_err(|_| format!("`{text}` is out of range at {exp} places"))?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn decimals(v: Option<&Value>, exp: u8, what: &str) -> Result<Vec<i64>, String> {
    v.and_then(Value::as_array).ok_or_else(|| format!("no list `{what}`"))?.iter().map(|x| decimal(x, exp)).collect()
}

fn ppms(v: Option<&Value>, what: &str) -> Result<Vec<u32>, String> {
    let list = v.and_then(Value::as_array).ok_or_else(|| format!("no list `{what}`"))?;
    list.iter()
        .map(|x| {
            x.as_integer().and_then(|i| u32::try_from(i).ok()).ok_or_else(|| format!("`{x}` in `{what}` is no share"))
        })
        .collect()
}

fn table(v: &Value) -> Result<&toml::Table, String> {
    v.as_table().ok_or_else(|| format!("`{v}` is not a table"))
}

fn text<'a>(t: &'a toml::Table, key: &str) -> Result<&'a str, String> {
    t.get(key).and_then(Value::as_str).ok_or_else(|| format!("no text `{key}`"))
}

fn names(t: &toml::Table, key: &str) -> Result<Vec<String>, String> {
    let list = t.get(key).and_then(Value::as_array).ok_or_else(|| format!("no list `{key}`"))?;
    list.iter().map(|n| n.as_str().map(str::to_owned).ok_or_else(|| format!("`{n}` in `{key}` is not text"))).collect()
}

fn increasing(axis: &[i64], what: &str) -> Result<(), String> {
    if axis.is_empty() || axis.windows(2).any(|w| w.first() >= w.last()) {
        return Err(format!("`{what}` is not a strictly increasing, non-empty axis"));
    }
    Ok(())
}

fn outside(t: &toml::Table) -> Result<Outside, String> {
    match text(t, "outside")? {
        "edge" => Ok(Outside::Edge),
        "refuse" => Ok(Outside::Refuse),
        other => Err(format!("`{other}` is not a rule outside the axes")),
    }
}

fn param(t: &toml::Table, key: &str) -> Result<i64, String> {
    decimal(t.get(key).ok_or_else(|| format!("no parameter `{key}`"))?, PARAM_EXP)
}

fn distribution(v: &Value, exp: u8) -> Result<Distribution, String> {
    let t = table(v)?;
    let discretisation = match text(t, "discretisation")? {
        "equal_shares" => Discretisation::EqualShares,
        other => return Err(format!("`{other}` is not a discretisation")),
    };
    let family = match text(t, "family")? {
        "normal" => Family::Normal { mean: param(t, "mean")?, sd: param(t, "sd")? },
        "lognormal" => Family::LogNormal { mu: param(t, "mu")?, sigma: param(t, "sigma")? },
        "pareto" => Family::Pareto { x_min: param(t, "x_min")?, alpha: param(t, "alpha")? },
        "lognormal_pareto_tail" => Family::LogNormalParetoTail {
            mu: param(t, "mu")?,
            sigma: param(t, "sigma")?,
            threshold: param(t, "threshold")?,
            alpha: param(t, "alpha")?,
        },
        "gamma" => Family::Gamma { shape: param(t, "shape")?, scale: param(t, "scale")? },
        "beta" => Family::Beta { a: param(t, "a")?, b: param(t, "b")? },
        "weibull" => Family::Weibull { k: param(t, "k")?, lambda: param(t, "lambda")? },
        "discrete" => {
            let values = decimals(t.get("values"), exp, "values")?;
            let shares_ppm = ppms(t.get("shares_ppm"), "shares_ppm")?;
            if values.len() != shares_ppm.len() {
                return Err("`values` and `shares_ppm` differ in length".to_owned());
            }
            Family::Discrete { values, shares_ppm }
        }
        "empirical" => {
            let values = decimals(t.get("values"), PARAM_EXP, "values")?;
            let cumulative_ppm = ppms(t.get("cumulative_ppm"), "cumulative_ppm")?;
            increasing(&values, "values")?;
            let ends = (cumulative_ppm.first().copied(), cumulative_ppm.last().copied());
            if values.len() != cumulative_ppm.len()
                || ends != (Some(0), Some(PPM))
                || cumulative_ppm.windows(2).any(|w| w.first() >= w.last())
            {
                return Err("an empirical distribution's shares rise from 0 to 10^6 at its values".to_owned());
            }
            Family::Empirical { values, cumulative_ppm }
        }
        other => return Err(format!("`{other}` is not a family")),
    };
    quantile::check(&family)?;
    Ok(Distribution { family, discretisation, exp })
}

fn weekday(v: &Value) -> Result<Weekday, String> {
    let names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let s = v.as_str().ok_or("a weekday is not text")?;
    let i = names.iter().position(|n| *n == s).ok_or_else(|| format!("`{s}` is not a weekday"))?;
    WEEK.get(i).copied().ok_or_else(|| format!("`{s}` is not a weekday"))
}

fn weekdays(v: Option<&Value>) -> Result<Vec<Weekday>, String> {
    v.and_then(Value::as_array).ok_or("weekdays are not a list")?.iter().map(weekday).collect()
}

fn small<T: TryFrom<i64>>(t: &toml::Table, key: &str, name: &str) -> Result<T, String> {
    let v = t.get(key).and_then(Value::as_integer).ok_or_else(|| format!("`{name}`: no whole number `{key}`"))?;
    T::try_from(v).map_err(|_| format!("`{name}`: `{key}` = {v} is out of range"))
}

fn holiday(v: &Value) -> Result<HolidayRule, String> {
    let t = table(v)?;
    let name = text(t, "name")?.to_owned();
    match text(t, "rule")? {
        "fixed" => Ok(HolidayRule::Fixed { month: small(t, "month", &name)?, day: small(t, "day", &name)?, name }),
        "nth_weekday" => Ok(HolidayRule::NthWeekday {
            month: small(t, "month", &name)?,
            weekday: weekday(t.get("weekday").ok_or_else(|| format!("`{name}`: no weekday"))?)?,
            n: small(t, "n", &name)?,
            name,
        }),
        "easter" => Ok(HolidayRule::EasterOffset { days: small(t, "offset_days", &name)?, name }),
        "substitute" => {
            Ok(HolidayRule::Substitute { of: text(t, "of")?.to_owned(), when_on: weekdays(t.get("when_on"))?, name })
        }
        other => Err(format!("`{name}`: `{other}` is not a kind of holiday rule")),
    }
}

fn calendar(v: &Value) -> Result<CountryRules, String> {
    let t = table(v)?;
    let weekend = WeekendRule { days: weekdays(t.get("weekend"))? };
    let list = t.get("holidays").and_then(Value::as_array).ok_or("no list `holidays`")?;
    let rules = CountryRules { weekend, holidays: list.iter().map(holiday).collect::<Result<_, _>>()? };
    rules.validate()?;
    Ok(rules)
}

fn legal_forms(v: &Value) -> Result<Vec<LegalForm>, String> {
    let list = v.as_array().ok_or("legal forms are not a list")?;
    let mut forms = Vec::with_capacity(list.len());
    for f in list {
        let t = table(f)?;
        let features = names(t, "features")?
            .iter()
            .map(|f| match f.as_str() {
                "separate_party" => Ok(Feature::SeparateParty),
                "limited_liability" => Ok(Feature::LimitedLiability),
                "takes_deposits" => Ok(Feature::TakesDeposits),
                "issues_currency" => Ok(Feature::IssuesCurrency),
                other => Err(format!("`{other}` is not a feature of a legal form")),
            })
            .collect::<Result<_, _>>()?;
        let form = LegalForm {
            name: text(t, "name")?.to_owned(),
            may_hold: names(t, "may_hold")?,
            features,
            endings: names(t, "endings")?,
            owners: text(t, "owners")?.to_owned(),
        };
        form.validate()?;
        if forms.iter().any(|g: &LegalForm| g.name == form.name) {
            return Err(format!("two legal forms named `{}`", form.name));
        }
        forms.push(form);
    }
    Ok(forms)
}

fn scaled_rows(t: &toml::Table, exp: u8, width: usize) -> Result<Vec<i64>, String> {
    let rows = t.get("values").and_then(Value::as_array).ok_or("no list `values`")?;
    let mut out = Vec::new();
    for row in rows {
        let row = decimals(Some(row), exp, "values")?;
        if row.len() != width {
            return Err(format!("a row of {} values over {width} columns", row.len()));
        }
        out.extend(row);
    }
    Ok(out)
}

/// A value read as its declared type.
///
/// # Errors
/// When the value is not of the type, or breaks the type's own rules.
pub fn parse(v: &Value, ty: ValueType, period: Option<RatePeriod>) -> Result<PrimValue, String> {
    match ty {
        ValueType::Fixed { exp } => Ok(PrimValue::Fixed { raw: decimal(v, exp)?, exp }),
        ValueType::Rate => {
            let per = period.ok_or("a rate with no period")?;
            Ok(PrimValue::Rate(Rate::new(decimal(v, RATE_EXP)?, per)))
        }
        ValueType::Money { exp } => Ok(PrimValue::Money(Amount::from_raw(decimal(v, exp)?))),
        ValueType::Qty { exp } => Ok(PrimValue::Qty(QtyRaw::from_raw(decimal(v, exp)?))),
        ValueType::Count => {
            let n = v.as_integer().and_then(|i| u64::try_from(i).ok()).ok_or_else(|| format!("`{v}` is no count"))?;
            Ok(PrimValue::Count(Count::new(n)))
        }
        ValueType::Date => {
            let d = v.as_datetime().and_then(|d| d.date).ok_or_else(|| format!("`{v}` is not a date"))?;
            let date = Date::new(i32::from(d.year), d.month, d.day).ok_or_else(|| format!("{d} is no date"))?;
            Ok(PrimValue::Date(date))
        }
        ValueType::Table1 { axis_exp, exp } => {
            let t = table(v)?;
            let axis = decimals(t.get("axis"), axis_exp, "axis")?;
            increasing(&axis, "axis")?;
            let values = decimals(t.get("values"), exp, "values")?;
            if values.len() != axis.len() {
                return Err(format!("{} values on an axis of {}", values.len(), axis.len()));
            }
            Ok(PrimValue::Table1(Table1 { axis, values, outside: outside(t)? }))
        }
        ValueType::Table2 { row_exp, column_exp, exp } => {
            let t = table(v)?;
            let rows = decimals(t.get("rows"), row_exp, "rows")?;
            let columns = decimals(t.get("columns"), column_exp, "columns")?;
            increasing(&rows, "rows")?;
            increasing(&columns, "columns")?;
            let values = scaled_rows(t, exp, columns.len())?;
            if values.len() != rows.len() * columns.len() {
                return Err(format!("{} values over {} rows of {}", values.len(), rows.len(), columns.len()));
            }
            Ok(PrimValue::Table2(Table2 { rows, columns, values, outside: outside(t)? }))
        }
        ValueType::Distribution { exp } => Ok(PrimValue::Distribution(distribution(v, exp)?)),
        ValueType::PointTable { exp } => {
            let raw = decimals(Some(v), exp, "points")?;
            PointTable::new(raw).map(PrimValue::PointTable).ok_or_else(|| "points not strictly increasing".to_owned())
        }
        ValueType::Calendar => Ok(PrimValue::Calendar(calendar(v)?)),
        ValueType::LegalForms => Ok(PrimValue::LegalForms(legal_forms(v)?)),
    }
}

/// A Rust type a primitive is read as, and the declared types it reads.
pub trait PrimType {
    type Read<'a>;
    fn reads(ty: ValueType) -> bool;
    fn read(v: &PrimValue) -> Option<Self::Read<'_>>;
}

impl<const E: u8> PrimType for Fixed<E> {
    type Read<'a> = Fixed<E>;
    fn reads(ty: ValueType) -> bool {
        ty == ValueType::Fixed { exp: E }
    }
    fn read(v: &PrimValue) -> Option<Fixed<E>> {
        match v {
            PrimValue::Fixed { raw, exp } if *exp == E => Some(Fixed::from_raw(*raw)),
            _ => None,
        }
    }
}

/// Implements `PrimType` for a value read by copy out of one variant.
macro_rules! read_copy {
    ($t:ty, $pat:pat => $out:expr, $ty:pat) => {
        impl PrimType for $t {
            type Read<'a> = $t;
            fn reads(ty: ValueType) -> bool {
                matches!(ty, $ty)
            }
            fn read(v: &PrimValue) -> Option<$t> {
                match v {
                    $pat => Some($out),
                    _ => None,
                }
            }
        }
    };
}

read_copy!(Rate, PrimValue::Rate(r) => *r, ValueType::Rate);
read_copy!(Amount, PrimValue::Money(a) => *a, ValueType::Money { .. });
read_copy!(QtyRaw, PrimValue::Qty(q) => *q, ValueType::Qty { .. });
read_copy!(Count, PrimValue::Count(c) => *c, ValueType::Count);
read_copy!(Date, PrimValue::Date(d) => *d, ValueType::Date);

/// Implements `PrimType` for a value read by reference.
macro_rules! read_ref {
    ($t:ty, $variant:ident, $ty:pat) => {
        impl PrimType for $t {
            type Read<'a> = &'a $t;
            fn reads(ty: ValueType) -> bool {
                matches!(ty, $ty)
            }
            fn read(v: &PrimValue) -> Option<&$t> {
                match v {
                    PrimValue::$variant(x) => Some(x),
                    _ => None,
                }
            }
        }
    };
}

read_ref!(Table1, Table1, ValueType::Table1 { .. });
read_ref!(Table2, Table2, ValueType::Table2 { .. });
read_ref!(Distribution, Distribution, ValueType::Distribution { .. });
read_ref!(PointTable, PointTable, ValueType::PointTable { .. });
read_ref!(CountryRules, Calendar, ValueType::Calendar);
read_ref!(Vec<LegalForm>, LegalForms, ValueType::LegalForms);

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};
    use toml::Value;

    use super::{
        Discretisation, Distribution, Family, Outside, OutsideAxes, PrimValue, TypeId, TypeSet, TypeShare, ValueType,
        decimal, draw_type, parse,
    };

    fn value(text: &str) -> Value {
        toml::from_str::<toml::Table>(&format!("v = {text}")).unwrap().remove("v").unwrap()
    }

    #[test]
    fn decimals_are_exact_or_refused() {
        assert_eq!(decimal(&value("0.035"), 4), Ok(350));
        assert_eq!(decimal(&value("-12"), 2), Ok(-1200));
        assert_eq!(decimal(&value("\"0.1000\""), 1), Ok(1));
        assert!(decimal(&value("0.035"), 2).is_err(), "more places than declared");
        assert!(decimal(&value("\"1e3\""), 2).is_err());
    }

    #[test]
    fn tables_follow_their_outside_rule() {
        let ty = ValueType::Table1 { axis_exp: 0, exp: 2 };
        let PrimValue::Table1(t) =
            parse(&value("{ axis = [0, 10, 20], values = [1, 2.5, 4], outside = \"edge\" }"), ty, None).unwrap()
        else {
            panic!()
        };
        assert_eq!((t.at(-5), t.at(0), t.at(15), t.at(20), t.at(99)), (Ok(100), Ok(100), Ok(250), Ok(400), Ok(400)));
        let PrimValue::Table1(r) =
            parse(&value("{ axis = [0, 10], values = [1, 2], outside = \"refuse\" }"), ty, None).unwrap()
        else {
            panic!()
        };
        assert_eq!((r.at(-1), r.at(10), r.at(11)), (Err(OutsideAxes), Ok(200), Err(OutsideAxes)));
        assert!(parse(&value("{ axis = [0, 0], values = [1, 2], outside = \"edge\" }"), ty, None).is_err());
        assert!(parse(&value("{ axis = [0, 1], values = [1, 2] }"), ty, None).is_err(), "no implicit rule");
        let ty2 = ValueType::Table2 { row_exp: 0, column_exp: 0, exp: 0 };
        let PrimValue::Table2(t2) = parse(
            &value("{ rows = [0, 10], columns = [0, 5], values = [[1, 2], [3, 4]], outside = \"edge\" }"),
            ty2,
            None,
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!((t2.at(0, 0), t2.at(12, 7), t2.at(3, 5)), (Ok(1), Ok(4), Ok(2)));
        assert_eq!(Outside::Edge, Outside::Edge);
    }

    #[test]
    fn typeset_shares_must_sum() {
        let share = |i, s| TypeShare { id: TypeId::new(i), share_ppm: s, value: 0 };
        assert!(TypeSet::new(vec![share(0, 500_000), share(1, 500_000)]).is_ok());
        assert!(TypeSet::new(vec![share(0, 500_000), share(1, 499_999)]).is_err());
        assert!(TypeSet::new(vec![share(0, 1_000_000), share(1, 0)]).is_err());
        assert!(TypeSet::new(vec![]).is_err());
    }

    #[test]
    fn typeset_built_from_distribution_and_count() {
        let normal = Distribution {
            family: Family::Normal { mean: 0, sd: 1_000_000_000_000 },
            discretisation: Discretisation::EqualShares,
            exp: 6,
        };
        let three = TypeSet::build(&normal, 3).unwrap();
        let shares: Vec<u32> = three.types().iter().map(|t| t.share_ppm).collect();
        assert_eq!(shares, vec![333_334, 333_333, 333_333]);
        // The middles of the shares are 0.166667, 0.5000005 and 0.8333335, whose quantiles are −0.967420, 0.000001
        // and 0.967422 to six places.
        let values: Vec<i64> = three.types().iter().map(|t| t.value).collect();
        assert_eq!(values, vec![-967_420, 1, 967_422]);
        let four = TypeSet::build(&normal, 4).unwrap();
        // The middles of the quarters: 1/8, 3/8, 5/8, 7/8, at ∓1.150349 and ∓0.318639.
        let values: Vec<i64> = four.types().iter().map(|t| t.value).collect();
        assert_eq!(values, vec![-1_150_349, -318_639, 318_639, 1_150_349]);
        assert!(TypeSet::build(&normal, 0).is_err());
        let discrete = Distribution {
            family: Family::Discrete { values: vec![1, 2], shares_ppm: vec![250_000, 750_000] },
            discretisation: Discretisation::EqualShares,
            exp: 0,
        };
        assert_eq!(TypeSet::build(&discrete, 2).unwrap().types()[1].share_ppm, 750_000);
        assert!(TypeSet::build(&discrete, 3).is_err(), "declared types are not re-cut");
    }

    #[test]
    fn draw_type_matches_shares() {
        let share = |i, s| TypeShare { id: TypeId::new(i), share_ppm: s, value: 0 };
        let set = TypeSet::new(vec![share(0, 100_000), share(1, 600_000), share(2, 300_000)]).unwrap();
        let mut d = Draws::new(stream_key(Seed::new(3), "types"), Subject::new(SubjectTag::World, 0), 0, 0);
        let n = 200_000;
        let mut counts = [0_u32; 3];
        for _ in 0..n {
            counts[usize::from(draw_type(&set, &mut d).get())] += 1;
        }
        // Each count is binomial; at z = 6.1 its deviation stays below 6.1·sqrt(n·p·(1−p)): 820, 1335 and 1250.
        for (count, (p, bound)) in counts.iter().zip([(0.1, 820.0), (0.6, 1335.0), (0.3, 1250.0)]) {
            assert!((f64::from(*count) - p * f64::from(n)).abs() < bound, "{counts:?}");
        }
    }
}
