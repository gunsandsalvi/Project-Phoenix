use syn::visit::{self, Visit};
use syn::{ExprPath, ExprStruct, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-24";
const LEDGER: &str = "phx-ledger";
/// The crate that declares the holder's lists, which the ledger alone fills.
const LISTS: &str = "phx-core";
/// The holder's lists the ledger alone writes: its rows, holdings, lots and named units.
const LIST_KINDS: &[&str] = &["RelationshipRows", "Holdings", "Lots", "NamedUnits"];
/// The cell tables' implementation of the ledger's holder interface, which maps each list the ledger names to where a
/// cell keeps it; what it writes, the ledger alone asks for.
const HOLDER_IMPLS: &[&str] = &["crates/kernel/phx-pop/src/holder.rs"];
/// The ledger's stored records, which only it builds.
const RECORDS: &[&str] =
    &["IndividualHolding", "CellHolding", "RelRow", "Lien", "NamedUnit", "InstrumentRow", "LineRow"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name != LEDGER && c.name != LISTS) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench() && !HOLDER_IMPLS.contains(&s.path.as_str())) {
            breaches.extend(check(source));
        }
    }
    breaches
}

fn check(source: &Source) -> Vec<Breach> {
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
    fn visit_expr_path(&mut self, e: &'ast ExprPath) {
        let segments: Vec<&syn::Ident> = e.path.segments.iter().map(|s| &s.ident).collect();
        if let [.., kind, list] = segments.as_slice()
            && *kind == "ListKind"
            && LIST_KINDS.iter().any(|l| *list == l)
        {
            let message = format!("`ListKind::{list}` is reached only through phx-ledger, its one writer");
            self.found.push((attrs::line(list.span()), message));
        }
        visit::visit_expr_path(self, e);
    }

    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if let Some(id) = e.path.segments.last().map(|s| &s.ident)
            && RECORDS.iter().any(|r| id == r)
        {
            self.found.push((attrs::line(id.span()), format!("`{id}` is built only by phx-ledger")));
        }
        visit::visit_expr_struct(self, e);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Finder;
    use syn::visit::Visit;

    fn found(code: &str) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn ledger_writes_only_in_ledger() {
        assert_eq!(found("fn f(t: &mut T) { t.append(s, ListKind::RelationshipRows, &w); }"), 1, "rows written");
        assert_eq!(found("fn f(t: &T) { t.read(s, phx_core::kind_tables::ListKind::Holdings); }"), 1, "a list read");
        assert_eq!(found("fn f() { let r = RelRow { line, count, record, point, role, flags }; }"), 1, "a row built");
        assert_eq!(found("fn f() { let l = Lien { key, units, to, chain }; }"), 1, "a lien built");
        assert_eq!(found("fn f(l: &mut Lines) { l.add_row(a, t, h, line, side, 0, 1, 0, w); }"), 0, "the ledger's own");
        assert_eq!(found("#[cfg(test)] mod t { fn f() { ListKind::Lots; } }"), 0, "tests are exempt");
    }
}
