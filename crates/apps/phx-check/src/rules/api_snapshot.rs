use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};

use super::{Breach, attrs};
use crate::workspace::{API_SNAPSHOT, Crate, Layer, Workspace};

/// The column a snapshot's line carries the crates that use its item in, after the item.
const USED_BY: &str = "  // used by:";

const RULE: &str = "PC-15";

/// Kernel and interface crates are the surfaces every system builds on, so their public API changes only on purpose.
pub fn needs_snapshot(c: &Crate) -> bool {
    matches!(c.layer, Layer::Kernel | Layer::Interfaces)
}

/// Every kernel and interface crate commits its public API; `phx-check public-api` compares the snapshot with the
/// API the crate has now.
pub fn run(ws: &Workspace) -> Vec<Breach> {
    ws.crates
        .iter()
        .filter(|c| needs_snapshot(c) && c.api_snapshot.is_none())
        .map(|c| {
            let message = "no public-API snapshot; `phx-check public-api --write` records one";
            Breach::new(RULE, &format!("{}/{API_SNAPSHOT}", c.dir), 1, message)
        })
        .collect()
}

/// The first line where the committed snapshot and the current API differ, or none when they agree; the "used by"
/// column is a report beside the API, not part of it.
pub fn first_difference(snapshot: &str, current: &str) -> Option<(usize, String)> {
    let (mut old, mut new) = (snapshot.lines().map(api_line), current.lines());
    for line in 1.. {
        match (old.next(), new.next()) {
            (None, None) => return None,
            (Some(a), Some(b)) if a == b => {}
            (a, b) => {
                let show = |l: Option<&str>| l.map_or_else(|| "(end)".to_owned(), |l| format!("`{l}`"));
                return Some((line, format!("snapshot has {}, the crate has {}", show(a), show(b))));
            }
        }
    }
    None
}

/// A snapshot line without its "used by" column.
fn api_line(line: &str) -> &str {
    line.split_once(USED_BY).map_or(line, |(api, _)| api)
}

/// The words that may lead an item's path on a line of the API.
const KINDS: &[&str] = &["fn", "struct", "enum", "mod", "trait", "const", "static", "type", "unsafe", "macro", "use"];

/// The name a line of the API declares: its path's last part; none for a trait's implementation, which declares nothing.
#[must_use]
pub fn item_name(line: &str) -> Option<String> {
    let rest = api_line(line).trim().strip_prefix("pub ")?;
    let mut words = rest.split(' ');
    let mut path = words.next()?;

    while KINDS.contains(&path) {
        path = words.next()?;
    }
    let path = path.split(['(', '<', '=']).next()?.trim_end_matches(':');
    path.rsplit("::").next().filter(|n| !n.is_empty()).map(str::to_owned)
}

/// Every identifier a crate's sources name outside their tests.
#[must_use]
pub fn used_idents(c: &Crate) -> BTreeSet<String> {
    let mut used = Idents::default();
    for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
        if let Ok(file) = &source.file
            && !attrs::is_test(&file.attrs)
        {
            used.visit_file(file);
        }
    }
    used.names
}

#[derive(Default)]
struct Idents {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for Idents {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }
    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        self.names.insert(ident.to_string());
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // A macro's tokens are no syntax tree, but the names in them are used all the same.
        for tt in mac.tokens.clone() {
            if let proc_macro2::TokenTree::Ident(i) = tt {
                self.names.insert(i.to_string());
            }
        }
        visit::visit_macro(self, mac);
    }
}

/// Each crate's used identifiers, by crate.
#[must_use]
pub fn usage(ws: &Workspace) -> BTreeMap<String, BTreeSet<String>> {
    ws.crates.iter().map(|c| (c.name.clone(), used_idents(c))).collect()
}

