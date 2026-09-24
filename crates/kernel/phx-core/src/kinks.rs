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

/// The points where a rule, a contract term or a constraint changes on one position: a tax schedule's bands on a
/// year-to-date position, a means test, a credit limit, a payment due. How many points the source puts there is
/// declared with its schedule and fixed for the run; a budget moves their values, never their count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KinkDecl {
    pub name: &'static str,
    pub on: KinkOn,
    pub source: KinkSource,
    pub points: u16,
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
    /// A kink of a name already registered, a second declaration of one source's points on one position, or a
    /// declaration of no points.
    pub fn register(&mut self, kink: KinkDecl) -> Result<(), String> {
        if self.kinks.iter().any(|k| k.name == kink.name) {
            return Err(format!("kink `{}` registered twice", kink.name));
        }
        if let Some(k) = self.kinks.iter().find(|k| k.on == kink.on && k.source == kink.source) {
            return Err(format!("kink `{}` repeats `{}`'s source on the same position", kink.name, k.name));
        }
        if kink.points == 0 {
            return Err(format!("kink `{}` declares no points", kink.name));
        }
        self.kinks.push(kink);
        Ok(())
    }

    /// A kink by its place in the order registered, as a split request names it.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&KinkDecl> {
        self.kinks.get(index)
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
            points: 3,
            owner: "TAX",
            clause: "TAX.2",
        };
        let mut r = KinkRegistry::default();
        r.register(band).unwrap();
        assert!(r.register(band).is_err());
        assert!(r.register(KinkDecl { name: "TAX.other", ..band }).is_err(), "one source's points on a position once");
        let none = KinkDecl { name: "SOC.means", source: KinkSource::Rule("SOC.benefit"), points: 0, ..band };
        assert!(r.register(none).is_err(), "a kink of no points");
        assert_eq!(r.on(KinkOn::Position("TAX.income_to_date")).count(), 1);
        assert_eq!(r.on(KinkOn::PerMember("TAX.income_to_date")).count(), 0);
    }
}
