//! PC-22: the audit reads the world and never changes it, so what it finds is what the world holds: `phx-audit` holds
//! the world's stores by shared reference only. Its reads of maintained aggregates are PC-101's.

use syn::Type;
use syn::visit::{self, Visit};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-22";
const AUDIT: &str = "phx-audit";

/// The world's stores, which the audit may hold only by shared reference.
const STORES: &[&str] = &[
    "Column",
    "Table",
    "SlotAlloc",
    "Parties",
    "KindStore",
    "EdgeTable",
    "ChunkArena",
    "BlockList",
    "BlockPool",
    "Region",
    "Persons",
    "DueWheel",
    "EventStore",
    "Register",
    "Calendar",
    "Core",
    "World",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let Some(audit) = ws.crates.iter().find(|c| c.name == AUDIT) else { return Vec::new() };
    audit.sources.iter().filter(|s| !s.is_test_or_bench()).flat_map(audit_breaches).collect()
}

fn named(ty: &Type, names: &[&str]) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| names.iter().any(|n| s.ident == n)))
}

/// The audit reaches no store mutably.
fn audit_breaches(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_type_reference(&mut self, r: &'ast syn::TypeReference) {
        if r.mutability.is_some() && named(&r.elem, STORES) {
            let line = attrs::line(r.and_token.span);
            self.found.push((line, "the audit holds a world store mutably".to_owned()));
        }
        visit::visit_type_reference(self, r);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn pc22_holds_stores_by_reference() {
        let audit = "fn recount(c: &Column<i64>, k: &KindStore<B>) -> i64 { 0 }\n\
                     fn repair(c: &mut Column<i64>, p: &mut Parties) {}\n\
                     #[cfg(test)]\nmod tests { fn t(d: &mut Column<u8>) {} }";
        let aud = with_source(krate("phx-audit", Layer::Kernel), "src/recount.rs", audit);
        let found: Vec<String> = run(&Workspace::new(vec![aud])).into_iter().map(|b| b.message).collect();
        assert_eq!(found, ["the audit holds a world store mutably", "the audit holds a world store mutably"]);
    }
}
