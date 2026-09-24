use syn::visit::{self, Visit};
use syn::{ExprCall, ExprMethodCall, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-31";
const POP: &str = "phx-pop";
/// The population's screening, which runs at 3b over the agenda's rows alone; the map's tile and region processes
/// are drawn by phx-geo at 3a with its own draws.
const SCREENING: &[&str] = &["screen_candidate", "screen_daily", "candidate", "candidate_outcomes", "next_booking"];

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

/// Calls of the population's screening outside it.
struct Finder {
    found: Vec<(usize, String)>,
}

impl Finder {
    fn note(&mut self, name: &syn::Ident) {
        if SCREENING.iter().any(|s| name == s) {
            let message = format!("`{name}` screens members, which only phx-pop's 3b screen does");
            self.found.push((attrs::line(name.span()), message));
        }
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_call(&mut self, e: &'ast ExprCall) {
        if let syn::Expr::Path(p) = e.func.as_ref()
            && let Some(last) = p.path.segments.last()
            && p.path.segments.len() > 1
            && p.path.segments.iter().any(|s| s.ident == "phx_pop" || s.ident == "envelope" || s.ident == "screen")
        {
            self.note(&last.ident);
        }
        visit::visit_expr_call(self, e);
    }

    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if e.method == "screen_candidate" || e.method == "screen_daily" {
            self.note(&e.method);
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
    fn members_are_screened_by_the_population_alone() {
        assert_eq!(found("fn f() { phx_pop::screen::screen_daily(t, s, p, d, r, c); }"), 1);
        assert_eq!(found("fn f() { let b = envelope::next_booking(d, day, p, w); }"), 1);
        assert_eq!(found("fn f() { let c = market.candidate(x); }"), 0, "another crate's own word");
        assert_eq!(found("#[cfg(test)] mod tests { fn f() { phx_pop::screen::screen_daily(); } }"), 0);
    }
}
