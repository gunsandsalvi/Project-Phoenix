//! EVERY `@spec` CITATION NAMES A CLAUSE THAT EXISTS.

use std::collections::{HashMap, HashSet};

pub struct Spec {
    /// Section key (`"37"`, `"XI-15"`) to the clause ids declared under it.
    clauses: HashMap<String, HashSet<String>>,
    /// A section's NAME, normalised, to its key — so `Treasury B3` and `30 B3` are one citation.
    by_name: HashMap<String, String>,
    /// And each heading split into words, for the short names a reader actually writes.
    headings: Vec<(Vec<String>, String)>,
    laws: usize,
    appendices: HashSet<String>,
}

/// `TRADE CREDIT` and `Trade Credit` and `trade-credit` are one name.
fn normal(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

impl Spec {
    pub fn read(text: &str) -> Spec {
        let mut clauses: HashMap<String, HashSet<String>> = HashMap::new();
        let mut by_name: HashMap<String, String> = HashMap::new();
        let mut headings: Vec<(Vec<String>, String)> = Vec::new();
        let mut appendices: HashSet<String> = HashSet::new();
        let mut laws = 0usize;
        let mut at: Option<String> = None;
        let mut in_part_one = false;

        for line in text.lines() {
            let t = line.trim_start();
            if let Some(rest) = line.strip_prefix("# ") {
                in_part_one = rest.starts_with("PART I ");
                if let Some(a) = rest.strip_prefix("APPENDIX ") {
                    if let Some(letter) = a.split_whitespace().next() {
                        appendices.insert(letter.to_string());
                    }
                }
                // A PART heading ends the previous section: a clause after it belongs to whatever
                // section comes next, and to nothing until one does.
                at = None;
                continue;
            }
            // Part I's laws are `### 1.
            if in_part_one {
                if let Some(rest) = line.strip_prefix("### ") {
                    if rest.split('.').next().is_some_and(|n| n.parse::<usize>().is_ok()) {
                        laws += 1;
                    }
                }
            }
            if let Some(rest) = line.strip_prefix("## ") {
                let (head, name) = match rest.split_once(". ") {
                    Some((h, n)) => (h.trim().to_string(), n.trim().to_string()),
                    // A prose heading with no number is not a section a clause can be cited under.
                    None => {
                        at = None;
                        continue;
                    }
                };
                let numbered = head.parse::<usize>().is_ok() || head.starts_with("XI-");
                if !numbered {
                    at = None;
                    continue;
                }
                by_name.insert(normal(&name), head.clone());
                headings.push((words(&name), head.clone()));
                clauses.entry(head.clone()).or_default();
                at = Some(head);
                continue;
            }
            // `- A2.a...`, at any indent.
            let Some(key) = at.as_ref() else { continue };
            let Some(rest) = t.strip_prefix("- **") else { continue };
            let Some((id, _)) = rest.split_once("**") else { continue };
            if id.is_empty() || !id.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                continue;
            }
            clauses.entry(key.clone()).or_default().insert(id.to_string());
        }
        Spec { clauses, by_name, headings, laws, appendices }
    }

    /// Whether the specification carries what this citation names.
    pub fn resolve(&self, citation: &str) -> Option<String> {
        let c = citation.trim().trim_end_matches(&['.', ','][..]).trim();
        if c.is_empty() {
            return None;
        }
        // Another document's.
        if c.starts_with("ARCHITECTURE") || c.starts_with("PLAN") || c.starts_with("Part ") || c.starts_with("Appendix A") {
            return None;
        }
        if let Some(n) = c.strip_prefix("Law ") {
            let Ok(n) = n.trim().parse::<usize>() else {
                return Some(format!("`{c}` does not name a law"));
            };
            return if n >= 1 && n <= self.laws {
                None
            } else {
                Some(format!("`{c}`, and there are {} laws", self.laws))
            };
        }
        if let Some(a) = c.strip_prefix("Appendix ") {
            let letter = a.split_whitespace().next().unwrap_or("");
            return if self.appendices.contains(letter) {
                None
            } else {
                Some(format!("`{c}`: there is no such appendix"))
            };
        }
        let c = c.strip_prefix('§').unwrap_or(c);
        // `37 A2.a`, `XI-15`, `Treasury B3`, `46`.
        let (head, clause) = match c.rsplit_once(' ') {
            Some((h, tail)) if looks_like_clause(tail) => (h.trim(), Some(tail)),
            _ => (c, None),
        };
        let Some(key) = self.section(head) else {
            return Some(format!("`{c}`: there is no section `{head}`"));
        };
        let Some(clause) = clause else { return None };
        // `A1–A4` is one citation of its two ends, and both have to be there.
        for one in clause.split(['–', '—']).filter(|s| !s.is_empty()) {
            // A whole lettered group is there when any clause of it is.
            if one.len() == 1 {
                if self.clauses[&key].iter().any(|id| id.starts_with(one)) {
                    continue;
                }
                return Some(format!("`{c}`: section {key} has no group {one}"));
            }
            if !self.clauses[&key].contains(one) {
                return Some(format!("`{c}`: section {key} has no clause {one}"));
            }
        }
        None
    }

    /// The section a citation's head names, by number or by the SHORT NAME a reader writes.
    pub fn section(&self, head: &str) -> Option<String> {
        if self.clauses.contains_key(head) {
            return Some(head.to_string());
        }
        if let Some(k) = self.by_name.get(&normal(head)) {
            return Some(k.clone());
        }
        let wanted = words(head);
        if wanted.is_empty() {
            return None;
        }
        for (name, key) in &self.headings {
            if runs_through(&wanted, name) {
                return Some(key.clone());
            }
        }
        None
    }
}

