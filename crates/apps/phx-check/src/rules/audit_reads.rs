use syn::visit::{self, Visit};
use syn::{FnArg, ImplItem, Type, Visibility};

use super::{Breach, attrs, unparsed};
use crate::workspace::{DepKind, Source, Workspace};

const RULE: &str = "PC-22";
const CORE: &str = "phx-core";
const CONTEXT_FILE: &str = "/src/family.rs";
const CONTEXT: &str = "FamilyCtx";
const INPUTS: &str = "AuditInputs";
const AUDIT: &str = "phx-audit";
const STORAGE: &str = "phx_store";

/// The world's stores, which the audit may hold only by shared reference.
const STORES: &[&str] = &[
    "Directory",
    "RecordStore",
    "EventStore",
    "MessageStore",
    "DayMessages",
    "KindTable",
    "Register",
    "Calendar",
    "Agenda",
    "KernelMap",
    "PlayerQueue",
    "Bindings",
    "Column",
    "Table",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    if let Some(core) = ws.crates.iter().find(|c| c.name == CORE)
        && let Some(source) = core.sources.iter().find(|s| s.path.ends_with(CONTEXT_FILE))
    {
        breaches.extend(context_breaches(source));
    }
    if let Some(audit) = ws.crates.iter().find(|c| c.name == AUDIT) {
        for dep in audit.deps.iter().filter(|d| d.name == "phx-store" && d.kind == DepKind::Normal) {
            let line = audit.dependency_line(&dep.name);
            breaches.push(Breach::new(RULE, &audit.manifest_path(), line, "the audit depends on the storage crate"));
        }
        for source in audit.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(audit_breaches(source));
        }
    }
    breaches
}

fn named(ty: &Type, names: &[&str]) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| names.iter().any(|n| s.ident == n)))
}

fn mutable(ty: &Type) -> bool {
    matches!(ty, Type::Reference(r) if r.mutability.is_some())
}

/// The family context gives only shared reads: its inputs hold nothing mutably, and each of its public methods
/// reads through `&self`, bar its one constructor.
fn context_breaches(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    let mut found = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Struct(s) if s.ident == INPUTS || s.ident == CONTEXT => {
                for field in s.fields.iter().filter(|f| mutable(&f.ty)) {
                    let name = field.ident.as_ref().map(ToString::to_string).unwrap_or_default();
                    found.push((attrs::line(s.ident.span()), format!("`{}::{name}` is held mutably", s.ident)));
                }
                if s.ident == CONTEXT && s.fields.iter().any(|f| !matches!(f.vis, Visibility::Inherited)) {
                    found.push((attrs::line(s.ident.span()), "the family context has a public field".to_owned()));
                }
            }
            syn::Item::Impl(i) if named(&i.self_ty, &[CONTEXT]) && i.trait_.is_none() => {
                for f in i.items.iter().filter_map(|it| if let ImplItem::Fn(f) = it { Some(f) } else { None }) {
                    if !matches!(f.vis, Visibility::Public(_)) {
                        continue;
                    }
                    let shared = f.sig.inputs.iter().all(|input| match input {
                        FnArg::Receiver(r) => matches!(r.kind, syn::ReceiverKind::Reference(_, _, None)),
                        FnArg::Typed(t) => !mutable(&t.ty),
                    });
                    let constructor = f.sig.receiver().is_none() && f.sig.ident == "new";
                    if !shared || (f.sig.receiver().is_none() && !constructor) {
                        let line = attrs::line(f.sig.ident.span());
                        found.push((line, format!("`{CONTEXT}::{}` is not a read through `&self`", f.sig.ident)));
                    }
                }
            }
            _ => {}
        }
    }
    found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// The audit reaches no store mutably and nothing of the storage crate.
fn audit_breaches(source: &Source) -> Vec<Breach> {
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

struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_type_reference(&mut self, r: &'ast syn::TypeReference) {
        if r.mutability.is_some() && named(&r.elem, STORES) {
            let line = attrs::line(r.and_token.span);
            self.found.push((line, "the audit holds a world store mutably".to_owned()));
        }
        visit::visit_type_reference(self, r);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(first) = path.segments.first()
            && first.ident == STORAGE
        {
            self.found.push((attrs::line(first.ident.span()), "the audit reaches into the storage crate".to_owned()));
        }
        visit::visit_path(self, path);
    }

    fn visit_use_tree(&mut self, tree: &'ast syn::UseTree) {
        if let syn::UseTree::Path(p) = tree
            && p.ident == STORAGE
        {
            self.found.push((attrs::line(p.ident.span()), "the audit reaches into the storage crate".to_owned()));
        }
        visit::visit_use_tree(self, tree);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_dep, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn family_context_and_audit_read_only() {
        let context = "pub struct AuditInputs<'a> { pub directory: &'a mut Directory }\n\
                       pub struct FamilyCtx<'a> { pub inputs: AuditInputs<'a> }\n\
                       impl<'a> FamilyCtx<'a> {\n\
                       pub fn new(inputs: AuditInputs<'a>) -> FamilyCtx<'a> { FamilyCtx { inputs } }\n\
                       pub fn day(&self) -> Day { d }\n\
                       pub fn fix(&mut self) {}\n\
                       pub fn other(x: &mut u8) {}\n}";
        let audit = "use phx_store::Column;\n\
                     fn close(d: &mut Directory, r: &RecordStore) {}\n\
                     #[cfg(test)]\nmod tests { fn t(d: &mut Directory) {} }";
        let core = with_source(krate("phx-core", Layer::Kernel), "src/family.rs", context);
        let aud = with_dep(with_source(krate("phx-audit", Layer::Kernel), "src/runner.rs", audit), "phx-store", true);
        let found: Vec<(usize, String)> =
            run(&Workspace::new(vec![core, aud])).into_iter().map(|b| (b.line, b.message)).collect();
        let messages: Vec<&str> = found.iter().map(|(_, m)| m.as_str()).collect();
        assert_eq!(
            messages,
            [
                "`AuditInputs::directory` is held mutably",
                "the family context has a public field",
                "`FamilyCtx::fix` is not a read through `&self`",
                "`FamilyCtx::other` is not a read through `&self`",
                "the audit depends on the storage crate",
                "the audit reaches into the storage crate",
                "the audit holds a world store mutably",
            ]
        );
    }
}
