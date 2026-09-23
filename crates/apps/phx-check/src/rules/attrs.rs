use proc_macro2::{Span, TokenStream, TokenTree};
use syn::visit::{self, Visit};
use syn::{Attribute, Meta};

pub fn line(span: Span) -> usize {
    span.start().line
}

/// `#[test]`, or a `cfg` whose predicate names `test` anywhere.
pub fn is_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        let path = a.path();
        path.is_ident("test") || (path.is_ident("cfg") && list_tokens(a).is_some_and(|t| has_ident(t, "test")))
    })
}

pub fn is_doc(attr: &Attribute) -> bool {
    attr.path().is_ident("doc")
}

fn list_tokens(attr: &Attribute) -> Option<&TokenStream> {
    match &attr.meta {
        Meta::List(list) => Some(&list.tokens),
        Meta::Path(_) | Meta::NameValue(_) => None,
    }
}

pub fn has_ident(tokens: &TokenStream, name: &str) -> bool {
    tokens.clone().into_iter().any(|t| match t {
        TokenTree::Ident(ident) => ident == name,
        TokenTree::Group(group) => has_ident(&group.stream(), name),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

/// The argument lists of an attribute at lint level `level` (`allow` or `expect`), written directly or inside a
/// `cfg_attr`.
pub fn lint_levels(attr: &Attribute, level: &str) -> Vec<TokenStream> {
    let Some(tokens) = list_tokens(attr) else {
        return Vec::new();
    };
    if attr.path().is_ident(level) {
        return vec![tokens.clone()];
    }
    if !attr.path().is_ident("cfg_attr") {
        return Vec::new();
    }
    let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
    trees
        .windows(2)
        .filter_map(|pair| match pair {
            [TokenTree::Ident(ident), TokenTree::Group(group)] if ident == level => Some(group.stream()),
            _ => None,
        })
        .collect()
}

/// Every attribute of a file, at any depth.
pub fn all(file: &syn::File) -> Vec<&Attribute> {
    let mut collector = Collector { found: Vec::new() };
    collector.visit_file(file);
    collector.found
}

#[derive(Debug)]
struct Collector<'ast> {
    found: Vec<&'ast Attribute>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        self.found.push(attr);
        visit::visit_attribute(self, attr);
    }
}
