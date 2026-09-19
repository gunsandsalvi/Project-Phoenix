//! The ontology register: every store a module keeps, declared for what it is.
//!
//! It is to categories what `params` is to numbers. A module keeps state between its phases, and
//! the question nobody could answer was which of those stores are FACTS ABOUT THE WORLD that the
//! kernel should own, and which are a module's own scratch. An undeclared store is refused at the
//! read, so the answer cannot be avoided by not writing it down.
//!
//! Its count of HOMELESS nouns is the honest measure of how much ontology is missing — a noun
//! is a fact about the world, so it belongs in a kernel store, and one that has not got there yet
//! names the plan item that will give it a home. The count must fall; it is not a number to
//! tolerate.

use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sort {
    /// A FACT ABOUT THE WORLD, which belongs in a kernel store. Until it is there it names the
    /// plan item that gives it a home, and it is HOMELESS — which is what the count counts.
    Noun { home: Option<String> },
    /// A counter or a plan within one module's own phase, which does not survive in any sense a
    /// reader could use. It is not a fact about the world.
    Working,
    /// Weather, geography, the grid: given, not decided by anybody in this world.
    Physics,
}

pub struct NounDecl {
    pub name: String,
    pub sort: Sort,
    /// What it holds, in a reader's words.
    pub holds: String,
    /// Why it is the sort it is. A noun with no reason is a store nobody classified.
    pub why: String,
}

#[derive(Default)]
pub struct Nouns {
    at: HashMap<String, usize>,
    sort: Vec<Sort>,
    holds: Vec<String>,
    why: Vec<String>,
}

impl Nouns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn declare(&mut self, d: NounDecl) {
        assert!(!self.at.contains_key(&d.name), "Law 4: the store {} is declared twice", d.name);
        assert!(!d.why.is_empty(), "Law 16: {} is declared with no reason", d.name);
        if let Sort::Noun { home: Some(item) } = &d.sort {
            assert!(!item.is_empty(), "a noun's home names the item that gives it one");
        }
        self.at.insert(d.name.clone(), self.sort.len());
        self.sort.push(d.sort);
        self.holds.push(d.holds);
        self.why.push(d.why);
    }

    /// The read a module's store goes through. An undeclared store is refused HERE, at the read,
    /// because that is the moment the omission exists.
    pub fn sort_of(&self, name: &str) -> &Sort {
        match self.at.get(name) {
            Some(&at) => &self.sort[at],
            None => panic!(
                "Law 15: the store {name} is not declared — every store is a noun, a working store or physics"
            ),
        }
    }

    /// The nouns with no kernel home yet, in order. This count is the measure, and it must fall.
    pub fn homeless(&self) -> Vec<(&str, &str)> {
        let mut out: Vec<(&str, &str)> = Vec::new();
        for (name, &at) in &self.at {
            if let Sort::Noun { home: Some(item) } = &self.sort[at] {
                out.push((name.as_str(), item.as_str()));
            }
        }
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    pub fn len(&self) -> usize {
        self.sort.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sort.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(name: &str, sort: Sort) -> NounDecl {
        NounDecl {
            name: name.to_string(),
            sort,
            holds: "something".to_string(),
            why: "because the test says so".to_string(),
        }
    }

    #[test]
    fn the_homeless_count_is_the_measure_and_a_settled_noun_is_not_in_it() {
        let mut n = Nouns::new();
        n.declare(decl("control.advisory", Sort::Working));
        n.declare(decl("weather.today", Sort::Physics));
        n.declare(decl("firm.order.book", Sort::Noun { home: Some("13k".to_string()) }));
        n.declare(decl("register.holdings", Sort::Noun { home: None }));
        assert_eq!(n.len(), 4);
        // Only the noun that has not reached a kernel store is homeless.
        assert_eq!(n.homeless(), vec![("firm.order.book", "13k")]);
    }

    #[test]
    #[should_panic(expected = "is not declared")]
    fn an_undeclared_store_is_refused_at_the_read() {
        let n = Nouns::new();
        n.sort_of("somebody.kept.this.quietly");
    }

    #[test]
    #[should_panic(expected = "is declared twice")]
    fn one_store_has_one_declaration() {
        let mut n = Nouns::new();
        n.declare(decl("x", Sort::Working));
        n.declare(decl("x", Sort::Working));
    }
}
