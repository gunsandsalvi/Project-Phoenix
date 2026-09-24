use syn::visit::{self, Visit};
use syn::{ExprPath, LitStr};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-75";
const MARKET: &str = "phx-market";
const READER: &str = "src/reach.rs";
const ENTRY: &str = "XB.closed_borders";
const CONST: &str = "CLOSED_BORDERS";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            if c.name == MARKET && source.path.ends_with(READER) {
                continue;
            }
            breaches.extend(check(source));
        }
    }
    breaches
}

fn check(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    let mut finder = Finder { found: Vec::new() };
    finder.visit_file(file);
    finder
        .found
        .into_iter()
        .map(|line| {
            Breach::new(
                RULE,
                &source.path,
                line,
                "the closed borders read outside the markets' reach, which alone closes a border",
            )
        })
        .collect()
}

/// Every mention of the closed borders: the entry's name, or the constant that declares it.
struct Finder {
    found: Vec<usize>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_lit_str(&mut self, lit: &'ast LitStr) {
        if lit.value() == ENTRY {
            self.found.push(attrs::line(lit.span()));
        }
    }

    fn visit_expr_path(&mut self, e: &'ast ExprPath) {
        if let Some(last) = e.path.segments.last()
            && last.ident == CONST
        {
            self.found.push(attrs::line(last.ident.span()));
        }
        visit::visit_expr_path(self, e);
    }
}

#[cfg(test)]
mod tests {
    use syn::visit::Visit;

    use super::Finder;

    #[test]
    fn a_border_is_closed_in_one_place() {
        let mut f = Finder { found: Vec::new() };
        f.visit_file(&syn::parse_file(r#"fn f(d: &mut D) { let p = d.prim(&phx_market::reach::CLOSED_BORDERS); let n = "XB.closed_borders"; }"#).unwrap());
        assert_eq!(f.found.len(), 2);
    }
}
