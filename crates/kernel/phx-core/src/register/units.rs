use phx_num::{Missing, UnitId, UnitTable};
use serde::Deserialize;

use crate::register::Source;

/// What a unit counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    /// Money's smallest units of face, which a debt claim or a banknote promises.
    Face,
    Shares,
    FundUnits,
    Contracts,
    Physical,
    Hours,
    Persons,
    Dwellings,
    SquareMetres,
    Kilometres,
}

/// A unit as the world declares it: its name, what it counts, and its price exponent, the power of ten by which a
/// price in the smallest money units per unit is divided, with where that exponent comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitDecl {
    pub name: String,
    pub kind: UnitKind,
    pub price_exp: u8,
    pub source: Source,
    pub source_ref: String,
}

/// A `[[unit]]` entry of a data file.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnitEntry {
    pub name: String,
    pub kind: UnitKind,
    pub price_exp: u8,
    pub source: String,
    pub source_ref: String,
}

/// The world's units, in the order declared, each unit's identity its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Units {
    decls: Vec<UnitDecl>,
    table: UnitTable,
}

impl Units {
    /// The units, refusing a name given twice and an exponent a price cannot carry.
    ///
    /// # Errors
    /// Every refusal, listed.
    pub fn new(decls: Vec<UnitDecl>) -> Result<Units, Vec<String>> {
        let mut errors = Vec::new();
        for (i, d) in decls.iter().enumerate() {
            if decls.iter().take(i).any(|e| e.name == d.name) {
                errors.push(format!("unit `{}` declared twice", d.name));
            }
            if d.source_ref.trim().is_empty() {
                errors.push(format!("unit `{}` gives no reason for its price exponent", d.name));
            }
        }
        if u16::try_from(decls.len()).is_err() {
            errors.push(format!("{} units, beyond a unit identity's width", decls.len()));
        }
        let table = UnitTable::new(decls.iter().map(|d| d.price_exp).collect())
            .map_err(|e| vec![format!("a unit's price exponent: {e:?}")]);
        match (table, errors.is_empty()) {
            (Ok(table), true) => Ok(Units { decls, table }),
            (Ok(_), false) => Err(errors),
            (Err(mut e), _) => {
                errors.append(&mut e);
                Err(errors)
            }
        }
    }

    /// The unit of a name, if the world declares one.
    pub fn named(&self, name: &str) -> Missing<UnitId> {
        match self.decls.iter().position(|d| d.name == name).and_then(|i| u16::try_from(i).ok()) {
            Some(i) => Missing::Present(UnitId::new(i)),
            None => Missing::Absent,
        }
    }

    /// A unit's declaration.
    #[must_use]
    pub fn decl(&self, unit: UnitId) -> Option<&UnitDecl> {
        self.decls.get(usize::from(unit.index()))
    }

    /// The price exponents, by unit.
    pub fn table(&self) -> &UnitTable {
        &self.table
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.decls.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use phx_num::{Missing, UnitId};

    use super::{UnitDecl, UnitKind, Units};
    use crate::register::Source;

    fn unit(name: &str, price_exp: u8) -> UnitDecl {
        UnitDecl {
            name: name.to_owned(),
            kind: UnitKind::Shares,
            price_exp,
            source: Source::Assumed,
            source_ref: "a reason".to_owned(),
        }
    }

    #[test]
    fn units_are_named_and_refused_twice() {
        let units = Units::new(vec![unit("share", 2), unit("face", 6)]).unwrap();
        assert_eq!(units.named("face"), Missing::Present(UnitId::new(1)));
        assert_eq!(units.named("acre"), Missing::Absent);
        assert_eq!(units.table().exp(UnitId::new(0)), 2);
        assert!(Units::new(vec![unit("share", 2), unit("share", 2)]).is_err());
        assert!(Units::new(vec![unit("share", 200)]).is_err(), "an exponent no price can carry");
    }
}
