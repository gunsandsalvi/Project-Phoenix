use syn::visit::{self, Visit};
use syn::{ExprMethodCall, ItemImpl};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-26";
/// What the opening's code may not call: the day's ways of changing the books and the facts, since the opening
/// writes only through its contributions' opening writes.
const DAY_WRITERS: &[&str] = &["apply", "settle", "pay_dues", "contract_process", "write_fact"];
const CONTRIBUTION: &str = "Contribution";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
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
    let mut opening = Opening::default();
    opening.visit_file(file);
    if !opening.contributes {
        return Vec::new();
    }
    opening.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// A file of the opening's code: one that implements a contribution, and every call in it that writes the day's way.
#[derive(Default)]
struct Opening {
    contributes: bool,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Opening {
    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        if item.trait_.as_ref().and_then(|(p, _)| p.segments.last()).is_some_and(|s| s.ident == CONTRIBUTION) {
            self.contributes = true;
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if DAY_WRITERS.iter().any(|w| e.method == w) {
            let message = format!("the opening calls `{}`; it writes only through its opening writes", e.method);
            self.found.push((attrs::line(e.method.span()), message));
        }
        visit::visit_expr_method_call(self, e);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Opening;
    use syn::visit::Visit;

    fn found(code: &str) -> Option<usize> {
        let file = syn::parse_file(code).unwrap();
        let mut o = Opening::default();
        o.visit_file(&file);
        o.contributes.then_some(o.found.len())
    }

    #[test]
    fn the_opening_writes_only_through_opening_writes() {
        let opening = "impl Contribution for P { fn contribute(&self, o: &mut Opening) { b.open(r, legs, 1, rep); } }";
        assert_eq!(found(opening), Some(0));
        let settles = "impl Contribution for P { fn contribute(&self, o: &mut Opening) { b.apply(at, i, a); } }";
        assert_eq!(found(settles), Some(1));
        let helper = "impl Contribution for P {} fn pay(b: &mut Books) { b.pay_dues(d, c, a); }";
        assert_eq!(found(helper), Some(1), "a helper of the opening's file is the opening's code");
        assert_eq!(found("fn day(b: &mut Books) { b.apply(at, i, a); }"), None, "the day's code is not the opening's");
    }
}
