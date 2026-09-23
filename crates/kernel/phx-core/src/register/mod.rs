pub mod limit;
pub mod profile;
mod quantile;
pub mod values;

use std::marker::PhantomData;
use std::path::Path;

use phx_id::{CountryId, SystemCode};
use phx_macros::clause;
pub use phx_num::Missing;
use phx_num::{RatePeriod, violation};
use serde::Deserialize;

use crate::calendar::period::Period;
use crate::consts::MONTHS_PER_QUARTER;
use crate::register::values::{PrimType, PrimValue, ValueType, parse};

/// What kind of number a primitive is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimKind {
    Technology,
    Preference,
    Policy,
    Endowment,
    Resolution,
    Shape,
}

impl PrimKind {
    fn name(self) -> &'static str {
        match self {
            PrimKind::Technology => "TECHNOLOGY",
            PrimKind::Preference => "PREFERENCE",
            PrimKind::Policy => "POLICY",
            PrimKind::Endowment => "ENDOWMENT",
            PrimKind::Resolution => "RESOLUTION",
            PrimKind::Shape => "SHAPE",
        }
    }
}

/// Where a primitive's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Measured,
    Estimated,
    Assumed,
    Placeholder,
}

/// The period a rate or a flow is stated per.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimPeriod {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl PrimPeriod {
    fn name(self) -> &'static str {
        match self {
            PrimPeriod::Day => "day",
            PrimPeriod::Week => "week",
            PrimPeriod::Month => "month",
            PrimPeriod::Quarter => "quarter",
            PrimPeriod::Year => "year",
        }
    }

    /// The calendar period, which no primitive states as zero.
    pub fn period(self) -> Period {
        let period = match self {
            PrimPeriod::Day => Period::days(1),
            PrimPeriod::Week => Period::weeks(1),
            PrimPeriod::Month => Period::months(1),
            PrimPeriod::Quarter => Period::months(MONTHS_PER_QUARTER),
            PrimPeriod::Year => Period::months(crate::consts::MONTHS_PER_YEAR),
        };
        let Some(p) = period else {
            violation!(clause = "NUM.3", "a primitive's period of no length");
        };
        p
    }

    /// The period a rate is stated per, where `Rate` has one.
    #[must_use]
    pub fn rate_period(self) -> Option<RatePeriod> {
        match self {
            PrimPeriod::Day => Some(RatePeriod::Day),
            PrimPeriod::Month => Some(RatePeriod::Month),
            PrimPeriod::Year => Some(RatePeriod::Year),
            PrimPeriod::Week | PrimPeriod::Quarter => None,
        }
    }
}

/// What a SHAPE stands for: a mechanism a named system retires, or a standing choice with its reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeInfo {
    Placeholder { retired_by: &'static str },
    Standing { reason: &'static str },
}

/// Whether a primitive has one value for the world or one per country.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Shared,
    PerCountry,
}

/// An institution's role that may change a policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleId(pub &'static str);

/// A primitive's declaration: everything its data entry must match, and the type its value is read as. Its owner is
/// the system its identity begins with.
#[clause("NUM.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimDecl {
    pub id: &'static str,
    pub kind: PrimKind,
    pub unit: Missing<&'static str>,
    pub period: Missing<PrimPeriod>,
    pub decided_by: Missing<RoleId>,
    pub value: ValueType,
    pub clause: &'static str,
    pub shape: Missing<ShapeInfo>,
    pub scope: Scope,
}

impl PrimDecl {
    /// The owning system's code, which begins the identity.
    ///
    /// # Errors
    /// When the identity is not `<SYS>.<name>`.
    pub fn owner(&self) -> Result<SystemCode, String> {
        let (code, name) = self.id.split_once('.').ok_or_else(|| format!("`{}` is not <SYS>.<name>", self.id))?;
        let snake = !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        match SystemCode::new(code) {
            Some(c) if snake => Ok(c),
            _ => Err(format!("`{}` is not <SYS>.<snake_case>", self.id)),
        }
    }

