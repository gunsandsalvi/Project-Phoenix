//! PC-98: on a day's path a primitive is read by its handle, and a kind, family, reason or market by its typed
//! handle, never by a string: no register read by name, no comparison with a name, no test of a name's prefix. Names
//! are bound once, in functions marked `#[opening]`, whose bodies are not read.

use syn::visit::{self, Visit};
use syn::{Attribute, BinOp, Expr, ExprBinary, ExprMethodCall, ImplItemFn, ItemFn, ItemImpl, ItemMod, Lit};

use super::hot_paths::is_hot;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::Workspace;

pub const RULE: &str = "PC-98";

/// The register's readers by name, and the receivers a register is held as.
const READERS: &[&str] = &["count", "fixed", "table1", "table2", "products", "stored_by_id", "value"];
const REGISTERS: &[&str] = &["register", "reg"];
/// Tests of a name against a literal.
const NAME_TESTS: &[&str] = &["starts_with", "ends_with", "contains"];
const OPENING: &str = "opening";

/// Every read by name PC-98 finds in the hot set, and a breach for each source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| is_hot(c, s)) {
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
            let mut finder = Finder::default();
            finder.visit_file(file);
            sites.extend(finder.found.into_iter().map(|(line, item, what)| Found {
                path: source.path.clone(),
                message: format!("{what} on the day's path: read by its handle, bound in the opening"),
                item,
                what,
                line,
            }));
        }
    }
    (sites, breaches)
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let (sites, mut breaches) = found(ws);
    match Exceptions::load(ws, RULE) {
        Ok(ex) => breaches.extend(ex.judge(&ws.ratchets, sites)),
        Err(b) => breaches.push(b),
    }
    breaches
}

#[derive(Debug, Default)]
struct Finder {
    found: Vec<(usize, String, String)>,
    items: Vec<String>,
}

fn is_str(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(l.lit, Lit::Str(_)),
        Expr::Reference(r) => is_str(&r.expr),
        Expr::Paren(p) => is_str(&p.expr),
        _ => false,
    }
}

/// The name a receiver is held under: a path's or a field's last name.
fn held_as(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Expr::Field(f) => match &f.member {
            syn::Member::Named(n) => Some(n.to_string()),
            syn::Member::Unnamed(_) => None,
        },
        Expr::Reference(r) => held_as(&r.expr),
        _ => None,
    }
}

fn is_opening(attrs_: &[Attribute]) -> bool {
    attrs_.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == OPENING))
}

impl Finder {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
    }

    fn push(&mut self, line: usize, what: String) {
        self.found.push((line, self.item(), what));
    }

    fn within(&mut self, name: String, visit: impl FnOnce(&mut Finder)) {
        self.items.push(name);
        visit(self);
        self.items.pop();
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.within(format!("mod {}", item.ident), |f| visit::visit_item_mod(f, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        let name = match &*item.self_ty {
            syn::Type::Path(p) => p.path.segments.last().map_or_else(String::new, |s| s.ident.to_string()),
            _ => "impl".to_owned(),
        };
        self.within(name, |f| visit::visit_item_impl(f, item));
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) && !is_opening(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_item_fn(f, item));
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !attrs::is_test(&item.attrs) && !is_opening(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_impl_item_fn(f, item));
        }
    }

    fn visit_expr_binary(&mut self, e: &'ast ExprBinary) {
        if matches!(e.op, BinOp::Eq(_) | BinOp::Ne(_)) && (is_str(&e.left) || is_str(&e.right)) {
            let line = match e.op {
                BinOp::Eq(t) => attrs::line(t.spans[0]),
                BinOp::Ne(t) => attrs::line(t.spans[0]),
                _ => 1,
            };
            self.push(line, "a comparison with a name".to_owned());
        }
        visit::visit_expr_binary(self, e);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();
        let line = attrs::line(call.method.span());
        let on_register = held_as(&call.receiver).is_some_and(|n| REGISTERS.contains(&n.as_str()));
        let first_is_name = call.args.first().is_some_and(is_str);
        if on_register && (READERS.contains(&method.as_str()) || first_is_name) {
            self.push(line, format!("`register.{method}`"));
        } else if NAME_TESTS.contains(&method.as_str()) && first_is_name {
            self.push(line, format!("`.{method}` of a name"));
        }
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::{Finder, run};
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};
    use syn::visit::Visit;

    fn whats(text: &str) -> Vec<String> {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut f = Finder::default();
        f.visit_file(&file);
        f.found.into_iter().map(|(_, _, w)| w).collect()
    }

    #[test]
    fn register_reads_by_name_are_refused() {
        let found = whats(
            "fn day(c: &C) { let w = c.register.fixed(ID); let t = reg.table1(\"SRV.x\"); \
                           let s = c.register.lookup(\"FRM.markup\"); let ok = c.handles.fixed(ID); }",
        );
        assert_eq!(found, ["`register.fixed`", "`register.table1`", "`register.lookup`"]);
    }

    #[test]
    fn kind_name_comparisons_are_refused() {
        let found = whats(
            "fn day(c: &C) { let f = c.names.iter().position(|n| *n == \"firm\"); \
                           if name != \"bank\" {} if code.starts_with(\"LAB.\") {} if v.contains(&x) {} }",
        );
        assert_eq!(found, ["a comparison with a name", "a comparison with a name", "`.starts_with` of a name"]);
    }

    #[test]
    fn opening_functions_are_admitted() {
        let text = "impl Core { #[opening] fn bind(&mut self) { self.firm = self.names.iter().position(|n| *n == \"firm\"); } }\n\
                    #[phx_macros::opening] fn open(r: &R) { let w = r.register.fixed(ID); }";
        let c = with_source(krate("phx-world", Layer::Assembly), "src/core_open.rs", text);
        assert!(run(&Workspace::new(vec![c])).is_empty());
    }

    #[test]
    fn messages_are_not_comparisons() {
        let found = whats(
            "fn day() { violation!(clause = \"REP.1\", \"a record too long\"); \
                           let e = format!(\"no {}\", \"firm\"); return Err(\"refused\".to_owned()); }",
        );
        assert!(found.is_empty(), "{found:?}");
    }
}
