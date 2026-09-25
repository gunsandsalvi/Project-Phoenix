use syn::visit::{self, Visit};
use syn::{ExprMethodCall, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-34";
/// An agent's writes past its beginning, and the crates that may make each: the population itself, the openings that
/// form households, and the world, which seats the player's twins.
const WRITES: &[(&str, &[&str])] = &[
    ("set_persons", &["phx-pop", "sys-dem", "phx-world"]),
    ("set_attachments", &["phx-pop", "sys-dem", "phx-world"]),
    ("take_twin", &["phx-pop", "phx-world"]),
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source, &c.name));
        }
    }
    breaches
}

fn check(source: &Source, crate_name: &str) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { crate_name, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// Calls of an agent's writes from a crate not allowed them.
struct Finder<'a> {
    crate_name: &'a str,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if let Some((method, allowed)) = WRITES.iter().find(|(w, _)| e.method == w)
            && !allowed.contains(&self.crate_name)
        {
            let message = format!("`{method}` writes an agent, which only {} do", allowed.join(", "));
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

    fn found(code: &str, crate_name: &str) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { crate_name, found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn agents_written_where_allowed_alone() {
        assert_eq!(found("fn f(t: &mut T) { t.take_twin(s); t.set_persons(s, &w); }", "phx-world"), 0);
        assert_eq!(found("fn f(t: &mut T) { t.set_persons(s, &w); }", "sys-dem"), 0);
        assert_eq!(found("fn f(t: &mut T) { t.take_twin(s); }", "sys-dem"), 1, "only the world seats twins");
        assert_eq!(found("fn f(t: &mut T) { t.set_attachments(s, &w); }", "sys-frm"), 1);
        assert_eq!(found("#[cfg(test)] mod tests { fn f(t: &mut T) { t.take_twin(s); } }", "sys-frm"), 0);
    }
}