    /// The declaration's own consistency: a policy says who decides it, a SHAPE what it stands for, and nothing else
    /// says either.
    ///
    /// # Errors
    /// When the declaration contradicts itself.
    pub fn validate(&self) -> Result<(), String> {
        let _owner = self.owner()?;
        let policy = self.kind == PrimKind::Policy;
        if policy != matches!(self.decided_by, Missing::Present(_)) {
            return Err(format!("`{}`: `decided_by` is for policies, and every policy has one", self.id));
        }
        if (self.kind == PrimKind::Shape) != matches!(self.shape, Missing::Present(_)) {
            return Err(format!("`{}`: `shape` is for SHAPEs, and every SHAPE has one", self.id));
        }
        if self.value == ValueType::Rate && !matches!(self.period, Missing::Present(p) if p.rate_period().is_some()) {
            return Err(format!("`{}`: a rate is stated per day, month or year", self.id));
        }
        Ok(())
    }
}

/// A typed handle to a primitive, returned by its declaration; the only way a system reads it.
#[must_use]
#[derive(Debug)]
pub struct Prim<T> {
    index: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Prim<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Prim<T> {}

impl<T: PrimType> Prim<T> {
    /// A country's value, one indexed read.
    #[clause("NUM.3")]
    pub fn get(self, register: &Register, country: CountryId) -> T::Read<'_> {
        let value = match register.stored(self.index) {
            Stored::PerCountry(values) => values.get(usize::from(country.get())),
            Stored::Shared(_) => None,
        };
        let Some(read) = value.and_then(T::read) else {
            violation!(clause = "NUM.3", "a primitive read for a country it has no value for", index = self.index);
        };
        read
    }

    /// The world's value.
    #[clause("NUM.3")]
    pub fn shared(self, register: &Register) -> T::Read<'_> {
        let value = match register.stored(self.index) {
            Stored::Shared(v) => Some(v),
            Stored::PerCountry(_) => None,
        };
        let Some(read) = value.and_then(T::read) else {
            violation!(clause = "NUM.3", "a per-country primitive read as the world's", index = self.index);
        };
        read
    }

    #[must_use]
    pub fn decl(self, register: &Register) -> &PrimDecl {
        register.decl(self.index)
    }
}

/// The declarations collected before the data is read.
#[derive(Debug, Default)]
pub struct RegisterBuilder {
    decls: Vec<PrimDecl>,
}

impl RegisterBuilder {
    #[must_use]
    pub fn new() -> RegisterBuilder {
        RegisterBuilder { decls: Vec::new() }
    }

    /// Declares a primitive read as `T`, which must read the declared value type.
    pub fn declare<T: PrimType>(&mut self, decl: &PrimDecl) -> Prim<T> {
        if !T::reads(decl.value) {
            violation!(clause = "NUM.3", "a primitive read as a type its declaration does not name");
        }
        let Ok(index) = u32::try_from(self.decls.len()) else {
            phx_num::capacity_exceeded!("primitives", u32::MAX, self.decls.len());
        };
        self.decls.push(*decl);
        Prim { index, marker: PhantomData }
    }

