//! The sites a rule admits for now: each listed by path, enclosing item and what it is, with how many, in
//! `exceptions/<rule>.toml` beside the checker. A site found beyond its count is a breach; a listed site found fewer
//! times than its count is too, so the file only shrinks; and the total is held to its ratchet, which only falls.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;

use crate::ratchets;
use crate::rules::Breach;
use crate::workspace::{RATCHETS, Workspace};

/// Where the exceptions files are, from the workspace root.
pub const DIR: &str = "crates/apps/phx-check/exceptions";

/// A site a rule found: the file, the item it stands in, what it is, the line and the rule's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: String,
    pub item: String,
    pub what: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Site {
    path: String,
    item: String,
    what: String,
    count: usize,
}

#[derive(Debug, Default, Deserialize)]
struct File {
    #[serde(default)]
    site: Vec<Site>,
}

/// How a rule finds its sites, with a breach for each source it cannot read.
pub type Finder = fn(&Workspace) -> (Vec<Found>, Vec<Breach>);

/// A rule's admitted sites.
#[derive(Debug, Default)]
pub struct Exceptions {
    rule: &'static str,
    sites: Vec<Site>,
}

/// The file a rule's exceptions are in.
#[must_use]
pub fn path(rule: &str) -> String {
    format!("{DIR}/{rule}.toml")
}

/// The ratchet a rule's total is held to: `phx_check.exceptions_pcNN`.
#[must_use]
pub fn counter(rule: &str) -> String {
    format!("phx_check.exceptions_{}", rule.to_lowercase().replace('-', ""))
}

type Key = (String, String, String);

impl Exceptions {
    /// A rule's exceptions as the workspace holds them; none where it has no file.
    ///
    /// # Errors
    /// A file that does not parse.
    pub fn load(ws: &Workspace, rule: &'static str) -> Result<Exceptions, Breach> {
        let Some((_, text)) = ws.exceptions.iter().find(|(r, _)| r == rule) else {
            return Ok(Exceptions { rule, sites: Vec::new() });
        };
        let file: File =
            toml::from_str(text).map_err(|e| Breach::new(rule, &path(rule), 1, format!("does not parse: {e}")))?;
        Ok(Exceptions { rule, sites: file.site })
    }

    /// The total the file admits.
    #[must_use]
    pub fn total(&self) -> usize {
        self.sites.iter().map(|s| s.count).sum()
    }

    /// Every site found beyond what is admitted, every listed site found fewer times than listed, and a total above
    /// its ratchet.
    #[must_use]
    pub fn judge(&self, ratchets_text: &str, found: Vec<Found>) -> Vec<Breach> {
        let mut by_key: BTreeMap<Key, Vec<Found>> = BTreeMap::new();
        for f in found {
            by_key.entry((f.path.clone(), f.item.clone(), f.what.clone())).or_default().push(f);
        }
        let mut breaches = Vec::new();
        for (key, sites) in &by_key {
            let admitted = self.admitted(key);
            breaches.extend(
                sites.iter().skip(admitted).map(|f| Breach::new(self.rule, &f.path, f.line, f.message.clone())),
            );
        }
        for s in &self.sites {
            let found = by_key.get(&(s.path.clone(), s.item.clone(), s.what.clone())).map_or(0, Vec::len);
            if found < s.count {
                let message = format!(
                    "stale exception: `{}` in `{}` is found {found} times of the {} listed; lower its count or remove it",
                    s.what, s.item, s.count
                );
                breaches.push(Breach::new(self.rule, &path(self.rule), 1, message));
            }
        }
        let total = self.total();
        if total > 0 {
            let name = counter(self.rule);
            match ratchets::parse(ratchets_text) {
                Err(e) => breaches.push(Breach::new(self.rule, RATCHETS, 1, e)),
                Ok(r) => match r.find(&name) {
                    None => breaches.push(Breach::new(self.rule, RATCHETS, 1, format!("no ratchet for `{name}`"))),
                    Some(r) if u64::try_from(total).is_ok_and(|t| r.breached_by(t)) => breaches.push(Breach::new(
                        self.rule,
                        RATCHETS,
                        1,
                        format!("{total} exceptions admitted; the ratchet allows {}", r.value),
                    )),
                    Some(_) => {}
                },
            }
        }
        breaches
    }

    fn admitted(&self, (path, item, what): &Key) -> usize {
        self.sites.iter().filter(|s| s.path == *path && s.item == *item && s.what == *what).map(|s| s.count).sum()
    }
}

/// The file listing every site found, grouped and counted, in path order: what a rule admits when it is first
/// widened over the code as it stands.
#[must_use]
pub fn write(rule: &str, found: &[Found]) -> String {
    let mut counts: BTreeMap<Key, usize> = BTreeMap::new();
    for f in found {
        *counts.entry((f.path.clone(), f.item.clone(), f.what.clone())).or_default() += 1;
    }
    let total: usize = counts.values().sum();
    let mut out = format!(
        "# The sites {rule} admits for now, {total} in all, held to `{}`. Each migration removes its own; a site \
         is never added.\n",
        counter(rule)
    );
    for ((path, item, what), count) in counts {
        let _ = write!(out, "\n[[site]]\npath = {path:?}\nitem = {item:?}\nwhat = {what:?}\ncount = {count}\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Exceptions, Found, Site, write};

    fn found(item: &str, what: &str, line: usize) -> Found {
        Found { path: "a.rs".to_owned(), item: item.to_owned(), what: what.to_owned(), line, message: what.to_owned() }
    }

    const RATCHET: &str = "[[ratchet]]\ncounter = \"phx_check.exceptions_pc92\"\nvalue = 2\ndirection = \"down\"\n";

    fn listed(count: usize) -> Exceptions {
        let site = Site { path: "a.rs".to_owned(), item: "Core".to_owned(), what: "BTreeMap".to_owned(), count };
        Exceptions { rule: "PC-92", sites: vec![site] }
    }

    #[test]
    fn exceptions_admit_listed_sites_up_to_count() {
        let two = vec![found("Core", "BTreeMap", 3), found("Core", "BTreeMap", 9)];
        assert!(listed(2).judge(RATCHET, two.clone()).is_empty());
        let mut three = two;
        three.push(found("Core", "BTreeMap", 12));
        let breaches = listed(2).judge(RATCHET, three);
        assert_eq!(breaches.iter().map(|b| b.line).collect::<Vec<_>>(), vec![12], "the one beyond the count");
        assert_eq!(listed(1).judge(RATCHET, vec![found("Other", "BTreeMap", 4)]).len(), 2, "not listed, and stale");
    }

    #[test]
    fn stale_exception_is_refused() {
        let breaches = listed(2).judge(RATCHET, vec![found("Core", "BTreeMap", 3)]);
        assert_eq!(breaches.len(), 1);
        assert!(breaches[0].message.starts_with("stale exception"));
        assert_eq!(listed(3).judge(RATCHET, vec![found("Core", "BTreeMap", 3); 3]).len(), 1, "3 over the ratchet of 2");
    }

    #[test]
    fn written_file_reads_back() {
        let text = write("PC-92", &[found("Core", "BTreeMap", 3), found("Core", "BTreeMap", 9), found("f", "dyn", 1)]);
        let file: super::File = toml::from_str(&text).unwrap();
        assert_eq!(file.site.iter().map(|s| s.count).collect::<Vec<_>>(), vec![2, 1]);
    }
}
