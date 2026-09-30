//! PC-101: what the world keeps is declared as what it is. An aggregate a writer maintains is marked
//! `#[maintained(writer = path)]`, is an integer, so a sum rebuilt from its rows equals it, and is never read by the
//! audit, which recounts from the source rows so a maintained sum cannot hide the fault the audit is there to find.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{ExprField, ItemFn, ItemMod, ItemStruct, Member, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

pub const RULE: &str = "PC-101";

/// The mark of a maintained aggregate.
const MAINTAINED: &str = "maintained";
/// The integer types a maintained aggregate may be, as the marker's derive admits them.
const INTEGERS: &[&str] = &[
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "Amount", "Count", "Money", "Qty", "QtyRaw", "PriceRaw",
    "Fixed",
];
/// The crate of the audit.
const AUDIT: &str = "phx-audit";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let mut names = BTreeSet::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut fields = Maintained::default();
            fields.visit_file(file);
            for (line, name, integer) in fields.found {
                if !integer {
                    let message = format!("maintained `{name}` is no integer: a rebuilt sum could differ from it");
                    breaches.push(Breach::new(RULE, &source.path, line, message));
                }
                names.insert(name);
            }
        }
    }
    if let Some(audit) = ws.crates.iter().find(|c| c.name == AUDIT) {
        for source in audit.sources.iter().filter(|s| !s.is_test_or_bench()) {
            let Ok(file) = &source.file else { continue };
            let mut reads = Reads { names: &names, found: Vec::new() };
            reads.visit_file(file);
            breaches.extend(reads.found.into_iter().map(|(line, name)| {
                let message = format!("the audit reads the maintained `{name}`: recount it from its source rows");
                Breach::new(RULE, &source.path, line, message)
            }));
        }
    }
    breaches
}

/// Every maintained field: its line, its name, and whether it is an integer.
#[derive(Debug, Default)]
struct Maintained {
    found: Vec<(usize, String, bool)>,
}

impl<'ast> Visit<'ast> for Maintained {
    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        for field in &item.fields {
            if !field.attrs.iter().any(|a| a.path().is_ident(MAINTAINED)) {
                continue;
            }
            let Some(ident) = &field.ident else { continue };
            let integer = match &field.ty {
                Type::Path(p) => p.path.segments.last().is_some_and(|s| INTEGERS.iter().any(|n| s.ident == n)),
                _ => false,
            };
            self.found.push((attrs::line(ident.span()), ident.to_string(), integer));
        }
        visit::visit_item_struct(self, item);
    }
}

/// The audit's reads of a maintained field, by name: a clash is resolved by renaming, never by an exception.
struct Reads<'a> {
    names: &'a BTreeSet<String>,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Reads<'_> {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_expr_field(&mut self, e: &'ast ExprField) {
        if let Member::Named(n) = &e.member
            && self.names.contains(&n.to_string())
        {
            self.found.push((attrs::line(n.span()), n.to_string()));
        }
        visit::visit_expr_field(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    fn breaches(ledger: &str, audit: &str) -> Vec<String> {
        let l = with_source(krate("phx-ledger", Layer::Kernel), "src/books.rs", ledger);
        let a = with_source(krate("phx-audit", Layer::Kernel), "src/recount.rs", audit);
        run(&Workspace::new(vec![l, a])).into_iter().map(|b| b.message).collect()
    }

    #[test]
    fn maintained_must_be_integer() {
        let ledger = "#[derive(Maintained)] struct Books { #[maintained(writer = post)] owed: i64, \
                      #[maintained(writer = post)] share: f64, #[maintained(writer = post)] wide: i128, rows: Vec<u8> }";
        let found = breaches(ledger, "");
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().all(|m| m.contains("is no integer")));
    }

    #[test]
    fn audit_may_not_read_maintained() {
        let ledger = "#[derive(Maintained)] struct Books { #[maintained(writer = post)] owed: i64, rows: Vec<i64> }";
        let audit = "fn recount(b: &Books) -> bool { b.rows.iter().sum::<i64>() == b.owed }\n\
                     #[cfg(test)] mod tests { fn t(b: &Books) { let _ = b.owed; } }";
        let found = breaches(ledger, audit);
        assert_eq!(found, ["the audit reads the maintained `owed`: recount it from its source rows"]);
    }
}
