//! PC-97: the world never switches its parallelism off and never dispatches around the kernel. A kernel function that
//! takes the pool is never handed a literal `None` by the world or a system; nothing outside `phx-exec` maps or runs
//! work on a pool but through the kernel's traversals; and a system's rule decides without holding the world's state
//! mutably, writing only into the kernel's output buffers.

use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{
    Expr, ExprCall, ExprMethodCall, FnArg, GenericArgument, ImplItemFn, ItemFn, ItemImpl, ItemMod, PathArguments,
    Signature, Token, Type,
};

use super::hot_paths::is_hot;
use super::layering::KERNEL;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::{Crate, Workspace};

pub const RULE: &str = "PC-97";

/// The crate that implements the pool and the traversals.
const EXEC: &str = "phx-exec";
/// Calls that run work on a pool directly.
const DISPATCH: &[&str] = &["map", "for_each", "each"];
/// The kernel's output buffers, the only state a rule may write.
const OUTPUTS: &[&str] = &["DayBuf", "DayBufs", "IntentBuf", "OptionSet"];

/// Whether a type is `Option<&Pool>`, the parameter through which a kernel is handed the pool or none.
fn is_optional_pool(ty: &Type) -> bool {
    let Type::Path(p) = ty else { return false };
    let Some(seg) = p.path.segments.last() else { return false };
    if seg.ident != "Option" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else { return false };
    args.args.iter().any(|a| match a {
        GenericArgument::Type(Type::Reference(r)) => match &*r.elem {
            Type::Path(inner) => inner.path.segments.last().is_some_and(|s| s.ident == "Pool"),
            _ => false,
        },
        _ => false,
    })
}

/// A signature's pool parameter: its place among the arguments a caller passes.
fn pool_position(sig: &Signature) -> Option<usize> {
    sig.inputs.iter().filter(|a| matches!(a, FnArg::Typed(_))).position(|a| match a {
        FnArg::Typed(t) => is_optional_pool(&t.ty),
        FnArg::Receiver(_) => false,
    })
}

/// Every public kernel function and method taking the pool, by name, with the place of its pool parameter.
fn pool_takers(ws: &Workspace) -> Vec<(String, usize)> {
    #[derive(Default)]
    struct Takers(Vec<(String, usize)>);
    impl<'ast> Visit<'ast> for Takers {
        fn visit_item_fn(&mut self, f: &'ast ItemFn) {
            if matches!(f.vis, syn::Visibility::Public(_))
                && let Some(p) = pool_position(&f.sig)
            {
                self.0.push((f.sig.ident.to_string(), p));
            }
            visit::visit_item_fn(self, f);
        }
        fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
            if matches!(f.vis, syn::Visibility::Public(_))
                && let Some(p) = pool_position(&f.sig)
            {
                self.0.push((f.sig.ident.to_string(), p));
            }
            visit::visit_impl_item_fn(self, f);
        }
    }
    let mut takers = Takers::default();
    for c in ws.world_crates().filter(|c| KERNEL.contains(&c.name.as_str())) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            if let Ok(file) = &source.file {
                takers.visit_file(file);
            }
        }
    }
    takers.0.sort();
    takers.0.dedup();
    takers.0
}

fn world_or_system(c: &Crate) -> bool {
    c.name == "phx-world" || c.name.starts_with("sys-")
}

