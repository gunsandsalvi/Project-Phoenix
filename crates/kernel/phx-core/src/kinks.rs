use phx_macros::clause;

/// What a kink lies on: a position of a party, or an amount per member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KinkOn {
    Position(&'static str),
    PerMember(&'static str),
}

/// Where a kink comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KinkSource {
    Rule(&'static str),
    Term(&'static str),
    Constraint(&'static str),
}

/// A point where a rule, a contract term or a constraint changes: a tax band on a year-to-date position, a means
/// test, a credit limit, a payment due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KinkDecl {
    pub name: &'static str,
    pub on: KinkOn,
    pub source: KinkSource,
    pub owner: &'static str,
    pub clause: &'static str,
}

/// Every declared kink, so the population and the ledger read kinks without depending on each other.
#[clause("REP.16")]
#[derive(Debug, Default)]
pub struct KinkRegistry {
    kinks: Vec<KinkDecl>,
}

impl KinkRegistry {
    /// # Errors
    /// A kink of a name already registered.
    pub fn register(&mut self, kink: KinkDecl) -> Result<(), String> {
        if self.kinks.iter().any(|k| k.name == kink.name) {
            return Err(format!("kink `{}` registered twice", kink.name));
        }
        self.kinks.push(kink);
        Ok(())
    }

    /// The kinks on a position or per-member amount, in the order registered.
    pub fn on(&self, on: KinkOn) -> impl Iterator<Item = &KinkDecl> + '_ {
        self.kinks.iter().filter(move |k| k.on == on)
    }
}

#[cfg(test)]
mod tests {
    use super::{KinkDecl, KinkOn, KinkRegistry, KinkSource};

    #[test]
    fn kinks_by_what_they_lie_on() {
        let band = KinkDecl {
            name: "TAX.income_band",
            on: KinkOn::Position("TAX.income_to_date"),
            source: KinkSource::Rule("TAX.income_tax"),
            owner: "TAX",
            clause: "TAX.2",
        };
        let mut r = KinkRegistry::default();
        r.register(band).unwrap();
        assert!(r.register(band).is_err());
        assert_eq!(r.on(KinkOn::Position("TAX.income_to_date")).count(), 1);
        assert_eq!(r.on(KinkOn::PerMember("TAX.income_to_date")).count(), 0);
    }
}
