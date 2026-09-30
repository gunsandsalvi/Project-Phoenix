//! The workspace's names: every item any crate declares outside its tests — types, functions, constants, modules,
//! macros, variants, a derive's name — and every source file, so a document can be held to the code it names.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};

use crate::rules::attrs;
use crate::workspace::Workspace;

/// Every item name and every source path of the workspace, and its crates' names as paths spell them.
#[derive(Debug, Default)]
pub struct Names {
    pub items: BTreeSet<String>,
    pub files: BTreeSet<String>,
    pub crates: BTreeSet<String>,
}

impl Names {
    #[must_use]
    pub fn of(ws: &Workspace) -> Names {
        let mut names = Names::default();
        for c in &ws.crates {
            names.crates.insert(c.name.replace('-', "_"));
            for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
                let Ok(file) = &source.file else { continue };
                if attrs::is_test(&file.attrs) {
                    continue;
                }
                names.files.insert(source.path.clone());
                let mut collect = Collect { items: &mut names.items };
                collect.visit_file(file);
            }
        }
        names
    }

    /// Whether a file of the workspace ends with this path.
    #[must_use]
    pub fn has_file(&self, path: &str) -> bool {
        self.files.iter().any(|f| f == path || f.ends_with(&format!("/{path}")))
    }
}

struct Collect<'a> {
    items: &'a mut BTreeSet<String>,
}

impl Collect<'_> {
    fn add(&mut self, ident: &syn::Ident) {
        self.items.insert(ident.to_string());
    }
}

impl<'ast> Visit<'ast> for Collect<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.add(&item.ident);
            visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        self.add(&item.sig.ident);
        // A procedural macro is named by its attribute: a derive by the name it declares, an attribute or function
        // macro by its function's.
        for attr in item.attrs.iter().filter(|a| a.path().is_ident("proc_macro_derive")) {
            let _ = attr.parse_nested_meta(|m| {
                if let Some(ident) = m.path.get_ident() {
                    self.items.insert(ident.to_string());
                }
                if m.input.peek(syn::token::Paren) {
                    let _ = m.parse_nested_meta(|_| Ok(()));
                }
                Ok(())
            });
        }
        visit::visit_item_fn(self, item);
    }
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.add(&item.ident);
        visit::visit_item_struct(self, item);
    }
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.add(&item.ident);
        for v in &item.variants {
            self.add(&v.ident);
        }
        visit::visit_item_enum(self, item);
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.add(&item.ident);
        visit::visit_item_trait(self, item);
    }
    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.add(&item.ident);
        visit::visit_item_type(self, item);
    }
    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.add(&item.ident);
    }
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.add(&item.ident);
        visit::visit_item_const(self, item);
    }
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.add(&item.ident);
    }
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if let Some(ident) = &item.ident {
            self.add(ident);
        }
        // A declaring macro's items, `pub NAME = …` or `Name(inner)`, are named by what they declare, past any
        // attributes before them.
        let tokens: Vec<proc_macro2::TokenTree> = item.mac.tokens.clone().into_iter().collect();
        let mut at = 0;
        while let (Some(proc_macro2::TokenTree::Punct(p)), Some(proc_macro2::TokenTree::Group(_))) =
            (tokens.get(at), tokens.get(at + 1))
        {
            if p.as_char() != '#' {
                break;
            }
            at += 2;
        }
        match tokens.get(at..).unwrap_or_default() {
            [proc_macro2::TokenTree::Ident(vis), proc_macro2::TokenTree::Ident(name), ..] if vis == "pub" => {
                self.add(name);
            }
            [proc_macro2::TokenTree::Ident(name), proc_macro2::TokenTree::Group(_), ..] => {
                self.add(name);
            }
            _ => {}
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !attrs::is_test(&item.attrs) {
            self.add(&item.sig.ident);
            visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        self.add(&item.ident);
    }
    fn visit_impl_item_type(&mut self, item: &'ast syn::ImplItemType) {
        self.add(&item.ident);
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.add(&item.sig.ident);
        visit::visit_trait_item_fn(self, item);
    }
    fn visit_trait_item_const(&mut self, item: &'ast syn::TraitItemConst) {
        self.add(&item.ident);
    }
    fn visit_field(&mut self, field: &'ast syn::Field) {
        if let Some(ident) = &field.ident {
            self.add(ident);
        }
        visit::visit_field(self, field);
    }
}
