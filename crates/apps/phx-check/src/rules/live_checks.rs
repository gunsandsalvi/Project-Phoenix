use regex::Regex;
use syn::visit::{self, Visit};
use syn::{Expr, FnArg, ImplItem, Item, ItemConst, Type, Visibility};

use super::{Breach, attrs, unparsed};
use crate::docs;
use crate::workspace::{Crate, Source, Workspace};

const RULE: &str = "PC-20";
const SUITE: &str = "phx-cli";
const SUITE_FILE: &str = "/src/checks/mod.rs";
const WORLD: &str = "phx-world";
const INSPECTOR: &str = "Inspector";

/// A check defined by `live_check!`: the constant that holds it, its identity and where it is.
struct Defined {
    constant: String,
    id: String,
    file: String,
    line: usize,
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    if let Some(suite) = ws.crates.iter().find(|c| c.name == SUITE) {
        breaches.extend(suite_breaches(ws, suite));
    }
    if let Some(world) = ws.crates.iter().find(|c| c.name == WORLD) {
        for source in world.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(inspector_breaches(source));
        }
    }
    breaches
}

fn suite_breaches(ws: &Workspace, suite: &Crate) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let mut defined = Vec::new();
    let mut listed: Option<(String, Vec<String>)> = None;
    for source in suite.sources.iter().filter(|s| !s.is_test_or_bench()) {
        let file = match &source.file {
            Ok(file) => file,
            Err(error) => {
                breaches.push(unparsed(RULE, &source.path, error));
                continue;
            }
        };
        for item in &file.items {
            let Item::Const(c) = item else { continue };
            if let Some(id) = live_check_id(c) {
                let line = attrs::line(c.ident.span());
                defined.push(Defined { constant: c.ident.to_string(), id, file: source.path.clone(), line });
            } else if c.ident == "CHECKS" && source.path.ends_with(SUITE_FILE) {
                listed = Some((source.path.clone(), listed_names(&c.expr)));
            }
        }
    }
    let Some((suite_file, names)) = listed else {
        let at = format!("{}{SUITE_FILE}", suite.dir);
        return vec![Breach::new(RULE, &at, 1, "the suite has no `CHECKS` list")];
    };
    for d in &defined {
        if d.constant != d.id.replace('-', "_") {
            breaches.push(Breach::new(RULE, &d.file, d.line, format!("`{}` holds check `{}`", d.constant, d.id)));
        }
        if !names.contains(&d.constant) {
            breaches.push(Breach::new(RULE, &d.file, d.line, format!("check `{}` is not in `CHECKS`", d.id)));
        }
        if defined.iter().filter(|o| o.id == d.id).count() > 1 {
            breaches.push(Breach::new(RULE, &d.file, d.line, format!("check `{}` is defined twice", d.id)));
        }
    }
    for name in names.iter().filter(|n| !defined.iter().any(|d| d.constant == **n)) {
        breaches.push(Breach::new(
            RULE,
            &suite_file,
            1,
            format!("`CHECKS` lists `{name}`, which no `live_check!` defines"),
        ));
    }
    breaches.extend(registered_breaches(ws, &defined, &suite_file));
    breaches
}

/// The identity a constant's `live_check!` declares, when it is one.
fn live_check_id(item: &ItemConst) -> Option<String> {
    use proc_macro2::TokenTree as T;
    let Expr::Macro(call) = item.expr.as_ref() else { return None };
    if call.mac.path.segments.last().is_none_or(|s| s.ident != "live_check") {
        return None;
    }
    let tokens: Vec<T> = call.mac.tokens.clone().into_iter().collect();
    tokens.windows(3).find_map(|w| match w {
        [T::Ident(key), T::Punct(colon), T::Literal(id)] if key == "id" && colon.as_char() == ':' => {
            Some(id.to_string().trim_matches('"').to_owned())
        }
        _ => None,
    })
}

fn listed_names(expr: &Expr) -> Vec<String> {
    let Expr::Reference(r) = expr else { return Vec::new() };
    let Expr::Array(a) = r.expr.as_ref() else { return Vec::new() };
    a.elems
        .iter()
        .filter_map(|e| match e {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        })
        .collect()
}

/// Every check a finished step names stays defined, run or retired with its reason.
fn registered_breaches(ws: &Workspace, defined: &[Defined], suite_file: &str) -> Vec<Breach> {
    let Ok(steps) = docs::steps(&ws.plan) else { return Vec::new() };
    let Ok(id) = Regex::new(r"LC-\d+-\d{2}") else { return Vec::new() };
    let lines: Vec<&str> = ws.plan.lines().collect();
    let mut breaches = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        if step.status.as_deref() != Some("done") {
            continue;
        }
        let end = steps.get(index + 1).map_or(lines.len(), |next| next.line - 1);
        let text = lines.get(step.line..end).map(|l| l.join("\n")).unwrap_or_default();
        for found in id.find_iter(&text) {
            if !defined.iter().any(|d| d.id == found.as_str()) {
                let message = format!("{} names check `{}`, which the suite no longer holds", step.id, found.as_str());
                breaches.push(Breach::new(RULE, suite_file, 1, message));
            }
        }
    }
    breaches.dedup();
    breaches
}

