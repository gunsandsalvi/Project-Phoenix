use phx_macros::clause;

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

/// Where a kind's parties are rows: the kernel's table of its individuals, or the population's cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindTableRef {
    Individuals,
    Cells,
}

/// A kind of party, its legal form named from its country's declared forms, and where its parties are rows.
#[clause("PTY.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindDecl {
    pub name: &'static str,
    pub legal_form: &'static str,
    pub table: KindTableRef,
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
}

/// What a legal form permits, as its country declares it: what it may hold, its features, how it can end, and who
/// owns it.
#[clause("PTY.4", "PTY.15")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegalForm {
    pub name: String,
    pub may_hold: Vec<String>,
    pub features: Vec<Feature>,
    pub endings: Vec<String>,
    pub owners: String,
}

impl LegalForm {
    #[must_use]
    pub fn has(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    /// Every party can end, except an issuer of its own currency.
    ///
    /// # Errors
    /// When a form has no ending and issues no currency.
    #[clause("PTY.13")]
    pub fn validate(&self) -> Result<(), String> {
        if self.endings.is_empty() && !self.has(Feature::IssuesCurrency) {
            return Err(format!("legal form `{}` has no way to end", self.name));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Feature, LegalForm};

    #[test]
    fn legal_form_needs_ending() {
        let mut form = LegalForm {
            name: "company".to_owned(),
            may_hold: vec!["any".to_owned()],
            features: vec![Feature::SeparateParty, Feature::LimitedLiability],
            endings: vec![],
            owners: "shareholders".to_owned(),
        };
        assert!(form.validate().is_err());
        form.features.push(Feature::IssuesCurrency);
        assert!(form.validate().is_ok(), "a central bank in its own currency");
        form.features.pop();
        form.endings.push("insolvency".to_owned());
        assert!(form.validate().is_ok());
    }
}