    /// Reads every entry of the data files against the declarations: each entry declared and matching its
    /// declaration, each declaration with exactly one entry for the world or for each country.
    ///
    /// # Errors
    /// Every mismatch, listed.
    #[clause("NUM.3", "NUM.8")]
    pub fn build(self, files: &[DataFile], countries: usize) -> Result<Register, Vec<String>> {
        let mut errors = Vec::new();
        for d in &self.decls {
            if let Err(e) = d.validate() {
                errors.push(e);
            }
        }
        let mut by_id: Vec<(&str, usize)> = self.decls.iter().enumerate().map(|(i, d)| (d.id, i)).collect();
        by_id.sort_unstable();
        for pair in by_id.windows(2) {
            if let [(a, _), (b, _)] = pair
                && a == b
            {
                errors.push(format!("`{a}` declared twice"));
            }
        }
        let mut found: Vec<Vec<Option<(PrimValue, EntryMeta)>>> =
            self.decls.iter().map(|d| vec![None; if d.scope == Scope::Shared { 1 } else { countries }]).collect();
        for file in files {
            let entries = match entries(&file.text) {
                Ok(e) => e,
                Err(e) => {
                    errors.push(format!("{}: {e}", file.path));
                    continue;
                }
            };
            for entry in entries {
                let found_at = by_id.binary_search_by(|(id, _)| (*id).cmp(entry.id.as_str()));
                let Some(i) = found_at.ok().and_then(|at| by_id.get(at)).map(|(_, i)| *i) else {
                    errors.push(format!("{}: `{}` is not declared", file.path, entry.id));
                    continue;
                };
                let (Some(decl), Some(cells)) = (self.decls.get(i), found.get_mut(i)) else { continue };
                let at = match (decl.scope, file.country) {
                    (Scope::Shared, Missing::Absent) => 0,
                    (Scope::PerCountry, Missing::Present(c)) => usize::from(c.get()),
                    _ => {
                        errors.push(format!("{}: `{}` in a file of the wrong scope", file.path, entry.id));
                        continue;
                    }
                };
                match check(decl, &entry).and_then(|meta| {
                    let period = option(decl.period).and_then(PrimPeriod::rate_period);
                    parse(&entry.value, decl.value, period).map(|v| (v, meta))
                }) {
                    Ok(value) => match cells.get_mut(at) {
                        Some(cell @ None) => *cell = Some(value),
                        Some(Some(_)) => errors.push(format!("{}: `{}` given twice", file.path, entry.id)),
                        None => errors.push(format!("{}: `{}` for a country the world has not", file.path, entry.id)),
                    },
                    Err(e) => errors.push(format!("{}: `{}`: {e}", file.path, entry.id)),
                }
            }
        }
        let mut stored = Vec::with_capacity(self.decls.len());
        let mut sources = Vec::with_capacity(self.decls.len());
        for (decl, cells) in self.decls.iter().zip(found) {
            if cells.iter().any(Option::is_none) {
                errors.push(format!("`{}` has no value for every place it applies to", decl.id));
                continue;
            }
            let (values, metas): (Vec<PrimValue>, Vec<EntryMeta>) = cells.into_iter().flatten().unzip();
            let value = match decl.scope {
                Scope::Shared => values.into_iter().next().map(Stored::Shared),
                Scope::PerCountry => Some(Stored::PerCountry(values)),
            };
            if let Some(v) = value {
                stored.push(v);
                sources.push(metas);
            }
        }
        if errors.is_empty() { Ok(Register { decls: self.decls, stored, sources }) } else { Err(errors) }
    }
}

/// What an entry says about itself beyond its value.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EntryMeta {
    source: Source,
    source_ref: String,
}

/// A data file and the country it is for, or none for the world's.
#[derive(Clone, Debug)]
pub struct DataFile {
    pub path: String,
    pub country: Missing<CountryId>,
    pub text: String,
}

/// Every primitive's value, immutable once built.
#[clause("NUM.3")]
#[derive(Debug)]
pub struct Register {
    decls: Vec<PrimDecl>,
    stored: Vec<Stored>,
    sources: Vec<Vec<EntryMeta>>,
}

#[derive(Debug)]
enum Stored {
    Shared(PrimValue),
    PerCountry(Vec<PrimValue>),
}

impl Register {
    fn stored(&self, index: u32) -> &Stored {
        let Some(s) = usize::try_from(index).ok().and_then(|i| self.stored.get(i)) else {
            violation!(clause = "NUM.3", "a primitive handle from another register", index = index);
        };
        s
    }

    fn decl(&self, index: u32) -> &PrimDecl {
        let Some(d) = usize::try_from(index).ok().and_then(|i| self.decls.get(i)) else {
            violation!(clause = "NUM.3", "a primitive handle from another register", index = index);
        };
        d
    }

