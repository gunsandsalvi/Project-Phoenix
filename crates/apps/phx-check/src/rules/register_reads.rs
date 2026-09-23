use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ImplItemFn, ItemFn, ItemMod, UseTree};

use super::{Breach, attrs, unparsed};
use crate::ratchets;
use crate::workspace::{Crate, RATCHETS, Source, Workspace};

const RULE: &str = "PC-18";

/// Each token of a limit's origin, and the crates that may build one.
const TOKENS: &[(&str, &[&str])] = &[("TermsToken", &["phx-ledger"]), ("PhysicalToken", &["phx-ledger", "phx-geo"])];
/// The data-reading crates besides the kernel's register.
const DATA_READERS: &[&str] = &["phx-world", "phx-cli"];
/// The one module of the kernel that reads data.
const REGISTER: (&str, &str) = ("phx-core", "/src/register/");
const DATA_CRATES: [&str; 2] = ["toml", "serde"];
const PLACEHOLDERS: &str = "phx_check.placeholder_count";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(c, source));
        }
    }
    breaches.extend(placeholders(ws));
    breaches.extend(layout(ws));
    breaches
}

fn reads_data(c: &Crate, source: &Source) -> bool {
    DATA_READERS.contains(&c.name.as_str())
        || (c.name == REGISTER.0 && source.path.strip_prefix(&c.dir).is_some_and(|p| p.starts_with(REGISTER.1)))
}

