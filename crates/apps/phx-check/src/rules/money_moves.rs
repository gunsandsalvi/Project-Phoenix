use syn::visit::{self, Visit};
use syn::{ExprStruct, ImplItemFn, ItemFn, ItemMod, Visibility};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-25";
const LEDGER: &str = "phx-ledger";
/// The ledger's writers of balances, counts, rows and holdings, which only its apply routine calls.
const WRITERS: &[&str] = &[
    "add_row",
    "remove_row",
    "set_count",
    "set_words",
    "set_record",
    "change_issued",
    "acquire",
    "dispose",
    "set_state",
];
/// What only the apply routine builds: the record of a settled leg it feeds the audit.
const DIGEST: &str = "LegDigest";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        let ledger = c.name == LEDGER;
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source, ledger));
        }
    }
    breaches
}

fn check(source: &Source, ledger: bool) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { ledger, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder {
    ledger: bool,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        if self.ledger && matches!(item.vis, Visibility::Public(_)) && WRITERS.iter().any(|w| item.sig.ident == w) {
            let message =
                format!("`{}` writes the books and is public; only the apply routine may call it", item.sig.ident);
            self.found.push((attrs::line(item.sig.ident.span()), message));
        }
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if !self.ledger
            && let Some(id) = e.path.segments.last().map(|s| &s.ident)
            && id == DIGEST
        {
            self.found
                .push((attrs::line(id.span()), "a settled leg's digest is built only by the apply routine".to_owned()));
        }
        visit::visit_expr_struct(self, e);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
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

    fn found(code: &str, ledger: bool) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { ledger, found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn money_moves_only_through_apply() {
        assert_eq!(found("impl L { pub fn set_words(&mut self) {} }", true), 1, "a public writer");
        assert_eq!(found("impl L { pub(crate) fn set_words(&mut self) {} }", true), 0, "a crate writer");
        assert_eq!(
            found("fn f() { let d = LegDigest { party, account, denom, qty, before, paired, money }; }", false),
            1
        );
        assert_eq!(
            found("fn f() { let d = LegDigest { party, account, denom, qty, before, paired, money }; }", true),
            0
        );
    }
}