/// A heading or a citation split into comparable words: lowercase, letters and digits only, and the
/// leading `the` dropped because `Treasury` and `THE TREASURY` are one section.
fn words(s: &str) -> Vec<String> {
    let mut out: Vec<String> = s
        .split(|c: char| !c.is_alphanumeric() && c != '&')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();
    if out.first().is_some_and(|w| w == "the") {
        out.remove(0);
    }
    out
}

/// Whether the citation's words run through the heading's in order, each matching a heading word or
/// standing for the initials of the next few.
fn runs_through(wanted: &[String], heading: &[String]) -> bool {
    let mut at = 0usize;
    for w in wanted {
        let mut matched = false;
        while at < heading.len() {
            if heading[at] == *w {
                at += 1;
                matched = true;
                break;
            }
            // `FX` for `FOREIGN EXCHANGE`, `CDS` for `CREDIT DEFAULT SWAPS`.
            let initials: String = heading[at..]
                .iter()
                .take(w.len())
                .filter_map(|h| h.chars().next())
                .collect();
            if initials == *w && heading.len() - at >= w.len() {
                at += w.len();
                matched = true;
                break;
            }
            at += 1;
        }
        if !matched {
            return false;
        }
    }
    true
}

/// `A2`, `B3.a`, `D12`, the range `A1–A4`, and the whole lettered group `C`.
fn looks_like_clause(s: &str) -> bool {
    // A range is one citation of its ends, and it reads as a clause if its ends do.
    if let Some((from, to)) = s.split_once(['–', '—']) {
        return looks_like_clause(from) && looks_like_clause(to);
    }
    // `Banks Lending C` cites a whole lettered GROUP — a real citation, and the coarsest one there
    // is.
    if s.len() == 1 && s.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return true;
    }
    let mut cs = s.chars();
    let Some(first) = cs.next() else { return false };
    if !first.is_ascii_uppercase() {
        return false;
    }
    let rest: Vec<char> = cs.collect();
    if rest.is_empty() || !rest[0].is_ascii_digit() {
        return false;
    }
    rest.iter().all(|c| c.is_ascii_digit() || *c == '.' || c.is_ascii_lowercase())
}

