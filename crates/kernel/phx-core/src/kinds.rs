use phx_macros::{clause, opening};

use crate::declare_prim;

declare_prim! {
    /// The legal forms a party may take: what each may hold, whether it is a party separate from its owners with
    /// limited liability, whether it takes deposits or issues a currency, how it ends and who owns it.
    pub LEGAL_FORMS = "PTY.legal_forms" {
        kind: Policy, decided_by: "parliament", value: LegalForms, clause: "PTY.4", scope: Shared
    }
}

crate::declare_kind! {
    /// An estate: the members of a cell who end on one occasion, holding their count of what they held until it is
    /// sold and passed on.
    pub ESTATE_KIND = "estate" { legal_form: "estate", clause: "PTY.9" }
}

/// A kind of party, numbered at assembly in declaration order.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KindId(u16);

impl KindId {
    pub const fn new(index: u16) -> KindId {
        KindId(index)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A kind of party, its legal form named from its country's declared forms.
#[clause("PTY.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindDecl {
    pub name: &'static str,
    pub legal_form: &'static str,
    pub clause: &'static str,
}

/// What a legal form may be; a form has each feature its country lists for it, and no other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// A party apart from its owners.
    SeparateParty,
    /// Its owners are liable only to what they put in.
    LimitedLiability,
    TakesDeposits,
    /// It issues its own currency, and so cannot end in it.
    IssuesCurrency,
    /// Owners hold its equity, so it keeps an equity account for them.
    HasOwners,
}

/// Who owns a legal form's parties: the state, its shareholders, its own members, or the heirs and creditors of the
/// party it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owners {
    State,
    Shareholders,
    Members,
    HeirsAndCreditors,
}

impl Owners {
    /// The owners a form declares, by their name in the forms' vocabulary.
    ///
    /// # Errors
    /// A name outside the vocabulary.
    #[opening]
    pub fn parse(name: &str) -> Result<Owners, String> {
        match name {
            "state" => Ok(Owners::State),
            "shareholders" => Ok(Owners::Shareholders),
            "members" => Ok(Owners::Members),
            "heirs_and_creditors" => Ok(Owners::HeirsAndCreditors),
            other => Err(format!("`{other}` is not an owner of a legal form")),
        }
    }
}

/// What a legal form permits, as its country declares it: what it may hold, its features, how it can end, who owns
/// it, and the offices it decides through.
#[clause("PTY.4", "PTY.15", "PTY.16")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegalForm {
    pub name: String,
    pub may_hold: Vec<String>,
    pub features: Vec<Feature>,
    pub endings: Vec<String>,
    pub owners: Owners,
    pub offices: Vec<String>,
}

impl LegalForm {
    #[must_use]
    pub fn has(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    /// Where an office sits among the form's offices.
    #[must_use]
    pub fn office(&self, name: &str) -> Option<usize> {
        self.offices.iter().position(|o| o == name)
    }

    /// Every party can end, except an issuer of its own currency; no office is named twice; a form whose owners hold
    /// its equity is owned by others than its own members.
    ///
    /// # Errors
    /// When a form has no ending and issues no currency, names an office twice, or keeps equity for its members.
    #[opening]
    #[clause("PTY.4", "PTY.13", "PTY.16")]
    pub fn validate(&self) -> Result<(), String> {
        if self.endings.is_empty() && !self.has(Feature::IssuesCurrency) {
            return Err(format!("legal form `{}` has no way to end", self.name));
        }
        if self.offices.iter().enumerate().any(|(i, o)| self.office(o) != Some(i)) {
            return Err(format!("legal form `{}` names an office twice", self.name));
        }
        if self.has(Feature::HasOwners) && self.owners == Owners::Members {
            return Err(format!("legal form `{}` keeps equity for its own members", self.name));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Feature, LegalForm, Owners};

    #[test]
    fn legal_form_needs_ending() {
        let mut form = LegalForm {
            name: "company".to_owned(),
            may_hold: vec!["any".to_owned()],
            features: vec![Feature::SeparateParty, Feature::LimitedLiability],
            endings: vec![],
            owners: Owners::Shareholders,
            offices: vec!["chief_executive".to_owned()],
        };
        assert!(form.validate().is_err());
        form.features.push(Feature::IssuesCurrency);
        assert!(form.validate().is_ok(), "a central bank in its own currency");
        form.features.pop();
        form.endings.push("insolvency".to_owned());
        assert!(form.validate().is_ok());
    }

    #[test]
    fn unknown_owners_refused() {
        assert_eq!(Owners::parse("the state"), Err("`the state` is not an owner of a legal form".to_owned()));
        assert_eq!(Owners::parse("heirs_and_creditors"), Ok(Owners::HeirsAndCreditors));
    }

    #[test]
    fn owned_by_members_keeps_no_equity() {
        let form = LegalForm {
            name: "household".to_owned(),
            may_hold: vec![],
            features: vec![Feature::HasOwners],
            endings: vec!["dissolution".to_owned()],
            owners: Owners::Members,
            offices: vec![],
        };
        assert!(form.validate().is_err());
        assert!(LegalForm { features: vec![], ..form }.validate().is_ok());
    }
}