/// The crates outside `owner` that name an item.
fn users<'a>(name: &str, owner: &str, usage: &'a BTreeMap<String, BTreeSet<String>>) -> Vec<&'a str> {
    usage.iter().filter(|(c, names)| *c != owner && names.contains(name)).map(|(c, _)| c.as_str()).collect()
}

/// An API with each item's users beside it.
#[must_use]
pub fn annotate(api: &str, owner: &str, usage: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut out = String::new();
    for line in api.lines() {
        out.push_str(line);
        if let Some(name) = item_name(line) {
            let who = users(&name, owner, usage);
            if !who.is_empty() {
                out.push_str(USED_BY);
                out.push(' ');
                out.push_str(&who.join(", "));
            }
        }
        out.push('\n');
    }
    out
}

/// Every public item of a kernel or interface crate that no crate outside it names, by crate and line.
#[must_use]
pub fn unused(ws: &Workspace, usage: &BTreeMap<String, BTreeSet<String>>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in ws.crates.iter().filter(|c| needs_snapshot(c)) {
        for line in c.api_snapshot.as_deref().unwrap_or_default().lines() {
            if let Some(name) = item_name(line)
                && users(&name, &c.name, usage).is_empty()
            {
                out.push((c.name.clone(), api_line(line).to_owned()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{annotate, first_difference, item_name, run, unused, usage};
    use crate::workspace::fixture::krate;
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn kernel_and_interface_crates_need_snapshots() {
        let mut store = krate("phx-store", Layer::Kernel);
        let terms = krate("if-base", Layer::Interfaces);
        let num = krate("phx-num", Layer::Foundation);
        assert_eq!(run(&Workspace::new(vec![store.clone(), terms.clone(), num.clone()])).len(), 2);
        store.api_snapshot = Some("pub mod phx_store\n".to_owned());
        assert_eq!(run(&Workspace::new(vec![store, terms, num])).len(), 1);
    }

    #[test]
    fn unused_items_are_listed() {
        use crate::workspace::fixture::with_source;
        let mut exec = with_source(krate("phx-exec", Layer::Kernel), "src/lib.rs", "pub fn used() {} pub fn idle() {}");
        let api = "pub mod phx_exec\npub fn phx_exec::used()\npub fn phx_exec::idle()\n";
        let core = with_source(
            krate("phx-core", Layer::Kernel),
            "src/a.rs",
            "fn f() { phx_exec::used(); }\n#[cfg(test)] mod t { fn g() { phx_exec::idle(); } }",
        );
        exec.api_snapshot = Some(api.to_owned());
        let ws = Workspace::new(vec![exec, core]);
        let usage = usage(&ws);
        let listed: Vec<String> = unused(&ws, &usage).into_iter().map(|(_, l)| l).collect();
        assert_eq!(listed, ["pub fn phx_exec::idle()"], "a use in tests is no use");
        let written = annotate(api, "phx-exec", &usage);
        assert!(written.contains("pub fn phx_exec::used()  // used by: phx-core"), "{written}");
        assert_eq!(first_difference(&written, api), None, "the column is no part of the API");
    }

    #[test]
    fn item_names_are_read() {
        assert_eq!(item_name("pub fn phx_exec::clock::Clock::now_ns(&self) -> u64").as_deref(), Some("now_ns"));
        assert_eq!(item_name("pub struct phx_exec::pool::PoolUsage").as_deref(), Some("PoolUsage"));
        assert_eq!(item_name("pub phx_exec::pool::PoolUsage::chunks: u64").as_deref(), Some("chunks"));
        assert_eq!(item_name("pub const fn phx_id::Slot::new(raw: u32) -> Self").as_deref(), Some("new"));
        assert_eq!(item_name("impl core::fmt::Debug for phx_exec::Pool"), None);
    }

    #[test]
    fn differences_name_their_line() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
        assert_eq!(first_difference("a\nb\n", "a\nc\n").map(|d| d.0), Some(2));
        assert_eq!(first_difference("a\n", "a\nb\n").map(|d| d.0), Some(2));
    }
}