fn check(c: &Crate, source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { krate: &c.name, reads_data: reads_data(c, source), found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

#[derive(Debug)]
struct Finder<'a> {
    krate: &'a str,
    reads_data: bool,
    found: Vec<(usize, String)>,
}

impl Finder<'_> {
    fn data_crate(&mut self, ident: &syn::Ident) {
        if !self.reads_data && DATA_CRATES.iter().any(|d| ident == d) {
            self.found.push((attrs::line(ident.span()), format!("`{ident}` read outside the register")));
        }
    }
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(p) = &*call.func {
            let names: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            for (token, allowed) in TOKENS {
                if names.ends_with(&[(*token).to_owned(), "new".to_owned()]) && !allowed.contains(&self.krate) {
                    let line = attrs::line(call.paren_token.span.open());
                    self.found.push((line, format!("a `{token}` built outside {allowed:?}")));
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(first) = path.segments.first() {
            self.data_crate(&first.ident);
        }
        visit::visit_path(self, path);
    }

    fn visit_use_tree(&mut self, tree: &'ast UseTree) {
        match tree {
            UseTree::Path(p) => self.data_crate(&p.ident),
            UseTree::Name(n) => self.data_crate(&n.ident),
            UseTree::Rename(r) => self.data_crate(&r.ident),
            UseTree::Glob(_) | UseTree::Group(_) => {}
        }
        visit::visit_use_tree(self, tree);
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

/// Where the committed data may lie: the world's constants, the shared primitives, the setups, the level templates,
/// the name tables and the inventory. A country's own data is instantiated in a run's directory at a new game and is
/// never committed.
const DATA_PLACES: &[&str] = &[
    "data/world.toml",
    "data/inventory.toml",
    "data/shared/",
    "data/setup/",
    "data/names/",
    "data/profiles/developed/",
    "data/profiles/emerging/",
    "data/profiles/developing/",
];

fn layout(ws: &Workspace) -> Vec<Breach> {
    ws.data
        .iter()
        .filter(|(path, _)| !DATA_PLACES.iter().any(|p| path == p || (p.ends_with('/') && path.starts_with(p))))
        .map(|(path, _)| {
            Breach::new(RULE, path, 1, "data outside its places: a country's own data belongs in a run's directory")
        })
        .collect()
}

/// The placeholder SHAPEs in the committed data, held to their ratchet: the count only falls, except by the
/// placeholders a stage brings for later systems, which edit the entry.
fn placeholders(ws: &Workspace) -> Vec<Breach> {
    let mut count = 0_u64;
    let mut breaches = Vec::new();
    for (path, text) in &ws.data {
        let table: toml::Table = match toml::from_str(text) {
            Ok(t) => t,
            Err(e) => {
                breaches.push(Breach::new(RULE, path, 1, format!("does not parse: {e}")));
                continue;
            }
        };
        let entries = table.get("primitive").and_then(toml::Value::as_array).map_or(&[][..], Vec::as_slice);
        let placeholder = |e: &&toml::Value| {
            e.get("shape").and_then(toml::Value::as_str).is_some_and(|s| s.starts_with("placeholder:"))
        };
        count += u64::try_from(entries.iter().filter(placeholder).count()).unwrap_or(u64::MAX);
    }
    let ratchets = match ratchets::parse(&ws.ratchets) {
        Ok(r) => r,
        Err(error) => return vec![Breach::new(RULE, RATCHETS, 1, error)],
    };
    match ratchets.find(PLACEHOLDERS) {
        None => breaches.push(Breach::new(RULE, RATCHETS, 1, format!("no ratchet for `{PLACEHOLDERS}`"))),
        Some(r) if r.breached_by(count) => {
            let message = format!("{count} placeholder SHAPEs in the data; the ratchet allows {}", r.value);
            breaches.push(Breach::new(RULE, RATCHETS, 1, message));
        }
        Some(_) => {}
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const RATCHET: &str = "[[ratchet]]\ncounter = \"phx_check.placeholder_count\"\nvalue = 1\ndirection = \"down\"\n";

    fn workspace(crates: Vec<crate::workspace::Crate>, data: &[&str]) -> Workspace {
        let mut ws = Workspace::new(crates);
        ws.ratchets = RATCHET.to_owned();
        ws.data = data.iter().enumerate().map(|(i, t)| (format!("data/shared/{i}.toml"), (*t).to_owned())).collect();
        ws
    }

    #[test]
    fn numbers_come_through_the_register() {
        let text = "use toml::Value;\n\
                    fn a() { let _ = phx_core::TermsToken::new(); }\n\
                    fn b() { let _ = PhysicalToken::new(); }\n\
                    #[cfg(test)]\nmod tests { fn t() { let _ = TermsToken::new(); } }";
        let lines = |name: &str, layer, path: &str| -> Vec<usize> {
            run(&workspace(vec![with_source(krate(name, layer), path, text)], &[])).iter().map(|b| b.line).collect()
        };
        assert_eq!(lines("sys-bnk", Layer::Systems, "src/lib.rs"), vec![1, 2, 3]);
        assert_eq!(lines("phx-ledger", Layer::Kernel, "src/lib.rs"), vec![1]);
        assert_eq!(lines("phx-geo", Layer::Kernel, "src/lib.rs"), vec![1, 2]);
        assert_eq!(lines("phx-core", Layer::Kernel, "src/register/mod.rs"), vec![2, 3]);
        assert_eq!(lines("phx-core", Layer::Kernel, "src/agenda.rs"), vec![1, 2, 3]);
    }

    #[test]
    fn placeholders_are_ratcheted() {
        let entry = |shape: &str| format!("[[primitive]]\nid = \"A.b\"\nshape = \"{shape}\"\nvalue = 1\n");
        let one = entry("placeholder:LAB");
        let two = format!("{one}{}", entry("placeholder:EDU"));
        assert!(run(&workspace(vec![], &[&one, &entry("standing:why")])).is_empty());
        assert_eq!(run(&workspace(vec![], &[&two])).len(), 1);
    }

    #[test]
    fn country_data_is_never_committed() {
        let mut ws = workspace(vec![], &[]);
        for path in [
            "data/world.toml",
            "data/profiles/emerging/TIME.toml",
            "data/names/real.toml",
            "data/0-noredia/GEN.toml",
            "data/profiles/kenya/GEN.toml",
        ] {
            ws.data.push((path.to_owned(), String::new()));
        }
        let found: Vec<String> = run(&ws).into_iter().map(|b| b.file).collect();
        assert_eq!(found, ["data/0-noredia/GEN.toml", "data/profiles/kenya/GEN.toml"]);
    }
}
