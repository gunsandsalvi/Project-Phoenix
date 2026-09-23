use syn::visit::{self, Visit};
use syn::{ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Crate, Source, Workspace};

const RULE: &str = "PC-19";

/// Where each name may be written: a crate, and within it the files, or every file when none are listed.
struct Allowed {
    name: &'static str,
    places: &'static [(&'static str, &'static [&'static str])],
    why: &'static str,
}

const ALLOWED: &[Allowed] = &[
    Allowed {
        name: "Draws::new",
        places: &[("phx-rand", &[]), ("phx-core", &["/src/streams.rs"])],
        why: "draws are made only by the run's streams",
    },
    Allowed {
        name: "Streams",
        places: &[
            ("phx-core", &["/src/streams.rs", "/src/handler.rs", "/src/contribution.rs", "/src/lib.rs"]),
            ("phx-world", &[]),
        ],
        why: "streams are opened only by the handler context, the opening's context and the observer's draws",
    },
    Allowed {
        name: "open_keyed",
        places: &[("phx-core", &["/src/streams.rs", "/src/handler.rs"])],
        why: "keyed streams are opened only by the handler context",
    },
    Allowed {
        name: "ObserverDraws",
        places: &[("phx-core", &["/src/streams.rs", "/src/lib.rs"]), ("phx-obs", &[])],
        why: "only the observer opens the observer's streams",
    },
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(c, source));
        }
    }
    breaches
}

fn allowed_here(rule: &Allowed, c: &Crate, source: &Source) -> bool {
    rule.places.iter().any(|(krate, files)| {
        c.name == *krate && (files.is_empty() || source.path.strip_prefix(&c.dir).is_some_and(|p| files.contains(&p)))
    })
}

fn check(c: &Crate, source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let barred: Vec<&Allowed> = ALLOWED.iter().filter(|a| !allowed_here(a, c, source)).collect();
    let mut finder = Finder { barred, found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder<'a> {
    barred: Vec<&'a Allowed>,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        for rule in self.barred.iter().filter(|r| r.name.contains("::")) {
            let wanted: Vec<&str> = rule.name.split("::").collect();
            let hit = names.windows(wanted.len()).any(|w| w.iter().zip(&wanted).all(|(a, b)| a == b));
            if hit && let Some(first) = path.segments.first() {
                self.found.push((attrs::line(first.ident.span()), format!("`{}`: {}", rule.name, rule.why)));
            }
        }
        visit::visit_path(self, path);
    }

    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        for rule in self.barred.iter().filter(|r| !r.name.contains("::")) {
            if ident == rule.name {
                self.found.push((attrs::line(ident.span()), format!("`{}`: {}", rule.name, rule.why)));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn draws_are_made_only_by_the_streams() {
        let text = "fn a(k: StreamKey) { let _ = phx_rand::Draws::new(k, s, 0, 0); }\n\
                    fn b(s: &Streams) {}\n\
                    fn c(o: ObserverDraws) {}\n\
                    #[cfg(test)]\nmod tests { fn t() { let _ = Draws::new(k, s, 0, 0); } }";
        let lines = |name: &str, layer, path: &str| -> Vec<usize> {
            run(&Workspace::new(vec![with_source(krate(name, layer), path, text)])).iter().map(|b| b.line).collect()
        };
        assert_eq!(lines("sys-dem", Layer::Systems, "src/lib.rs"), vec![1, 2, 3]);
        assert_eq!(lines("phx-core", Layer::Kernel, "src/streams.rs"), Vec::<usize>::new());
        assert_eq!(lines("phx-core", Layer::Kernel, "src/handler.rs"), vec![1, 3]);
        assert_eq!(lines("phx-obs", Layer::Assembly, "src/view.rs"), vec![1, 2]);
        assert_eq!(lines("phx-world", Layer::Assembly, "src/day.rs"), vec![1, 3]);
    }
}
