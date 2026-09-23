use syn::visit::{self, Visit};
use syn::{ForeignItemStatic, ItemStatic, Macro};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-05";

/// The pool records the running site in one thread-local, read only by the panic hook.
const SITE: (&str, &str) = ("phx-exec", "src/site.rs");

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        let mut thread_locals = 0_usize;
        for source in &c.sources {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut finder = Finder { statics: Vec::new(), thread_locals: Vec::new() };
            finder.visit_file(file);
            for line in finder.statics {
                breaches.push(Breach::new(RULE, &source.path, line, "a `static` item in a world crate"));
            }
            let at_site = c.name == SITE.0 && source.path == format!("{}/{}", c.dir, SITE.1);
            for line in finder.thread_locals {
                thread_locals += 1;
                if !at_site || thread_locals > 1 {
                    breaches.push(Breach::new(RULE, &source.path, line, "a `thread_local!` beyond the pool's site"));
                }
            }
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    statics: Vec<usize>,
    thread_locals: Vec<usize>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_static(&mut self, item: &'ast ItemStatic) {
        self.statics.push(attrs::line(item.static_token.span));
        visit::visit_item_static(self, item);
    }

    fn visit_foreign_item_static(&mut self, item: &'ast ForeignItemStatic) {
        self.statics.push(attrs::line(item.static_token.span));
        visit::visit_foreign_item_static(self, item);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        if mac.path.segments.last().is_some_and(|s| s.ident == "thread_local") {
            self.thread_locals.push(attrs::line(mac.bang_token.span));
        }
        visit::visit_macro(self, mac);
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn statics_refused_but_the_site_thread_local() {
        let tl = "std::thread_local! { static SITE: u8 = 0; }";
        let exec = with_source(krate("phx-exec", Layer::Kernel), "src/site.rs", tl);
        let exec = with_source(exec, "src/pool.rs", tl);
        let core = with_source(krate("phx-core", Layer::Kernel), "src/lib.rs", "static X: u8 = 0;");
        let cli = with_source(krate("phx-cli", Layer::Apps), "src/main.rs", "static X: u8 = 0;");
        assert_eq!(run(&Workspace::new(vec![exec, core, cli])).len(), 2);
    }
}
