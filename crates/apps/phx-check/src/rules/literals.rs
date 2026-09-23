use proc_macro2::{TokenStream, TokenTree};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, ExprLit, ExprRepeat, GenericArgument, ImplItem, Item, ItemConst, Lit, Macro, Token, TraitItem, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-06";

/// Macros whose arguments are expressions a mechanism writes, so their literals are read as the code's; the assert,
/// format and write families share their argument forms.
const PARSED_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "vec",
    "violation",
    "format",
    "format_args",
    "write",
    "writeln",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
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
    if source.file_name() == "consts.rs" {
        return undocumented_consts(source, file);
    }
    let mut finder = Finder { lines: Vec::new() };
    finder.visit_file(file);
    finder
        .lines
        .into_iter()
        .map(|line| Breach::new(RULE, &source.path, line, "a numeric literal in a mechanism"))
        .collect()
}

/// An engineering constant says why its value is what it is.
fn undocumented_consts(source: &Source, file: &syn::File) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for item in &file.items {
        if let Item::Const(ItemConst { attrs: a, expr, const_token, .. }) = item {
            let mut finder = Finder { lines: Vec::new() };
            finder.visit_expr(expr);
            if !finder.lines.is_empty() && !a.iter().any(attrs::is_doc) {
                let line = attrs::line(const_token.span);
                breaches.push(Breach::new(RULE, &source.path, line, "a constant without a doc comment"));
            }
        }
    }
    breaches
}

/// Whether a literal's value is one of 0, 1 and 2; a minus sign is an operator on it, so −1 passes as 1 does.
fn allowed(lit: &Lit) -> bool {
    match lit {
        Lit::Int(int) => int.base10_parse::<u128>().is_ok_and(|v| v <= 2),
        Lit::Float(float) => {
            let digits = float.base10_digits();
            if digits.contains(['e', 'E']) {
                return false;
            }
            let trimmed =
                if digits.contains('.') { digits.trim_end_matches('0').trim_end_matches('.') } else { digits };
            matches!(trimmed, "" | "0" | "1" | "2")
        }
        _ => true,
    }
}

#[derive(Debug)]
struct Finder {
    lines: Vec<usize>,
}

impl Finder {
    fn tokens(&mut self, tokens: TokenStream) {
        for tree in tokens {
            match tree {
                TokenTree::Literal(literal) => {
                    let lit = Lit::new(literal.clone());
                    if !allowed(&lit) {
                        self.lines.push(attrs::line(literal.span()));
                    }
                }
                TokenTree::Group(group) => self.tokens(group.stream()),
                TokenTree::Ident(_) | TokenTree::Punct(_) => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item(&mut self, item: &'ast Item) {
        let a = match item {
            Item::Const(i) => &i.attrs,
            Item::Enum(i) => &i.attrs,
            Item::Fn(i) => &i.attrs,
            Item::Impl(i) => &i.attrs,
            Item::Macro(i) => &i.attrs,
            Item::Mod(i) => &i.attrs,
            Item::Static(i) => &i.attrs,
            Item::Struct(i) => &i.attrs,
            Item::Trait(i) => &i.attrs,
            Item::Type(i) => &i.attrs,
            Item::Union(i) => &i.attrs,
            Item::Use(i) => &i.attrs,
            _ => return visit::visit_item(self, item),
        };
        if !attrs::is_test(a) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        let skip = match item {
            ImplItem::Fn(f) => attrs::is_test(&f.attrs),
            ImplItem::Const(c) => attrs::is_test(&c.attrs),
            _ => false,
        };
        if !skip {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        let skip = match item {
            TraitItem::Fn(f) => attrs::is_test(&f.attrs),
            TraitItem::Const(c) => attrs::is_test(&c.attrs),
            _ => false,
        };
        if !skip {
            visit::visit_trait_item(self, item);
        }
    }

    /// Array lengths and const-generic arguments state a layout, not a number of the world.
    fn visit_type(&mut self, _: &'ast Type) {}

    fn visit_generic_argument(&mut self, arg: &'ast GenericArgument) {
        if !matches!(arg, GenericArgument::Const(_)) {
            visit::visit_generic_argument(self, arg);
        }
    }

    fn visit_expr_repeat(&mut self, repeat: &'ast ExprRepeat) {
        self.visit_expr(&repeat.expr);
    }

    fn visit_expr_lit(&mut self, lit: &'ast ExprLit) {
        if !allowed(&lit.lit) {
            self.lines.push(attrs::line(lit.lit.span()));
        }
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        let Some(name) = mac.path.segments.last().map(|s| s.ident.to_string()) else {
            return;
        };
        if !PARSED_MACROS.contains(&name.as_str()) {
            return;
        }
        match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(mac.tokens.clone()) {
            Ok(args) => args.iter().for_each(|arg| self.visit_expr(arg)),
            Err(_) => self.tokens(mac.tokens.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    fn count(file: &str, text: &str) -> usize {
        let c = with_source(krate("phx-core", Layer::Kernel), file, text);
        run(&Workspace::new(vec![c])).len()
    }

    #[test]
    fn literals_refuse_three_in_mechanism() {
        assert_eq!(count("src/a.rs", "fn f() -> i64 { 3 }"), 1);
        assert_eq!(count("src/a.rs", "fn f() -> f64 { 0.5 + 1.0 - 2.00 }"), 1);
        assert_eq!(count("src/a.rs", "fn f() -> i64 { -1 + 0 + 2 }"), 0);
        assert_eq!(count("src/a.rs", "struct S { a: [u32; 4] }"), 0);
        assert_eq!(count("src/a.rs", "fn f() { let _ = [0_u8; 16]; let _ = g::<8>(); }"), 0);
        assert_eq!(count("src/consts.rs", "/// A chunk is sized to the cache.\npub const X: i64 = 3;"), 0);
        assert_eq!(count("src/consts.rs", "pub const X: i64 = 3;"), 1);
        assert_eq!(count("src/a.rs", "#[cfg(test)]\nmod tests { fn t() { assert!(x == 3); } }"), 0);
        assert_eq!(count("tests/t.rs", "fn t() { let _ = 7; }"), 0);
        assert_eq!(count("src/tests/t.rs", "fn t() { let _ = 7; }"), 1);
    }

    #[test]
    fn literals_in_macro_args_refused() {
        assert_eq!(count("src/a.rs", "fn f() { violation!(clause = \"Law 7\", \"m\", k = 3); }"), 1);
        assert_eq!(count("src/a.rs", "fn f() { let _ = vec![5; n]; let _ = format!(\"{}\", 9); }"), 2);
    }
}
