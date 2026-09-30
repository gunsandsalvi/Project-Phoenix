//! PC-96: nothing on a day's path in the world or a system walks a whole kind, table, account column or holding
//! list except through the kernel's traversals — whose chunks are the unit and are counted — or a function declared
//! a sweep with its cycle or reason, so a day's cost follows its events, not the world's size. The kernel's own
//! crates implement the traversals and are not read.

use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMethodCall, ExprRange, ImplItemFn, ItemFn, ItemImpl, ItemMod, Lit, RangeLimits,
};

use super::hot_paths::is_hot;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::{Crate, Workspace};

pub const RULE: &str = "PC-96";

/// Calls that reach every row of a store; those named with no argument only when called with none, as a store's are.
const WALKS: &[&str] =
    &["live_slots", "live_every", "open_slots", "firm_slots", "deposits_of", "money_totals", "issuer_held"];
const BARE_WALKS: &[&str] = &["all", "totals", "money"];
/// The ends of a range from nothing that span a whole table.
const WHOLE: &[&str] = &["len", "count", "rows"];
/// The kernel's traversals: a walk inside what they are handed is theirs.
const TRAVERSALS: &[&str] = &["for_chunks", "for_agenda", "apply_by_range"];
/// A function declared a sweep, and the arguments naming its cycle or its reason; a function of the opening.
const SWEEP: &str = "sweep";
const CYCLE: &str = "cycle";
const REASON: &str = "reason";
const OPENING: &str = "opening";

fn in_scope(c: &Crate) -> bool {
    c.name == "phx-world" || c.name.starts_with("sys-")
}

