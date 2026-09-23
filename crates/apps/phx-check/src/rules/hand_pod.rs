use syn::visit::{self, Visit};
use syn::{ItemImpl, ItemStruct, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-12";

/// The storage's own module is the one place a type becomes storable without the derive's layout checks.
const SANCTIONED: &str = "crates/kernel/phx-store/src/pod.rs";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in &ws.crates {
        for source in &c.sources {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut finder = Finder { found: Vec::new(), sanctioned: source.path == SANCTIONED };
            finder.visit_file(file);
            for (line, message) in finder.found {
                breaches.push(Breach::new(RULE, &source.path, line, message));
            }
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    found: Vec<(usize, &'static str)>,
    sanctioned: bool,
}

fn has_float(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == "f32" || s.ident == "f64"),
        Type::Array(a) => has_float(&a.elem),
        Type::Tuple(t) => t.elems.iter().any(has_float),
        Type::Group(g) => has_float(&g.elem),
        Type::Paren(p) => has_float(&p.elem),
        _ => false,
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let marker = item.trait_.as_ref().and_then(|(path, _)| path.segments.last()).map(|s| s.ident.to_string());
        if matches!(marker.as_deref(), Some("Pod" | "Sealed")) && !self.sanctioned {
            self.found.push((attrs::line(item.impl_token.span), "a hand-written storable impl; derive `Pod` instead"));
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        let derives_pod = item.attrs.iter().filter(|a| a.path().is_ident("derive")).any(|a| {
            let mut pod = false;
            let parsed = a.parse_nested_meta(|meta| {
                pod |= meta.path.segments.last().is_some_and(|s| s.ident == "Pod");
                Ok(())
            });
            parsed.is_ok() && pod
        });
        if derives_pod && item.fields.iter().any(|f| has_float(&f.ty)) {
            self.found.push((attrs::line(item.struct_token.span), "a float field in a stored type"));
        }
        visit::visit_item_struct(self, item);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn hand_written_pod_and_float_fields_refused() {
        let text = "unsafe impl Pod for X {}\nimpl phx_store::__seal::Sealed for X {}\n\
                    #[derive(Clone, Copy, phx_macros::Pod)]\n#[repr(C)]\nstruct S { a: u64, b: [f64; 2] }\n\
                    #[derive(Clone, Copy)]\nstruct Free { a: f64 }";
        let core = with_source(krate("phx-core", Layer::Kernel), "src/lib.rs", text);
        let lines: Vec<usize> = run(&Workspace::new(vec![core])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![1, 2, 5]);
        let mut store = krate("phx-store", Layer::Kernel);
        store.dir = "crates/kernel/phx-store".to_owned();
        let store = with_source(store, "src/pod.rs", "unsafe impl Pod for u64 {}\nimpl __seal::Sealed for u64 {}");
        assert!(run(&Workspace::new(vec![store])).is_empty());
    }
}
