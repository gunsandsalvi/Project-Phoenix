//! PC-99: a day allocates nothing. On the day's paths every buffer is a day buffer owned by a base, sized by the
//! heaviest day and kept, so the syntax of allocation is refused there; what the syntax cannot see the bench's
//! allocation counter measures. An error path marked `#[cold]` and the opening are not read, nor the arguments of the
//! run-stopping macros, whose tokens are no calls.

use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn, ItemImpl, ItemMod, Macro};

use super::hot_paths::is_hot;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::Workspace;

pub const RULE: &str = "PC-99";

/// Constructors that allocate, by type and function.
const CONSTRUCTORS: &[(&str, &str)] =
    &[("Vec", "new"), ("Vec", "with_capacity"), ("Box", "new"), ("String", "new"), ("String", "from")];
/// Methods that allocate when called with no argument.
const METHODS: &[&str] = &["collect", "to_vec", "to_owned", "to_string", "clone"];
/// Macros that allocate.
const MACROS: &[&str] = &["vec", "format"];
/// Functions not read: an error path, and the opening.
const COLD_FN: &str = "cold";
const OPENING: &str = "opening";

/// Every allocation PC-99 finds in the hot set, and a breach for each source that does not parse.
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
                message: format!("{what} allocates on the day's path: write into a kept day buffer"),
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

fn not_read(attrs_: &[Attribute]) -> bool {
    attrs::is_test(attrs_)
        || attrs_.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == COLD_FN || s.ident == OPENING))
}

impl Finder {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
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
        if !not_read(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_item_fn(f, item));
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !not_read(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_impl_item_fn(f, item));
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(p) = &*call.func {
            let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if let [.., ty, f] = segs.as_slice()
                && CONSTRUCTORS.iter().any(|(t, n)| t == ty && n == f)
            {
                self.found.push((attrs::line(call.paren_token.span.open()), self.item(), format!("`{ty}::{f}`")));
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = call.method.to_string();
        if call.args.is_empty() && METHODS.contains(&name.as_str()) {
            self.found.push((attrs::line(call.method.span()), self.item(), format!("`.{name}()`")));
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        if let Some(seg) = mac.path.segments.last()
            && MACROS.contains(&seg.ident.to_string().as_str())
        {
            self.found.push((attrs::line(mac.bang_token.span), self.item(), format!("`{}!`", seg.ident)));
        }
        visit::visit_macro(self, mac);
    }
}

#[cfg(test)]
mod tests {
    use super::Finder;
    use syn::visit::Visit;

    fn whats(text: &str) -> Vec<String> {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut f = Finder::default();
        f.visit_file(&file);
        f.found.into_iter().map(|(_, _, w)| w).collect()
    }

    #[test]
    fn allocation_forms_are_refused() {
        let found = whats(
            "fn day(v: &[u32], s: &S) { let a: Vec<u32> = Vec::new(); let b = Vec::with_capacity(4); let c = vec![0; 3]; \
             let d: Vec<_> = v.iter().collect(); let e = v.to_vec(); let f = s.name.to_owned(); let g = 3.to_string(); \
             let h = s.laws.clone(); let i = Box::new(3); let j = String::new(); let k = String::from(\"x\"); \
             let l = format!(\"{}\", 1); }",
        );
        assert_eq!(
            found,
            [
                "`Vec::new`",
                "`Vec::with_capacity`",
                "`vec!`",
                "`.collect()`",
                "`.to_vec()`",
                "`.to_owned()`",
                "`.to_string()`",
                "`.clone()`",
                "`Box::new`",
                "`String::new`",
                "`String::from`",
                "`format!`"
            ]
        );
    }

    #[test]
    fn day_buffer_pushes_are_admitted() {
        assert!(
            whats("fn day(b: &mut DayBuf<u32>, i: &mut IntentBuf<u8>) { b.push(3); i.push(1); b.extend([1, 2]); }")
                .is_empty()
        );
    }

    #[test]
    fn cold_paths_are_admitted() {
        let text = "#[cold] fn refuse(n: u32) -> String { format!(\"refused {n}\") }\n\
                    #[opening] fn open() -> Vec<u32> { Vec::new() }";
        assert!(whats(text).is_empty());
    }

    #[test]
    fn violation_arguments_are_not_read() {
        let text = "fn day(n: u32) { if n > 3 { violation!(clause = \"REP.1\", \"too many\", n = n.clone()); } \
                    capacity_exceeded!(\"rows\", 3, vec![n].len()); }";
        assert!(whats(text).is_empty());
    }
}