/// Every whole-table walk PC-96 finds, and a breach for each source that does not parse or sweeps without a cycle or reason.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates().filter(|c| in_scope(c)) {
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
                message: format!("`{what}` walks a whole table outside the kernel's traversals and declared sweeps"),
                item,
                what,
                line,
            }));
            breaches.extend(finder.uncycled.into_iter().map(|(line, name)| {
                Breach::new(RULE, &source.path, line, format!("`{name}` declared a sweep without its cycle or reason"))
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
    uncycled: Vec<(usize, String)>,
    items: Vec<String>,
}

/// A function's standing: a sweep with its cycle or reason, or a function of the opening, is admitted; a sweep with
/// neither is refused, and admitted as a sweep so its walks are not reported twice.
fn admitted_fn(attrs: &[Attribute]) -> (bool, bool) {
    let sweep = attrs.iter().find(|a| a.path().is_ident(SWEEP));
    let cycled = sweep.is_some_and(|a| match &a.meta {
        syn::Meta::List(list) => attrs::has_ident(&list.tokens, CYCLE) || attrs::has_ident(&list.tokens, REASON),
        _ => false,
    });
    let opening = attrs.iter().any(|a| a.path().is_ident(OPENING));
    (sweep.is_some() || opening, sweep.is_some() && !cycled)
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Lit(l) if matches!(&l.lit, Lit::Int(i) if i.base10_digits() == "0"))
}

impl Finder {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
    }

    fn function(&mut self, name: String, attrs_: &[Attribute], line: usize, visit: impl FnOnce(&mut Finder)) {
        if attrs::is_test(attrs_) {
            return;
        }
        let (admitted, uncycled) = admitted_fn(attrs_);
        if uncycled {
            self.uncycled.push((line, name.clone()));
        }
        if admitted {
            return;
        }
        self.items.push(name);
        visit(self);
        self.items.pop();
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.items.push(format!("mod {}", item.ident));
            visit::visit_item_mod(self, item);
            self.items.pop();
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
        self.items.push(name);
        visit::visit_item_impl(self, item);
        self.items.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let line = attrs::line(item.sig.ident.span());
        self.function(item.sig.ident.to_string(), &item.attrs, line, |f| visit::visit_item_fn(f, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let line = attrs::line(item.sig.ident.span());
        self.function(item.sig.ident.to_string(), &item.attrs, line, |f| visit::visit_impl_item_fn(f, item));
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        let name = match &*call.func {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        if let Some(name) = &name {
            if TRAVERSALS.contains(&name.as_str()) {
                return;
            }
            if WALKS.contains(&name.as_str()) {
                let line = attrs::line(call.paren_token.span.open());
                self.found.push((line, self.item(), name.clone()));
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = call.method.to_string();
        if TRAVERSALS.contains(&name.as_str()) {
            visit::visit_expr(self, &call.receiver);
            return;
        }
        if WALKS.contains(&name.as_str()) || (BARE_WALKS.contains(&name.as_str()) && call.args.is_empty()) {
            self.found.push((attrs::line(call.method.span()), self.item(), name));
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_range(&mut self, range: &'ast ExprRange) {
        let whole_end = match (&range.start, &range.end, &range.limits) {
            (Some(start), Some(end), RangeLimits::HalfOpen(_)) if is_zero(start) => match &**end {
                Expr::MethodCall(m) if m.args.is_empty() && WHOLE.contains(&m.method.to_string().as_str()) => {
                    Some(m.method.to_string())
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(end) = whole_end {
            let line = match &range.end {
                Some(e) => match &**e {
                    Expr::MethodCall(m) => attrs::line(m.method.span()),
                    _ => 1,
                },
                None => 1,
            };
            self.found.push((line, self.item(), format!("0..{end}()")));
        }
        visit::visit_expr_range(self, range);
    }
}

#[cfg(test)]
mod tests {
    use super::{Finder, run};
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};
    use syn::visit::Visit;

    fn finds(text: &str) -> Finder {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut f = Finder::default();
        f.visit_file(&file);
        f
    }

    fn whats(f: &Finder) -> Vec<&str> {
        f.found.iter().map(|(_, _, w)| w.as_str()).collect()
    }

    #[test]
    fn whole_table_walks_are_refused() {
        let f = finds(
            "impl Core { fn day(&self) {\n\
             for s in self.kinds[0].live_slots() {}\n\
             let w = live_every(words, 0, 2);\n\
             let _ = self.firm_slots(f);\n\
             let t = stocks.totals();\n\
             let a = store.all();\n\
             let ok = v.iter().all(|x| *x > 0);\n\
             for i in 0..self.rows.len() {}\n\
             let r: Vec<_> = (0..n.count()).collect();\n\
             for i in 1..v.len() {}\n\
             } }",
        );
        assert_eq!(whats(&f), ["live_slots", "live_every", "firm_slots", "totals", "all", "0..len()", "0..count()"]);
        assert!(f.found.iter().all(|(_, item, _)| item == "Core::day"));
    }

    #[test]
    fn walks_inside_traversals_are_admitted() {
        let f = finds(
            "fn payday(p: &Pool) { for_chunks(p, site, n, |c| store.live_slots().count()); \
                       phx_exec::traverse::for_agenda(p, a, |x| 0..x.len()); }",
        );
        assert!(f.found.is_empty(), "{:?}", f.found);
    }

    #[test]
    fn declared_sweeps_are_admitted() {
        let f = finds(
            "#[sweep(store = accounts, cycle = 30)] fn audit_accounts(s: &S) { for x in s.live_slots() {} }\n\
                       #[sweep(store = accounts, reason = \"a bank failing\")] fn depositors(s: &S) { for x in s.live_slots() {} }\n\
                       #[opening] fn open(s: &S) { for x in s.live_slots() {} }",
        );
        assert!(f.found.is_empty() && f.uncycled.is_empty());
    }

    #[test]
    fn sweep_without_cycle_is_refused() {
        let text = "#[sweep(store = accounts)] fn audit(s: &S) { for x in s.live_slots() {} }";
        let c = with_source(krate("phx-world", Layer::Assembly), "src/core_audit.rs", text);
        let breaches = run(&Workspace::new(vec![c]));
        assert_eq!(breaches.len(), 1);
        assert!(breaches[0].message.contains("without its cycle"));
    }
}
