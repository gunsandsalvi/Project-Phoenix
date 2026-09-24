use syn::visit::{self, Visit};
use syn::{ExprStruct, Field, ImplItemFn, ItemImpl, Type, Visibility};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-29";
const ACCT: &str = "phx-acct";
/// The one file that makes an equity event or opens an equity account.
const EQUITY_FILE: &str = "crates/kernel/phx-acct/src/equity.rs";
const STATEMENT: &str = "Statement";
const MADE_IN_EQUITY: &[&str] = &["EquityEvent", "EquityAccount"];
const ACCOUNTS: &str = "EquityAccounts";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source, c.name == ACCT));
        }
    }
    breaches
}

fn check(source: &Source, acct: bool) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { acct, equity_file: source.path == EQUITY_FILE, in_accounts: false, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// Statements kept in a field, equity events or accounts made outside the one file that makes them, and a way to
/// write an equity account opened to other crates.
struct Finder {
    acct: bool,
    equity_file: bool,
    in_accounts: bool,
    found: Vec<(usize, String)>,
}

/// Whether a type names `name` anywhere in it: a field of `Vec<Statement>` keeps statements as much as one of
/// `Statement`.
fn mentions(ty: &Type, name: &str) -> bool {
    struct Names<'n> {
        name: &'n str,
        seen: bool,
    }
    impl<'ast> Visit<'ast> for Names<'_> {
        fn visit_path_segment(&mut self, s: &'ast syn::PathSegment) {
            self.seen |= s.ident == self.name;
            visit::visit_path_segment(self, s);
        }
    }
    let mut n = Names { name, seen: false };
    n.visit_type(ty);
    n.seen
}

fn self_named(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == name))
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_field(&mut self, f: &'ast Field) {
        if mentions(&f.ty, STATEMENT) {
            let line = f.ident.as_ref().map_or(1, |i| attrs::line(i.span()));
            self.found.push((line, "a statement kept in a field: a statement is a read, never stored".to_owned()));
        }
        visit::visit_field(self, f);
    }

    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if let Some(last) = e.path.segments.last()
            && MADE_IN_EQUITY.iter().any(|n| last.ident == n)
            && !self.equity_file
        {
            self.found.push((
                attrs::line(last.ident.span()),
                format!(
                    "`{}` made outside the accounts' equity, which alone makes one from a declared effect",
                    last.ident
                ),
            ));
        }
        visit::visit_expr_struct(self, e);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        let was = self.in_accounts;
        self.in_accounts = self.acct && item.trait_.is_none() && self_named(&item.self_ty, ACCOUNTS);
        visit::visit_item_impl(self, item);
        self.in_accounts = was;
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let writes = f.sig.receiver().is_some_and(|r| matches!(r.kind, syn::ReceiverKind::Reference(_, _, Some(_))));
        if self.in_accounts && writes && matches!(f.vis, Visibility::Public(_)) {
            self.found.push((
                attrs::line(f.sig.ident.span()),
                "an equity account written from outside the accounts: its writes are the crate's own".to_owned(),
            ));
        }
        visit::visit_impl_item_fn(self, f);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::visit::Visit;

    use super::Finder;

    fn found(code: &str, acct: bool, equity_file: bool) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { acct, equity_file, in_accounts: false, found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn equity_moves_only_through_the_accounts() {
        assert_eq!(found("struct Report { last: Missing<Statement> }", false, false), 1, "a statement kept");
        assert_eq!(found("struct Report { last: Vec<Statement> }", true, false), 1);
        let made = "fn f() { let e = EquityEvent { party, kind, amount }; }";
        assert_eq!(found(made, true, false), 1, "an event made beside the declared effects");
        assert_eq!(found(made, true, true), 0, "the equity file makes its own");
        assert_eq!(found("impl EquityAccounts { pub fn set(&mut self) {} }", true, true), 1, "a write opened");
        assert_eq!(found("impl EquityAccounts { pub(crate) fn post(&mut self) {} }", true, true), 0);
        assert_eq!(found("impl EquityAccounts { pub fn of(&self) {} }", true, true), 0, "reads are open");
    }
}
