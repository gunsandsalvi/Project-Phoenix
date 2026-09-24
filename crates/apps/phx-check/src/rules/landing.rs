use syn::visit::{self, Visit};
use syn::{ExprCall, ExprMethodCall, ExprStruct, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-32";
const POP: &str = "phx-pop";
const LEDGER: &str = "phx-ledger";
/// The population's splits and landings: a split made at an apply sub-step, a landing at 10b, each by phx-pop alone
/// until the world's apply and 10b call it through the population's own entry points.
const PARTS: &[&str] = &[
    "split",
    "split_batch",
    "land",
    "join",
    "join_batch",
    "rekey_flagged",
    "take_whole",
    "set_key",
    "promote",
    "demote",
    "sweep",
];
/// The ledger's moves of a cell's rows and holdings into a part and back, which only the population's splits and
/// landings ask for.
const MOVES: &[&str] = &[
    "detach_row",
    "detach_rows",
    "detach_rows_batch",
    "attach_row",
    "attach_rows",
    "detach_holding",
    "attach_holding",
    "attach_holding_as_lot",
    "detach_lots_pooled",
    "swap_holders",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name != POP && c.name != LEDGER) {
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

/// Splits, landings and the moves they make, and a `Landing` built, outside the population.
struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_call(&mut self, e: &'ast ExprCall) {
        if let syn::Expr::Path(p) = e.func.as_ref()
            && let Some(last) = p.path.segments.last()
            && p.path.segments.iter().any(|s| s.ident == "phx_pop")
            && PARTS.iter().any(|n| last.ident == n)
        {
            let message = format!("`{}` splits or lands members, which only the population does", last.ident);
            self.found.push((attrs::line(last.ident.span()), message));
        }
        visit::visit_expr_call(self, e);
    }

    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if MOVES.iter().any(|n| e.method == n) {
            let message = format!("`{}` moves a cell's rows for a part, which only the population asks", e.method);
            self.found.push((attrs::line(e.method.span()), message));
        }
        visit::visit_expr_method_call(self, e);
    }

    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if e.path.segments.last().is_some_and(|s| s.ident == "Landing") {
            let message = "a `Landing` built outside the population, the one way a cell's totals change at 10b";
            self.found.push((attrs::line(e.brace_token.span.open()), message.to_owned()));
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
    use syn::visit::Visit;

    use super::Finder;

    fn found(code: &str) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn members_split_and_land_by_the_population_alone() {
        assert_eq!(found("fn f() { phx_pop::landing::land(&mut ctx, &mut index, parts); }"), 1);
        assert_eq!(found("fn f() { let p = phx_pop::split::split(&mut c, s, id, &spec, d); }"), 1);
        assert_eq!(found("fn f() { books.ledger.attach_row(t, 0, s, row); }"), 1);
        assert_eq!(found("fn f() { l.detach_rows_batch(t, 0, s, &plans); }"), 1);
        assert_eq!(found("fn f() { phx_pop::join::join_batch(l, t, 0, s, parts); }"), 1);
        assert_eq!(found("fn f() { let l = Landing { part, target }; }"), 1);
        assert_eq!(found("fn f() { let s = text.split(','); }"), 0, "another crate's own word");
        assert_eq!(found("#[cfg(test)] mod tests { fn f() { phx_pop::landing::land(); } }"), 0);
    }
}