    /// The standing SHAPEs and their reasons.
    #[clause("NUM.7")]
    #[must_use]
    pub fn standing_shapes(&self) -> Vec<(&'static str, &'static str)> {
        let standing = |d: &PrimDecl| match d.shape {
            Missing::Present(ShapeInfo::Standing { reason }) => Some((d.id, reason)),
            _ => None,
        };
        self.decls.iter().filter_map(standing).collect()
    }

    /// The placeholder SHAPEs, each with the system that retires it.
    #[clause("NUM.7")]
    #[must_use]
    pub fn placeholders(&self) -> Vec<(&'static str, &'static str)> {
        let placeholder = |d: &PrimDecl| match d.shape {
            Missing::Present(ShapeInfo::Placeholder { retired_by }) => Some((d.id, retired_by)),
            _ => None,
        };
        self.decls.iter().filter_map(placeholder).collect()
    }

    /// Entries whose value is itself a placeholder, awaiting a source.
    #[must_use]
    pub fn unsourced(&self) -> Vec<&'static str> {
        let unsourced = |(d, metas): (&PrimDecl, &Vec<EntryMeta>)| {
            metas.iter().any(|m| m.source == Source::Placeholder).then_some(d.id)
        };
        self.decls.iter().zip(&self.sources).filter_map(unsourced).collect()
    }

    /// Every entry's reason, for the report.
    #[must_use]
    pub fn source_refs(&self) -> Vec<(&'static str, &str)> {
        self.decls
            .iter()
            .zip(&self.sources)
            .flat_map(|(d, m)| m.iter().map(|m| (d.id, m.source_ref.as_str())))
            .collect()
    }
}

/// One entry of a data file, as every declared number is written.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    kind: String,
    unit: Option<String>,
    period: Option<String>,
    owner: String,
    decided_by: Option<String>,
    source: String,
    source_ref: String,
    shape: Option<String>,
    value: toml::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    primitive: Vec<Entry>,
}

/// A development level, whose templates a country's data is instantiated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Developed,
    Emerging,
    Developing,
}

impl Level {
    /// The directory of the level's templates under `data/profiles/`.
    #[must_use]
    pub fn dir(self) -> &'static str {
        match self {
            Level::Developed => "developed",
            Level::Emerging => "emerging",
            Level::Developing => "developing",
        }
    }
}

/// One of the world's countries: the development level its data is instantiated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryEntry {
    pub level: Level,
}

fn entries(text: &str) -> Result<Vec<Entry>, String> {
    Ok(toml::from_str::<File>(text).map_err(|e| e.to_string())?.primitive)
}

fn source(text: &str) -> Result<Source, String> {
    match text {
        "measured" => Ok(Source::Measured),
        "estimated" => Ok(Source::Estimated),
        "assumed" => Ok(Source::Assumed),
        "placeholder" => Ok(Source::Placeholder),
        other => Err(format!("source `{other}` is not one of the four")),
    }
}

/// Whether an entry's shape, `<form>:<what>`, is its declaration's.
fn shape_matches(text: Option<&str>, declared: Option<ShapeInfo>) -> Result<bool, String> {
    match (text, declared) {
        (None, None) => Ok(true),
        (Some(t), Some(d)) => {
            let (form, what) = t.split_once(':').ok_or_else(|| format!("shape `{t}` is not <form>:<what>"))?;
            match (form, d) {
                ("placeholder", ShapeInfo::Placeholder { retired_by }) => Ok(what == retired_by),
                ("standing", ShapeInfo::Standing { reason }) => Ok(what == reason),
                ("placeholder" | "standing", _) => Ok(false),
                (other, _) => Err(format!("shape `{other}` is neither placeholder nor standing")),
            }
        }
        _ => Ok(false),
    }
}

