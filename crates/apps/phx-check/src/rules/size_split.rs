use syn::visit::{self, Visit};
use syn::{Ident, ItemMod, LitStr};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-94";

/// The endings that name a mechanism split by size, which one kind of firm forbids.
const SPLITS: [&str; 2] = ["_small", "_large"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
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
            let mut finder = Finder { found: Vec::new() };
            finder.visit_file(file);
            for (line, name) in finder.found {
                let message = format!("`{name}` names a mechanism split by size; a firm is one kind whatever its size");
                breaches.push(Breach::new(RULE, &source.path, line, message));
            }
        }
    }
    breaches
}

fn split(name: &str) -> bool {
    let lower = name.to_lowercase();
    SPLITS.iter().any(|s| lower.ends_with(s))
}

struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        let name = ident.to_string();
        if split(&name) {
            self.found.push((attrs::line(ident.span()), name));
        }
    }

    fn visit_lit_str(&mut self, lit: &'ast LitStr) {
        let value = lit.value();
        if !value.contains(' ') && split(&value) {
            self.found.push((attrs::line(lit.span()), value));
        }
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn a_size_split_is_refused() {
        let text = "const A: &str = \"FRM.attend_small\";\n\
                    fn review_large() {}\n\
                    fn review() { let _ = \"a small firm\"; }\n\
                    #[cfg(test)]\nmod tests { fn t_small() {} }";
        let sys = with_source(krate("sys-frm", Layer::Systems), "src/lib.rs", text);
        let lines: Vec<usize> = run(&Workspace::new(vec![sys])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![1, 2]);
    }
}
