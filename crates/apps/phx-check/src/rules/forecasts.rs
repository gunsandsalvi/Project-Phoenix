//! No forecast by running the world: the outlook methods read only what they are handed. Their crate depends on
//! nothing that holds the world, and no function of it takes the world, a table or a handler's context.

use syn::visit::{self, Visit};
use syn::{FnArg, PathSegment, Signature};

use super::{Breach, attrs, unparsed};
use crate::workspace::{DepKind, Workspace};

const RULE: &str = "PC-33";
const VAL: &str = "phx-val";
/// The crates the outlook methods may use: numbers, identities, draws and the register's declarations.
const ALLOWED: &[&str] = &["phx-core", "phx-id", "phx-macros", "phx-num", "phx-rand"];
/// The names a parameter holding the world, its stores or a handler's view would carry.
const HOLDERS: &[&str] = &["World", "Inspector", "Table", "Books", "Store", "Ctx", "Arenas"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let Some(c) = ws.crates.iter().find(|c| c.name == VAL) else { return breaches };
    for dep in c.deps.iter().filter(|d| d.internal && d.kind == DepKind::Normal) {
        if !ALLOWED.contains(&dep.name.as_str()) {
            let line = c.dependency_line(&dep.name);
            breaches.push(Breach::new(RULE, &c.manifest_path(), line, format!("`{VAL}` depends on `{}`", dep.name)));
        }
    }
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
        for (line, name) in finder.lines {
            breaches.push(Breach::new(RULE, &source.path, line, format!("a function taking `{name}`")));
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    lines: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_signature(&mut self, sig: &'ast Signature) {
        for input in &sig.inputs {
            if let FnArg::Typed(t) = input {
                let mut names = Names(Vec::new());
                names.visit_type(&t.ty);
                for name in names.0 {
                    if HOLDERS.iter().any(|h| name.ends_with(h)) {
                        self.lines.push((attrs::line(sig.fn_token.span), name));
                    }
                }
            }
        }
        visit::visit_signature(self, sig);
    }
}

/// Every type name a parameter's type mentions, generics included.
#[derive(Debug)]
struct Names(Vec<String>);

impl<'ast> Visit<'ast> for Names {
    fn visit_path_segment(&mut self, seg: &'ast PathSegment) {
        self.0.push(seg.ident.to_string());
        visit::visit_path_segment(self, seg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(src: &str) -> Vec<String> {
        let file = syn::parse_file(src).unwrap_or_else(|e| panic!("{e}"));
        let mut f = Finder { lines: Vec::new() };
        f.visit_file(&file);
        f.lines.into_iter().map(|(_, n)| n).collect()
    }

    #[test]
    fn methods_take_values_not_the_world() {
        assert!(found("pub fn adaptive(previous: f64, observed: f64, lambda: f64) -> f64 { 0.0 }").is_empty());
        assert_eq!(found("pub fn peek(w: &phx_world::World) {}"), vec!["World"]);
        assert_eq!(found("pub fn peek(t: Option<&KindTable<SystemBacking>>) {}"), vec!["KindTable"]);
        assert_eq!(found("impl X { fn f(&self, ctx: &mut Ctx<'_>) {} }"), vec!["Ctx"]);
    }
}
