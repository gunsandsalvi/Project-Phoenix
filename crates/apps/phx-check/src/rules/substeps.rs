use syn::visit::{self, Visit};
use syn::{Expr, ImplItemFn, Item, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Crate, Source, Workspace};

const RULE: &str = "PC-21";
const CORE: &str = "phx-core";
const TABLE_FILE: &str = "/src/substep.rs";
const MACROS: &str = "phx-macros";
const MACROS_FILE: &str = "/src/decl.rs";
const TABLE: &str = "SUB_STEPS";

/// The sub-steps the table holds, in order, read from the kernel's table.
fn table(core: &Crate) -> Option<(String, Vec<String>)> {
    let source = core.sources.iter().find(|s| s.path.ends_with(TABLE_FILE))?;
    let file = source.file.as_ref().ok()?;
    let entries = file.items.iter().find_map(|item| match item {
        Item::Const(c) if c.ident == TABLE => Some(array_names(&c.expr)),
        _ => None,
    })?;
    Some((source.path.clone(), entries))
}

/// Each entry's first path, or its string, in a constant array.
fn array_names(expr: &Expr) -> Vec<String> {
    let Expr::Array(a) = expr else { return Vec::new() };
    a.elems
        .iter()
        .filter_map(|e| match e {
            Expr::Call(call) => call.args.first().and_then(|arg| match arg {
                Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            }),
            Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => Some(s.value()),
            _ => None,
        })
        .collect()
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let Some(core) = ws.crates.iter().find(|c| c.name == CORE) else { return Vec::new() };
    let Some((table_file, known)) = table(core) else {
        let at = format!("{}{TABLE_FILE}", core.dir);
        return vec![Breach::new(RULE, &at, 1, "the kernel has no sub-step table")];
    };
    let mut breaches = Vec::new();
    if let Some(macros) = ws.crates.iter().find(|c| c.name == MACROS)
        && let Some(source) = macros.sources.iter().find(|s| s.path.ends_with(MACROS_FILE))
        && let Ok(file) = &source.file
    {
        let copy = file.items.iter().find_map(|item| match item {
            Item::Const(c) if c.ident == TABLE => Some(array_names(&c.expr)),
            _ => None,
        });
        if copy.as_ref() != Some(&known) {
            let message = format!("the declaration macros' sub-steps differ from the table in {table_file}");
            breaches.push(Breach::new(RULE, &source.path, 1, message));
        }
    }
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source, &known));
        }
    }
    breaches
}

fn check(source: &Source, known: &[String]) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { known, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder<'a> {
    known: &'a [String],
    found: Vec<(usize, String)>,
}

impl Finder<'_> {
    fn named(&mut self, name: &str, line: usize) {
        if !self.known.iter().any(|k| k == name) {
            self.found.push((line, format!("sub-step `{name}` is not in the table")));
        }
    }
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<&syn::PathSegment> = path.segments.iter().collect();
        for pair in segments.windows(2) {
            if let [enum_name, variant] = pair
                && enum_name.ident == "SubStep"
            {
                self.named(&variant.ident.to_string(), attrs::line(variant.ident.span()));
            }
        }
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let tokens: Vec<proc_macro2::TokenTree> = mac.tokens.clone().into_iter().collect();
        collect(&tokens, self);
        visit::visit_macro(self, mac);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_impl_item_fn(self, item);
        }
    }
}

/// A declaration macro's `substep: X`, at any depth of its groups.
fn collect(tokens: &[proc_macro2::TokenTree], finder: &mut Finder<'_>) {
    use proc_macro2::TokenTree as T;
    for w in tokens.windows(3) {
        if let [T::Ident(key), T::Punct(p), T::Ident(value)] = w
            && key == "substep"
            && p.as_char() == ':'
        {
            finder.named(&value.to_string(), attrs::line(value.span()));
        }
    }
    for t in tokens {
        if let T::Group(g) = t {
            let inner: Vec<T> = g.stream().into_iter().collect();
            collect(&inner, finder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const TABLE: &str = "pub const SUB_STEPS: [SubStepInfo; 2] = [info(S::S1a, \"1a\", false, Index), info(S::S1b, \"1b\", false, Index)];";

    fn lines(macros: &str, system: &str) -> Vec<(usize, String)> {
        let core = with_source(krate("phx-core", Layer::Kernel), "src/substep.rs", TABLE);
        let mac = with_source(krate("phx-macros", Layer::Foundation), "src/decl.rs", macros);
        let sys = with_source(krate("sys-dem", Layer::Systems), "src/lib.rs", system);
        run(&Workspace::new(vec![core, mac, sys])).into_iter().map(|b| (b.line, b.message)).collect()
    }

    #[test]
    fn handlers_name_only_the_table_s_sub_steps() {
        let same = "const SUB_STEPS: [&str; 2] = [\"S1a\", \"S1b\"];";
        let system = "declare_handler! { pub A = \"DEM.a\" { substep: S1b, table: \"h\" } }\n\
                      declare_handler! { pub B = \"DEM.b\" { substep: S9z, table: \"h\" } }\n\
                      fn f() -> SubStep { SubStep::S7q }\n\
                      #[cfg(test)]\nmod tests { fn t() -> SubStep { SubStep::S0x } }";
        let not_in = |s: &str| format!("sub-step `{s}` is not in the table");
        assert_eq!(lines(same, system), vec![(2, not_in("S9z")), (3, not_in("S7q"))]);
        let differs = "const SUB_STEPS: [&str; 1] = [\"S1a\"];";
        assert_eq!(lines(differs, "").len(), 1);
    }
}
