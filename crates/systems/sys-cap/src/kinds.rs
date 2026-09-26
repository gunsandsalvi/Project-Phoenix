//! The kinds of plant as the register declares them, each held in condition classes of age, and the wear the kernel
//! realises along each kind's chain of classes.

use phx_core::register::values::Table1;
use phx_core::{Prim, Register, WearSpec};
use phx_macros::clause;
use phx_num::Count;

use crate::consts::TEN;
use crate::rules::wear;

/// The primitives the kinds are compiled from.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub depreciation: Prim<Table1>,
    pub life: Prim<Table1>,
    pub shape: Prim<Table1>,
    pub classes: Prim<Count>,
}

/// A kind of plant: its geometric depreciation rate a year, mean service life in years and efficiency shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kind {
    pub depreciation: f64,
    pub life: f64,
    pub shape: f64,
}

impl Kind {
    /// A class's mid-age in years.
    #[must_use]
    pub fn age(&self, class: usize, classes: usize) -> f64 {
        wear::mid_age(class, classes, self.life)
    }

    /// Each class's efficiency, newest first.
    #[must_use]
    pub fn efficiencies(&self, classes: usize) -> Vec<f64> {
        (0..classes).map(|c| wear::efficiency(self.age(c, classes), self.life, self.shape)).collect()
    }

    /// Each class's value as a share of cost new, newest first.
    #[must_use]
    pub fn values(&self, classes: usize) -> Vec<f64> {
        (0..classes).map(|c| wear::value(self.age(c, classes), self.depreciation)).collect()
    }
}

/// The kinds, in TEC.capital's order, and the classes each is held in.
#[derive(Clone, Debug, PartialEq)]
pub struct Kinds {
    pub kinds: Vec<Kind>,
    pub classes: usize,
}

/// The table's values as decimals of `exp` places.
pub(crate) fn decimals(t: &Table1, exp: u8) -> Vec<f64> {
    let scale = libm::pow(TEN, f64::from(exp));
    t.values().iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect()
}

/// The places a table primitive's integers carry, as its declaration states them.
pub(crate) fn places(p: &phx_core::PrimDecl) -> u8 {
    match p.value {
        phx_core::ValueType::Table1 { exp, .. } => exp,
        _ => phx_num::violation!(clause = "CAP.13", "a kind's table declared as another type"),
    }
}

impl Kinds {
    /// The kinds from the register.
    ///
    /// # Errors
    /// When the tables do not cover the same kinds, or a kind lives no time.
    #[clause("CAP.13", "CAP.1")]
    pub fn compile(prims: &Prims, register: &Register) -> Result<Kinds, String> {
        let classes = prims.classes.shared(register).get();
        Kinds::from_tables(
            (
                decimals(prims.depreciation.shared(register), places(&crate::DEPRECIATION)),
                decimals(prims.life.shared(register), places(&crate::SERVICE_LIFE)),
                decimals(prims.shape.shared(register), places(&crate::EFFICIENCY_SHAPE)),
            ),
            classes,
        )
    }

    fn from_tables((rate, life, shape): (Vec<f64>, Vec<f64>, Vec<f64>), classes: u64) -> Result<Kinds, String> {
        if rate.len() != life.len() || rate.len() != shape.len() {
            return Err("the kinds' rates, lives and shapes cover different kinds".to_owned());
        }
        if life.iter().any(|l| *l <= 0.0) {
            return Err("a kind of plant that lives no time".to_owned());
        }
        let classes = usize::try_from(classes).map_err(|_| "condition classes beyond a count".to_owned())?;
        if classes == 0 {
            return Err("plant held in no condition class".to_owned());
        }
        let kinds = (0..rate.len())
            .filter_map(|i| Some(Kind { depreciation: *rate.get(i)?, life: *life.get(i)?, shape: *shape.get(i)? }))
            .collect();
        Ok(Kinds { kinds, classes })
    }
}

/// The wear of each kind's chains, tagged by the kind's place: units leave each class at the classes over the life
/// a year, and each class keeps its value's share of cost new.
///
/// # Errors
/// When the register's kinds do not compile.
#[clause("CAP.6", "REP.24")]
pub fn wear_specs(register: &Register) -> Result<Vec<WearSpec>, String> {
    let table = |id: &str, exp| register.table1(id).map(|t| decimals(t, exp));
    let kinds = Kinds::from_tables(
        (
            table(crate::DEPRECIATION.id, places(&crate::DEPRECIATION))?,
            table(crate::SERVICE_LIFE.id, places(&crate::SERVICE_LIFE))?,
            table(crate::EFFICIENCY_SHAPE.id, places(&crate::EFFICIENCY_SHAPE))?,
        ),
        register.count(crate::CONDITION_CLASSES.id)?,
    )?;
    Ok((0_u32..)
        .zip(&kinds.kinds)
        .map(|(tag, k)| WearSpec {
            tag,
            leaving_per_year: wear::leaving_rate(kinds.classes, k.life),
            values: k.values(kinds.classes),
        })
        .collect())
}