/// Every site PC-97 finds, and a breach for each source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let takers = pool_takers(ws);
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates().filter(|c| c.name != EXEC) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
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
            let mut finder = Finder {
                takers: if world_or_system(c) { &takers } else { &[] },
                rules: c.name.starts_with("sys-") && is_hot(c, source) && source.path.contains("/src/rules/"),
                found: Vec::new(),
                items: Vec::new(),
            };
            finder.visit_file(file);
            sites.extend(finder.found.into_iter().map(|(line, item, what, message)| Found {
                path: source.path.clone(),
                item,
                what,
                line,
                message,
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

struct Finder<'a> {
    takers: &'a [(String, usize)],
    rules: bool,
    found: Vec<(usize, String, String, String)>,
    items: Vec<String>,
}

fn is_none(e: &Expr) -> bool {
    matches!(e, Expr::Path(p) if p.path.is_ident("None"))
}

/// Whether an expression names a pool: a path or field whose last name ends in `pool`.
fn names_pool(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.path.segments.last().is_some_and(|s| s.ident.to_string().to_lowercase().ends_with("pool")),
        Expr::Field(f) => match &f.member {
            syn::Member::Named(n) => n.to_string().to_lowercase().ends_with("pool"),
            syn::Member::Unnamed(_) => false,
        },
        Expr::Reference(r) => names_pool(&r.expr),
        Expr::MethodCall(m) => {
            m.method == "as_ref" && names_pool(&m.receiver) || names_pool(&m.receiver) && m.args.is_empty()
        }
        _ => false,
    }
}

impl Finder<'_> {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
    }

    fn push(&mut self, line: usize, what: &str, message: String) {
        self.found.push((line, self.item(), what.to_owned(), message));
    }

    fn none_for_pool(&mut self, name: &str, args: &Punctuated<Expr, Token![,]>, line: usize) {
        let refused = self.takers.iter().any(|(n, at)| n == name && args.iter().nth(*at).is_some_and(is_none));
        if refused {
            self.push(
                line,
                "None pool",
                format!("`{name}` handed no pool: the world never switches its parallelism off"),
            );
        }
    }

    fn signature(&mut self, sig: &Signature) {
        if !self.rules {
            return;
        }
        for arg in &sig.inputs {
            let FnArg::Typed(t) = arg else { continue };
            let Type::Reference(r) = &*t.ty else { continue };
            if r.mutability.is_none() {
                continue;
            }
            let name = match &*r.elem {
                Type::Path(p) => p.path.segments.last().map_or_else(String::new, |s| s.ident.to_string()),
                _ => "a slice".to_owned(),
            };
            if !OUTPUTS.contains(&name.as_str()) {
                let line = attrs::line(sig.ident.span());
                self.push(line, "&mut state", format!("a rule holding `&mut {name}`: rules decide, applies write"));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Finder<'_> {
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
            Type::Path(p) => p.path.segments.last().map_or_else(String::new, |s| s.ident.to_string()),
            _ => "impl".to_owned(),
        };
        self.items.push(name);
        visit::visit_item_impl(self, item);
        self.items.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        self.items.push(item.sig.ident.to_string());
        self.signature(&item.sig);
        visit::visit_item_fn(self, item);
        self.items.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        self.items.push(item.sig.ident.to_string());
        self.signature(&item.sig);
        visit::visit_impl_item_fn(self, item);
        self.items.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(p) = &*call.func {
            let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let line = attrs::line(call.paren_token.span.open());
            if let Some(name) = segs.last() {
                self.none_for_pool(name, &call.args, line);
                let on_pool = segs.len() >= 2 && segs.get(segs.len() - 2).is_some_and(|s| s == "Pool" || s == "pool");
                if on_pool && DISPATCH.contains(&name.as_str()) {
                    self.push(
                        line,
                        "pool dispatch",
                        format!("`{}` dispatches around the kernel's traversals", segs.join("::")),
                    );
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = call.method.to_string();
        let line = attrs::line(call.method.span());
        self.none_for_pool(&name, &call.args, line);
        if DISPATCH.contains(&name.as_str()) && names_pool(&call.receiver) {
            self.push(line, "pool dispatch", format!("`.{name}` on a pool dispatches around the kernel's traversals"));
        }
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const RATCHET: &str = "";

    fn breaches(kernel: &str, world: &str, rules: &str) -> Vec<String> {
        let core = with_source(krate("phx-core", Layer::Kernel), "src/settle.rs", kernel);
        let w = with_source(krate("phx-world", Layer::Assembly), "src/core_day.rs", world);
        let s = with_source(krate("sys-frm", Layer::Systems), "src/rules/price.rs", rules);
        let mut ws = Workspace::new(vec![core, w, s]);
        ws.ratchets = RATCHET.to_owned();
        run(&ws).into_iter().map(|b| b.message).collect()
    }

    const KERNEL_FN: &str = "pub fn settle(pool: Option<&Pool>, g: &G) {}\n\
                             impl W { pub fn take(&mut self, day: Day, out: &mut V, pool: Option<&phx_exec::Pool>) {} }";

    #[test]
    fn literal_none_for_a_pool_is_refused() {
        let world = "fn day(c: &mut C) { settle(None, &g); c.wheel.take(d, &mut v, None); settle(c.pool.as_ref(), &g); \
                     other(None); }";
        let found = breaches(KERNEL_FN, world, "");
        assert_eq!(found.iter().filter(|m| m.contains("handed no pool")).count(), 2, "{found:?}");
    }

    #[test]
    fn pool_map_in_world_code_is_refused() {
        let world = "fn day(c: &C) { let v = c.pool.map(3, |i| i); Pool::for_each(&p, xs, f); \
                     phx_exec::pool::each(c.pool.as_ref(), xs, f); let w: Vec<_> = xs.iter().map(|x| x).collect(); }";
        let found = breaches("", world, "");
        assert_eq!(found.iter().filter(|m| m.contains("dispatches around")).count(), 3, "{found:?}");
    }

    #[test]
    fn rules_take_no_mut_world_state() {
        let rules = "pub fn price(core: &mut Core, v: &Values) {}\npub fn post(out: &mut DayBuf<u32>, v: &Values) {}";
        let found = breaches("", "", rules);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("&mut Core"));
    }

    #[test]
    fn tests_may_pass_no_pool() {
        let world = "#[cfg(test)] mod tests { fn t() { settle(None, &g); } }";
        assert!(breaches(KERNEL_FN, world, "").is_empty());
    }
}
