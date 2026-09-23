use std::any::Any;
use std::marker::PhantomData;

use phx_macros::clause;
use phx_num::violation;

/// A rule handle's signature, declared in an interface crate with the system that implements it: a pure function
/// from inputs any system may read to an answer.
#[derive(Debug)]
pub struct RuleSig<I, O> {
    pub name: &'static str,
    pub implementer: &'static str,
    marker: PhantomData<fn(&I) -> O>,
}

impl<I, O> Clone for RuleSig<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for RuleSig<I, O> {}

impl<I, O> RuleSig<I, O> {
    #[must_use]
    pub const fn new(name: &'static str, implementer: &'static str) -> RuleSig<I, O> {
        RuleSig { name, implementer, marker: PhantomData }
    }
}

#[derive(Debug)]
struct Implemented {
    name: &'static str,
    system: &'static str,
    function: Box<dyn Any + Send + Sync>,
}

/// Every rule handle's implementation, by name.
#[derive(Debug, Default)]
pub struct RuleTable {
    implemented: Vec<Implemented>,
}

impl RuleTable {
    /// A system's implementation of a signature.
    pub fn implement<I: 'static, O: 'static>(&mut self, system: &'static str, sig: RuleSig<I, O>, f: fn(&I) -> O) {
        self.implemented.push(Implemented { name: sig.name, system, function: Box::new(f) });
        self.implemented.sort_by_key(|i| i.name);
    }

    /// Every declared signature implemented once, by its declared implementer, and nothing implemented undeclared.
    ///
    /// # Errors
    /// Each signature with no implementation or two, or one by another system.
    #[clause("Law 4")]
    pub fn check(&self, declared: &[(&'static str, &'static str)]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for (name, implementer) in declared {
            let by: Vec<&str> = self.implemented.iter().filter(|i| i.name == *name).map(|i| i.system).collect();
            if by.as_slice() != [*implementer] {
                errors.push(format!("rule `{name}` is {implementer}'s and implemented by {by:?}"));
            }
        }
        for i in self.implemented.iter().filter(|i| !declared.iter().any(|(n, _)| *n == i.name)) {
            errors.push(format!("{} implements `{}`, which no interface declares", i.system, i.name));
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    /// The rule's answer for an input.
    pub fn call<I: 'static, O: 'static>(&self, sig: RuleSig<I, O>, input: &I) -> O {
        let at = self.implemented.binary_search_by_key(&sig.name, |i| i.name).ok();
        let f = at.and_then(|i| self.implemented.get(i)).and_then(|i| i.function.downcast_ref::<fn(&I) -> O>());
        let Some(f) = f else {
            violation!(clause = "Law 4", "a rule handle with no implementation of its signature");
        };
        f(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleSig, RuleTable};

    const TAX: RuleSig<[i64; 2], i64> = RuleSig::new("TAX.income_tax", "TAX");

    fn fifth(income: &[i64; 2]) -> i64 {
        (income[0] + income[1]) / 5
    }

    #[test]
    fn rule_sig_needs_one_implementer() {
        let declared = [(TAX.name, TAX.implementer)];
        let mut none = RuleTable::default();
        assert!(none.check(&declared).is_err());
        none.implement("TAX", TAX, fifth);
        assert_eq!(none.check(&declared), Ok(()));
        assert_eq!(none.call(TAX, &[60, 40]), 20);
        let mut twice = RuleTable::default();
        twice.implement("TAX", TAX, fifth);
        twice.implement("SOC", TAX, fifth);
        assert!(twice.check(&declared).is_err());
        let mut other = RuleTable::default();
        other.implement("SOC", TAX, fifth);
        assert!(other.check(&declared).is_err(), "implemented by a system not its own");
    }
}