/// The inspector gives observers only shared reads: no public field, and every public method on `&self`.
fn inspector_breaches(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    let mut finder = Finder { found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder {
    found: Vec<(usize, String)>,
}

fn is_inspector(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == INSPECTOR))
}

fn mutable(ty: &Type) -> bool {
    matches!(ty, Type::Reference(r) if r.mutability.is_some())
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if item.ident == INSPECTOR && item.fields.iter().any(|f| !matches!(f.vis, Visibility::Inherited)) {
            self.found.push((attrs::line(item.ident.span()), "the inspector has a public field".to_owned()));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if attrs::is_test(&item.attrs) || !is_inspector(&item.self_ty) {
            return;
        }
        for impl_item in &item.items {
            let ImplItem::Fn(f) = impl_item else { continue };
            if !matches!(f.vis, Visibility::Public(_)) {
                continue;
            }
            let line = attrs::line(f.sig.ident.span());
            let name = &f.sig.ident;
            for input in &f.sig.inputs {
                let bad = match input {
                    FnArg::Receiver(r) => !matches!(r.kind, syn::ReceiverKind::Reference(_, _, None)),
                    FnArg::Typed(t) => mutable(&t.ty),
                };
                if bad {
                    self.found.push((line, format!("`Inspector::{name}` is not a read through `&self`")));
                }
            }
            if f.sig.receiver().is_none() && !is_constructor(&f.sig) {
                self.found.push((line, format!("`Inspector::{name}` is not a read through `&self`")));
            }
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

/// The one function without a receiver: it takes the world by shared reference and returns the inspector.
fn is_constructor(sig: &syn::Signature) -> bool {
    let returns_self = matches!(&sig.output, syn::ReturnType::Type(_, ty) if is_inspector(ty));
    returns_self
        && sig.inputs.iter().all(|i| matches!(i, FnArg::Typed(t) if matches!(t.ty.as_ref(), Type::Reference(_))))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const STAGE: &str = "pub const LC_0_01: Check = live_check! { id: \"LC-0-01\", title: \"t\", from_step: \"S0.11\", check: f };\n\
                         pub const LC_0_02: Check = live_check! { id: \"LC-0-03\", title: \"t\", from_step: \"S0.11\", retired: \"why\" };\n\
                         pub const LC_0_04: Check = live_check! { id: \"LC-0-04\", title: \"t\", from_step: \"S0.11\", check: f };\n";

    fn breaches(suite: &str, plan: &str, inspector: &str) -> Vec<String> {
        let cli = with_source(
            with_source(krate("phx-cli", Layer::Apps), "src/checks/stage0.rs", STAGE),
            "src/checks/mod.rs",
            suite,
        );
        let world = with_source(krate("phx-world", Layer::Assembly), "src/inspector.rs", inspector);
        let mut ws = Workspace::new(vec![cli, world]);
        plan.clone_into(&mut ws.plan);
        run(&ws).iter().map(|b| b.message.clone()).collect()
    }

    #[test]
    fn checks_are_listed_kept_and_read_only() {
        let suite = "pub const CHECKS: &[Check] = &[stage0::LC_0_01, stage0::LC_0_02, stage0::LC_0_09];";
        let plan = "### S0.11 — a\n\n**Status**: done\n\nLC-0-01 and LC-0-05.\n\n### S0.12 — b\n\n**Status**: planned\n\nLC-0-06.\n";
        let inspector = "pub struct Inspector<'a> { pub world: &'a World }\n\
                         impl<'a> Inspector<'a> {\n\
                         pub fn new(world: &'a World) -> Inspector<'a> { Inspector { world } }\n\
                         pub fn today(&self) -> Day { d }\n\
                         pub fn set(&mut self) {}\n\
                         pub fn take(self) {}\n\
                         pub fn other(x: u8) -> u8 { x }\n\
                         fn private(&mut self) {}\n}";
        let found = breaches(suite, plan, inspector);
        let expected = [
            "`LC_0_02` holds check `LC-0-03`",
            "check `LC-0-04` is not in `CHECKS`",
            "`CHECKS` lists `LC_0_09`, which no `live_check!` defines",
            "S0.11 names check `LC-0-05`, which the suite no longer holds",
            "the inspector has a public field",
            "`Inspector::set` is not a read through `&self`",
            "`Inspector::take` is not a read through `&self`",
            "`Inspector::other` is not a read through `&self`",
        ];
        assert_eq!(found, expected);
    }

    #[test]
    fn a_suite_without_its_list_is_refused() {
        assert_eq!(breaches("", "", ""), ["the suite has no `CHECKS` list"]);
    }
}