/// A declared field as an option, to compare with an entry's.
fn option<T>(m: Missing<T>) -> Option<T> {
    match m {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

/// The entry against its declaration; what the entry adds is returned.
fn check(decl: &PrimDecl, e: &Entry) -> Result<EntryMeta, String> {
    let owner = decl.owner()?;
    if e.owner != owner.to_string() {
        return Err(format!("owner `{}` where the declaration's is `{owner}`", e.owner));
    }
    if e.kind != decl.kind.name() {
        return Err(format!("kind `{}` where the declaration's is `{}`", e.kind, decl.kind.name()));
    }
    if e.unit.as_deref() != option(decl.unit) {
        return Err(format!("unit {:?} where the declaration's is {:?}", e.unit, option(decl.unit)));
    }
    if e.period.as_deref() != option(decl.period).map(PrimPeriod::name) {
        return Err(format!("period {:?} where the declaration's is {:?}", e.period, option(decl.period)));
    }
    if e.decided_by.as_deref() != option(decl.decided_by).map(|r| r.0) {
        return Err(format!("decided by {:?} where the declaration says {:?}", e.decided_by, option(decl.decided_by)));
    }
    if !shape_matches(e.shape.as_deref(), option(decl.shape))? {
        return Err(format!("shape {:?} where the declaration's is {:?}", e.shape, option(decl.shape)));
    }
    if e.source_ref.trim().is_empty() {
        return Err("no source_ref".to_owned());
    }
    Ok(EntryMeta { source: source(&e.source)?, source_ref: e.source_ref.clone() })
}

/// The data files of a world: its constants and shared primitives, and each country's own.
///
/// # Errors
/// When a directory or file cannot be read.
pub fn read_data(root: &Path, countries: &[(CountryId, &str)]) -> Result<Vec<DataFile>, String> {
    let read = |path: &Path, country| -> Result<DataFile, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(DataFile { path: path.display().to_string(), country, text })
    };
    let toml_in = |dir: &Path| -> Result<Vec<std::path::PathBuf>, String> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        Ok(paths)
    };
    let mut files = vec![read(&root.join("world.toml"), Missing::Absent)?];
    for path in toml_in(&root.join("shared"))? {
        files.push(read(&path, Missing::Absent)?);
    }
    for (country, name) in countries {
        for path in toml_in(&root.join(name))? {
            files.push(read(&path, Missing::Present(*country))?);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use phx_id::{CountryId, Date};
    use phx_num::{Missing, Rate, RatePeriod};

    use super::{DataFile, PrimDecl, PrimKind, PrimPeriod, RegisterBuilder, RoleId, Scope};
    use crate::calendar::prims::{CALENDAR, EPOCH};
    use crate::calendar::rules::CountryRules;
    use crate::register::values::ValueType;

    const RATE: PrimDecl = PrimDecl {
        id: "CB.policy_rate",
        kind: PrimKind::Policy,
        unit: Missing::Present("per year"),
        period: Missing::Present(PrimPeriod::Year),
        decided_by: Missing::Present(RoleId("central_bank")),
        value: ValueType::Rate,
        clause: "CB.1",
        shape: Missing::Absent,
        scope: Scope::PerCountry,
    };

    const ENTRY: &str = r#"[[primitive]]
id = "CB.policy_rate"
kind = "POLICY"
unit = "per year"
period = "year"
owner = "CB"
decided_by = "central_bank"
source = "assumed"
source_ref = "A neutral rate."
value = 0.035
"#;

    fn file(country: u8, text: &str) -> DataFile {
        DataFile {
            path: format!("c{country}.toml"),
            country: Missing::Present(CountryId::new(country)),
            text: text.to_owned(),
        }
    }

    #[test]
    fn register_reads_a_declared_entry() {
        let mut b = RegisterBuilder::new();
        let rate = b.declare::<Rate>(&RATE);
        let register = b.build(&[file(0, ENTRY)], 1).unwrap();
        assert_eq!(rate.get(&register, CountryId::new(0)), Rate::new(35_000_000_000, RatePeriod::Year));
    }

    #[test]
    fn register_refuses_undeclared_and_missing() {
        let b = RegisterBuilder::new();
        assert!(b.build(&[file(0, ENTRY)], 1).is_err(), "an entry with no declaration");
        let mut b = RegisterBuilder::new();
        let _ = b.declare::<Rate>(&RATE);
        let errors = b.build(&[file(0, ENTRY)], 2).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("no value")), "country 1 has none: {errors:?}");
        let mut b = RegisterBuilder::new();
        let _ = b.declare::<Rate>(&RATE);
        assert!(b.build(&[file(0, ENTRY), file(0, ENTRY)], 1).is_err(), "given twice");
    }

    #[test]
    fn register_refuses_wrong_unit_or_kind() {
        for (from, to) in [
            ("unit = \"per year\"", "unit = \"per month\""),
            ("kind = \"POLICY\"", "kind = \"TECHNOLOGY\""),
            ("source = \"assumed\"", "source = \"guessed\""),
            ("source_ref = \"A neutral rate.\"", "source_ref = \" \""),
            ("value = 0.035", "value = \"x\""),
            ("owner = \"CB\"", "owner = \"BNK\""),
            ("period = \"year\"", "period = \"month\""),
        ] {
            let mut b = RegisterBuilder::new();
            let _ = b.declare::<Rate>(&RATE);
            assert!(b.build(&[file(0, &ENTRY.replace(from, to))], 1).is_err(), "{to}");
        }
    }

    #[test]
    fn policy_needs_decided_by() {
        let undecided = PrimDecl { decided_by: Missing::Absent, ..RATE };
        assert!(undecided.validate().is_err());
        let decided_technology = PrimDecl { kind: PrimKind::Technology, ..RATE };
        assert!(decided_technology.validate().is_err());
        let mut b = RegisterBuilder::new();
        let _ = b.declare::<Rate>(&RATE);
        let without = ENTRY.replace("decided_by = \"central_bank\"\n", "");
        assert!(b.build(&[file(0, &without)], 1).is_err(), "an entry of a policy says who decides it");
        assert!(PrimDecl { id: "cb.Rate", ..RATE }.validate().is_err());
        let shape = PrimDecl { kind: PrimKind::Shape, decided_by: Missing::Absent, ..RATE };
        assert!(shape.validate().is_err(), "a SHAPE says what it stands for");
    }

    #[test]
    fn the_committed_data_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        let mut files =
            vec![DataFile { path: "world.toml".to_owned(), country: Missing::Absent, text: read("world.toml") }];
        for (i, level) in ["developed", "emerging", "developing"].into_iter().enumerate() {
            let text = read(&format!("profiles/{level}/TIME.toml"));
            files.push(file(u8::try_from(i).unwrap(), &text));
        }
        let mut b = RegisterBuilder::new();
        let epoch = b.declare::<Date>(&EPOCH);
        let day_zero = b.declare::<Date>(&crate::calendar::prims::DAY_ZERO);
        let calendar = b.declare::<CountryRules>(&CALENDAR);
        let register = b.build(&files, 3).unwrap();
        assert_eq!(epoch.shared(&register), Date::new(1950, 1, 1).unwrap());
        assert!(day_zero.shared(&register) > epoch.shared(&register));
        // 2021 in England: 1 Jan, 2 Apr, 5 Apr, 3 May, 31 May, 30 Aug, and Christmas and Boxing Day on the weekend
        // with their substitutes on 27 and 28 Dec.
        let expected: Vec<Date> =
            [(1, 1), (4, 2), (4, 5), (5, 3), (5, 31), (8, 30), (12, 25), (12, 26), (12, 27), (12, 28)]
                .iter()
                .map(|(m, d)| Date::new(2021, *m, *d).unwrap())
                .collect();
        assert_eq!(calendar.get(&register, CountryId::new(0)).holidays_in(2021), expected);
        // Carnival 2025 fell on 3 and 4 March, Corpus Christi on 19 June.
        let h = calendar.get(&register, CountryId::new(1)).holidays_in(2025);
        for (m, d) in [(3, 3), (3, 4), (6, 19), (11, 20)] {
            assert!(h.contains(&Date::new(2025, m, d).unwrap()), "{m}-{d}");
        }
        assert!(register.standing_shapes().is_empty() && register.placeholders().is_empty());
    }
}
