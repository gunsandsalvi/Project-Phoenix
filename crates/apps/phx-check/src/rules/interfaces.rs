use syn::visit::{self, Visit};
use syn::{Block, Expr, ImplItemFn, ItemFn, Member, Signature, Stmt, TraitItemFn};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Layer, Workspace};

const RULE: &str = "PC-08";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.crates.iter().filter(|c| c.layer == Layer::Interfaces) {
        for source in &c.sources {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut finder = Finder { found: Vec::new() };
            finder.visit_file(file);
            for (line, name) in finder.found {
                let message = format!("`{name}` has behaviour; an interface crate holds data and signatures");
                breaches.push(Breach::new(RULE, &source.path, line, message));
            }
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    found: Vec<(usize, String)>,
}

impl Finder {
    fn body(&mut self, sig: &Signature, block: &Block) {
        let name = sig.ident.to_string();
        if name == "new" || name.starts_with("from_") || is_accessor(block) {
            return;
        }
        self.found.push((attrs::line(sig.fn_token.span), name));
    }
}

/// A body that returns one of `self`'s fields, by value or by reference.
fn is_accessor(block: &Block) -> bool {
    let [Stmt::Expr(expr, None)] = block.stmts.as_slice() else {
        return false;
    };
    let field = match expr {
        Expr::Reference(r) => r.expr.as_ref(),
        other => other,
    };
    let Expr::Field(field) = field else {
        return false;
    };
    matches!(&field.member, Member::Named(_) | Member::Unnamed(_))
        && matches!(field.base.as_ref(), Expr::Path(p) if p.path.is_ident("self"))
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.body(&item.sig, &item.block);
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.body(&item.sig, &item.block);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        if let Some(block) = &item.default {
            self.body(&item.sig, block);
        }
        visit::visit_trait_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn interfaces_refuse_behaviour() {
        let text = "impl T {\n\
                    pub fn new(a: u8) -> T { T { a } }\n\
                    pub fn from_raw(a: u8) -> T { T { a } }\n\
                    pub fn a(&self) -> &u8 { &self.a }\n\
                    pub fn doubled(&self) -> u8 { self.a * 2 }\n\
                    }\n\
                    trait R { fn rule(&self) -> u8; fn default_rule(&self) -> u8 { 0 } }";
        let c = with_source(krate("if-base", Layer::Interfaces), "src/terms.rs", text);
        let lines: Vec<usize> = run(&Workspace::new(vec![c])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![5, 7]);
    }
}
