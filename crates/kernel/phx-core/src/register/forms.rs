use serde::Deserialize;

use crate::register::Source;

/// A decision rule's form, a standing SHAPE: the rule as the literature states how people decide, with why no
/// mechanism in the world derives it and where it comes from. Its parameters are primitives of their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDecl {
    pub id: String,
    pub owner: String,
    pub reason: String,
    pub source: Source,
    pub source_ref: String,
}

/// A `[[form]]` entry of a data file.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FormEntry {
    pub id: String,
    pub owner: String,
    pub reason: String,
    pub source: String,
    pub source_ref: String,
}

/// The world's rule forms, in the order declared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Forms {
    decls: Vec<FormDecl>,
}

impl Forms {
    /// The forms, refusing an id given twice and a form that says neither why it stands nor where it comes from.
    ///
    /// # Errors
    /// Every refusal, listed.
    pub fn new(decls: Vec<FormDecl>) -> Result<Forms, Vec<String>> {
        let mut errors = Vec::new();
        for (i, d) in decls.iter().enumerate() {
            if decls.iter().take(i).any(|e| e.id == d.id) {
                errors.push(format!("form `{}` declared twice", d.id));
            }
            if !d.id.starts_with(&format!("{}.", d.owner)) {
                errors.push(format!("form `{}` is not named for its owner `{}`", d.id, d.owner));
            }
            if d.reason.trim().is_empty() || d.source_ref.trim().is_empty() {
                errors.push(format!("form `{}` says neither why it stands nor where it comes from", d.id));
            }
        }
        if errors.is_empty() { Ok(Forms { decls }) } else { Err(errors) }
    }

    #[must_use]
    pub fn decls(&self) -> &[FormDecl] {
        &self.decls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(id: &str, owner: &str, reason: &str) -> FormDecl {
        FormDecl {
            id: id.to_owned(),
            owner: owner.to_owned(),
            reason: reason.to_owned(),
            source: Source::Measured,
            source_ref: "a paper".to_owned(),
        }
    }

    #[test]
    fn forms_refuse_twins_strangers_and_silence() {
        assert!(Forms::new(vec![form("VAL.menu", "VAL", "how people forecast")]).is_ok());
        let twice = Forms::new(vec![form("VAL.menu", "VAL", "a"), form("VAL.menu", "VAL", "b")]);
        assert_eq!(twice.map_err(|e| e.len()), Err(1));
        assert!(Forms::new(vec![form("HH.menu", "VAL", "a")]).is_err());
        assert!(Forms::new(vec![form("VAL.menu", "VAL", " ")]).is_err());
    }
}
