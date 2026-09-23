use syn::visit::{self, Visit};
use syn::{Attribute, ItemEnum, ItemImpl, ItemMacro, ItemStruct};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-14";

/// The crate of identities, where a default would stand for an identity no one allocated.
const ID_CRATE: &str = "phx-id";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.crates.iter().filter(|c| c.name == ID_CRATE) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut finder = Finder { lines: Vec::new() };
            finder.visit_file(file);
            breaches.extend(finder.lines.into_iter().map(|line| {
                Breach::new(RULE, &source.path, line, "an identity with a default; allocate it or leave it missing")
            }));
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    lines: Vec<usize>,
}

fn derives_default(attributes: &[Attribute]) -> bool {
    attributes.iter().filter(|a| a.path().is_ident("derive")).any(|a| {
        let mut default = false;
        let parsed = a.parse_nested_meta(|meta| {
            default |= meta.path.segments.last().is_some_and(|s| s.ident == "Default");
            Ok(())
        });
        parsed.is_ok() && default
    })
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let is_default =
            item.trait_.as_ref().and_then(|(path, _)| path.segments.last()).is_some_and(|s| s.ident == "Default");
        if is_default {
            self.lines.push(attrs::line(item.impl_token.span));
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if derives_default(&item.attrs) {
            self.lines.push(attrs::line(item.struct_token.span));
        }
        visit::visit_item_struct(self, item);
    }

    /// A macro's body is not parsed as items, so a `Default` named anywhere in it counts.
    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if attrs::has_ident(&item.mac.tokens, "Default") {
            self.lines.push(attrs::line(item.mac.bang_token.span));
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        if derives_default(&item.attrs) {
            self.lines.push(attrs::line(item.enum_token.span));
        }
        visit::visit_item_enum(self, item);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn defaults_refused_in_the_id_crate_only() {
        let text = "#[derive(Clone, Default)]\nstruct A(u32);\nimpl core::default::Default for B { }\n\
                    #[derive(Default)]\nenum C { #[default] X }\n#[derive(Clone, Copy)]\nstruct D(u32);\n\
                    macro_rules! id { () => { #[derive(Default)] struct E; } }";
        let ids = with_source(krate("phx-id", Layer::Foundation), "src/ids.rs", text);
        let lines: Vec<usize> = run(&Workspace::new(vec![ids])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![2, 3, 5, 8]);
        let other = with_source(krate("phx-num", Layer::Foundation), "src/lib.rs", text);
        assert!(run(&Workspace::new(vec![other])).is_empty());
    }
}
