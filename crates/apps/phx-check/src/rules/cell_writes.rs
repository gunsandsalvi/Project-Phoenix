use syn::visit::{self, Visit};
use syn::{ExprMethodCall, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-30";
const POP: &str = "phx-pop";
/// A cell table's column writes, which only the population's own typed writes make.
const CELL_WRITES: &[&str] = &[
    "set_position",
    "set_rate",
    "set_profile",
    "set_sig",
    "set_exposure",
    "set_attention",
    "set_key",
    "set_weight",
    "rekey",
    "make_individual",
    "edit_ext",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name != POP) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
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

/// Calls of a cell table's column writes outside the population.
struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if CELL_WRITES.iter().any(|w| e.method == w) {
            let message = format!("`{}` writes a cell's column, which only phx-pop's typed writes do", e.method);
            self.found.push((attrs::line(e.method.span()), message));
        }
        visit::visit_expr_method_call(self, e);
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
    use syn::visit::Visit;

    use super::Finder;

    fn found(code: &str) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn cells_are_written_by_the_population_alone() {
        assert_eq!(found("fn f(t: &mut CellTable) { t.set_position(s, 0, 5); t.rekey(s, k, l); }"), 2);
        assert_eq!(found("fn f(t: &CellTable) { let p = t.position(s, 0); }"), 0, "reads are open");
        assert_eq!(found("#[cfg(test)] mod tests { fn f(t: &mut CellTable) { t.set_weight(s, w); } }"), 0);
    }
}