/// Every citation in one file's `@spec` lines, with the line each is on.
pub fn citations(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut section: Option<String> = None;
    for (n, line) in text.lines().enumerate() {
        let Some(at) = line.find("@spec ") else {
            // A citation list runs across `@spec` lines and stops at the first line that is not one.
            if !line.trim_start().starts_with("//") {
                section = None;
            }
            continue;
        };
        let list = &line[at + "@spec ".len()..];
        for part in list.split(['·', ',']) {
            let t = part.trim();
            if t.is_empty() {
                continue;
            }
            if looks_like_clause(t.split(['–', '—', '-']).next().unwrap_or(t)) {
                match &section {
                    Some(s) => out.push((n + 1, format!("{s} {t}"))),
                    // A clause with no section before it names nothing a reader can follow.
                    None => out.push((n + 1, t.to_string())),
                }
                continue;
            }
            section = head_of(t);
            out.push((n + 1, t.to_string()));
        }
    }
    out
}

/// The section part of a citation, for the bare clauses that follow it.
fn head_of(c: &str) -> Option<String> {
    if c.starts_with("Law ") || c.starts_with("Appendix ") || c.starts_with("Part ") {
        return None;
    }
    match c.rsplit_once(' ') {
        Some((h, tail)) if looks_like_clause(tail) => Some(h.trim().to_string()),
        _ => Some(c.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "# PART I — FIRST PRINCIPLES\n\
        ### 1. Reflect the real mechanism\n\
        ### 2. The fewest primitives\n\
        # PART V — THE MARKETS\n\
        ## 37. GOODS\n\
        ### A. What a good is\n\
        - **A1** REASON — a good is a thing.\n\
        - **A2** REASON — it is produced.\n\
        \x20 - **A2.a** fixed input quantities.\n\
        ## XI-15. THE UNIT OF REPRESENTATION\n\
        - **B1** REASON — a party is named or a cell.\n\
        # APPENDIX B — THE PROHIBITIONS, CONSOLIDATED\n";

    #[test]
    fn a_clause_that_exists_resolves_and_one_that_does_not_is_named() {
        let s = Spec::read(SPEC);
        assert!(s.resolve("37 A2.a").is_none());
        assert!(s.resolve("37 A1").is_none());
        // The defect this exists for: a citation that reads as evidence and names nothing.
        let miss = s.resolve("37 C1.b").unwrap();
        assert!(miss.contains("no clause C1.b"), "{miss}");
    }

    #[test]
    fn a_section_by_name_is_the_same_citation_as_a_section_by_number() {
        // `Treasury B3` and `30 B3` are one thing, and a reader writes whichever is clearer.
        let s = Spec::read(SPEC);
        assert!(s.resolve("Goods A2").is_none());
        assert!(s.resolve("GOODS A2").is_none());
        assert!(s.resolve("Goods D9").is_some());
    }

    #[test]
    fn a_law_outside_the_nineteen_is_a_finding_and_the_count_comes_from_the_document() {
        let s = Spec::read(SPEC);
        assert!(s.resolve("Law 1").is_none());
        assert!(s.resolve("Law 2").is_none());
        // Two laws in this fixture, so the third does not exist — the count is read, never stated.
        let miss = s.resolve("Law 3").unwrap();
        assert!(miss.contains("there are 2 laws"), "{miss}");
    }

    #[test]
    fn the_part_xi_mechanisms_and_the_appendices_resolve_by_their_own_names() {
        let s = Spec::read(SPEC);
        assert!(s.resolve("XI-15").is_none());
        assert!(s.resolve("XI-15 B1").is_none());
        assert!(s.resolve("XI-99").is_some());
        assert!(s.resolve("Appendix B").is_none());
        assert!(s.resolve("Appendix Z").is_some());
    }

    #[test]
    fn a_citation_list_splits_on_both_separators_and_keeps_its_line() {
        let found = citations("//! @spec 37 A2 · Law 4, Law 8 · XI-15\n");
        let ids: Vec<&str> = found.iter().map(|(_, c)| c.as_str()).collect();
        assert_eq!(ids, vec!["37 A2", "Law 4", "Law 8", "XI-15"]);
        assert!(found.iter().all(|(n, _)| *n == 1));
    }

    #[test]
    fn a_name_ending_in_a_number_is_not_a_clause() {
        // `looks_like_clause` decides where the section ends and the clause begins, and a bare
        // number at the end of a name would split the citation in the wrong place.
        assert!(looks_like_clause("A2"));
        assert!(looks_like_clause("B3.a"));
        assert!(!looks_like_clause("2"));
        assert!(!looks_like_clause("Credit"));
    }
}
