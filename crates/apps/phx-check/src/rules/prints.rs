use syn::visit::{self, Visit};
use syn::{ExprStruct, ItemImpl, PathArguments, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-28";
const MARKET: &str = "phx-market";
const PRINT: &str = "Print";
/// Conversions that would make a print from something else: a valuation, a quote, an order.
const CONVERSIONS: &[&str] = &["From", "TryFrom"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        let market = c.name == MARKET;
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source, market));
        }
    }
    breaches
}

fn check(source: &Source, market: bool) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { market, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// Prints built outside the markets, and conversions into a print anywhere.
struct Finder {
    market: bool,
    found: Vec<(usize, String)>,
}

fn names_print(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == PRINT))
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if !self.market && e.path.segments.last().is_some_and(|s| s.ident == PRINT) {
            let line = e.path.segments.last().map_or(1, |s| attrs::line(s.ident.span()));
            self.found
                .push((line, "a print built outside the markets, which alone make one from a match set".to_owned()));
        }
        visit::visit_expr_struct(self, e);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        if let Some((path, _)) = &item.trait_
            && let Some(last) = path.segments.last()
        {
            let into_print = last.ident == "Into"
                && matches!(&last.arguments, PathArguments::AngleBracketed(a) if a.args.iter().any(|g| matches!(g, syn::GenericArgument::Type(t) if names_print(t))));
            let from = CONVERSIONS.iter().any(|c| last.ident == c) && names_print(&item.self_ty);
            if from || into_print {
                let line = attrs::line(last.ident.span());
                self.found
                    .push((line, "a conversion into a print: a valuation, quote or order is never a print".to_owned()));
            }
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::visit::Visit;

    use super::Finder;

    fn found(code: &str, market: bool) -> usize {
        let file = syn::parse_file(code).unwrap();
        let mut f = Finder { market, found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn prints_only_from_the_markets() {
        let built = "fn f() { let p = Print { market, day }; }";
        assert_eq!(found(built, false), 1, "a print built elsewhere");
        assert_eq!(found(built, true), 0, "the markets build theirs");
        assert_eq!(found("impl From<Valuation> for Print { }", true), 1, "no valuation becomes a print");
        assert_eq!(found("impl Into<Print> for Quote { }", false), 1);
        assert_eq!(found("impl From<Print> for Mark { }", false), 0, "a print may become something else");
    }
}
