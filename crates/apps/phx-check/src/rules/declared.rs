//! PC-101: what the world keeps is declared as what it is. An aggregate a writer maintains is marked
//! `#[maintained(writer = path)]`, is an integer, so a sum rebuilt from its rows equals it, and is never read by the
//! audit, which recounts from the source rows so a maintained sum cannot hide the fault the audit is there to find.
//! And a value computed once a day per party, its function marked `#[per_day]`, is reached only through the day's
//! cache: the function is passed to it as a path and never called, so no path computes it twice in a day. And every
//! index left out of a save names the function that rebuilds it after a load: `#[saved(skip, rebuild = path)]`.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprField, ExprMethodCall, ImplItemFn, ItemFn, ItemMod, ItemStruct, Member, Type};

use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
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
/// The mark of a value computed once a day per party.
const PER_DAY: &str = "per_day";

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
    breaches.extend(per_day(ws));
    let (skips, unread) = found(ws);
    breaches.extend(unread);
    match Exceptions::load(ws, RULE) {
        Ok(ex) => breaches.extend(ex.judge(&ws.ratchets, skips)),
        Err(b) => breaches.push(b),
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

/// Every field left out of a save without naming its rebuild, and a breach for each source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            if attrs::is_test(&file.attrs) {
                continue;
            }
            let mut skips = Skips::default();
            skips.visit_file(file);
            sites.extend(skips.found.into_iter().map(|(line, item, field)| Found {
                path: source.path.clone(),
                message: format!("`{field}` left out of the save without naming its rebuild: `skip, rebuild = path`"),
                what: format!("skip `{field}`"),
                item,
                line,
            }));
        }
    }
    (sites, breaches)
}

/// Fields marked `#[saved(skip)]` with no `rebuild =`, by struct.
#[derive(Debug, Default)]
struct Skips {
    found: Vec<(usize, String, String)>,
}

impl<'ast> Visit<'ast> for Skips {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        for (i, field) in item.fields.iter().enumerate() {
            let bare = field.attrs.iter().filter(|a| a.path().is_ident("saved")).any(|a| {
                let (mut skip, mut rebuild) = (false, false);
                let _ = a.parse_nested_meta(|m| {
                    skip |= m.path.is_ident("skip");
                    if m.path.is_ident("rebuild") {
                        rebuild = true;
                        let _ = m.value().and_then(syn::parse::ParseBuffer::parse::<syn::Path>);
                    }
                    Ok(())
                });
                skip && !rebuild
            });
            if bare {
                let (line, name) = match &field.ident {
                    Some(ident) => (attrs::line(ident.span()), ident.to_string()),
                    None => (attrs::line(item.ident.span()), i.to_string()),
                };
                self.found.push((line, item.ident.to_string(), name));
            }
        }
        visit::visit_item_struct(self, item);
    }
}

/// Every call of a function marked `#[per_day]` in a world crate: the function is named only as a path handed to the
/// day's cache.
fn per_day(ws: &Workspace) -> Vec<Breach> {
    let mut marked = PerDay::default();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            if let Ok(file) = &source.file {
                marked.visit_file(file);
            }
        }
    }
    if marked.names.is_empty() {
        return Vec::new();
    }
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            let Ok(file) = &source.file else { continue };
            let mut calls = Calls { names: &marked.names, found: Vec::new() };
            calls.visit_file(file);
            breaches.extend(calls.found.into_iter().map(|(line, name)| {
                let message = format!("`{name}` called directly: a per-day value is reached through the day's cache");
                Breach::new(RULE, &source.path, line, message)
            }));
        }
    }
    breaches
}

/// The functions marked `#[per_day]`, by name.
#[derive(Debug, Default)]
struct PerDay {
    names: BTreeSet<String>,
}

fn marked_per_day(attrs_: &[syn::Attribute]) -> bool {
    attrs_.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == PER_DAY))
}

impl<'ast> Visit<'ast> for PerDay {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if marked_per_day(&item.attrs) {
            self.names.insert(item.sig.ident.to_string());
        }
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if marked_per_day(&item.attrs) {
            self.names.insert(item.sig.ident.to_string());
        }
        visit::visit_impl_item_fn(self, item);
    }
}

/// The calls of the marked functions, outside tests.
struct Calls<'a> {
    names: &'a BTreeSet<String>,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Calls<'_> {
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

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(p) = &*call.func
            && let Some(seg) = p.path.segments.last()
            && self.names.contains(&seg.ident.to_string())
        {
            self.found.push((attrs::line(seg.ident.span()), seg.ident.to_string()));
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if self.names.contains(&call.method.to_string()) {
            self.found.push((attrs::line(call.method.span()), call.method.to_string()));
        }
        visit::visit_expr_method_call(self, call);
    }
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

    fn per_day_breaches(text: &str) -> Vec<String> {
        let c = with_source(krate("phx-acct", Layer::Kernel), "src/cost.rs", text);
        run(&Workspace::new(vec![c])).into_iter().map(|b| b.message).collect()
    }

    const UNIT_COST: &str = "impl Firm { #[per_day] pub fn unit_cost(&self, day: Day) -> Money { self.costs } }\n";

    #[test]
    fn per_day_only_through_the_cache() {
        let text =
            format!("{UNIT_COST}fn price(f: &Firm, d: Day) -> Money {{ f.unit_cost(d) + Firm::unit_cost(f, d) }}");
        assert_eq!(per_day_breaches(&text).len(), 2, "a method call and a path call");
    }

    #[test]
    fn per_day_path_is_admitted() {
        let text = format!(
            "{UNIT_COST}fn price(c: &mut DayCached<Money>, d: Day) -> Money {{ c.get_or(d, Firm::unit_cost) }}"
        );
        assert!(per_day_breaches(&text).is_empty());
    }

    #[test]
    fn skip_must_name_rebuild() {
        let text = "#[derive(Saved)] struct Core { rows: Vec<u8>, #[saved(skip)] space: Space, \
                    #[saved(skip, rebuild = Self::reindex)] index: Vec<u32> }";
        let found: Vec<String> =
            super::found(&Workspace::new(vec![with_source(krate("phx-world", Layer::Assembly), "src/core.rs", text)]))
                .0
                .into_iter()
                .map(|f| f.what)
                .collect();
        assert_eq!(found, ["skip `space`"]);
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
