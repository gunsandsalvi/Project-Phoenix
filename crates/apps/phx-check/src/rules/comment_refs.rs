use regex::RegexSet;

use super::Breach;
use crate::comments::comments;
use crate::workspace::Workspace;

const RULE: &str = "PC-07";

/// A comment says why; a reference to a clause, a law, a document, a step or unfinished work belongs elsewhere.
const PATTERNS: &[&str] = &[
    r"\b[A-Z]{2,4}\.\d+\b",
    r"\bLaw \d+\b",
    r"\bN\d+(\.\d+)?\b",
    r"§",
    r"\bspec(ification)?\b",
    r"ARCHITECTURE",
    r"IMPLEMENTATION",
    r"PROJECT_PHOENIX",
    r"\bS\d+\.\d+\b",
    r"TODO",
    r"FIXME",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let set = match RegexSet::new(PATTERNS) {
        Ok(set) => set,
        Err(error) => return vec![Breach::new(RULE, "crates/apps/phx-check", 1, error.to_string())],
    };
    let mut breaches = Vec::new();
    for c in &ws.crates {
        for source in &c.sources {
            for comment in comments(&source.text) {
                if set.is_match(&comment.text) {
                    let message = format!("a reference in a comment: `{}`", comment.text.trim());
                    breaches.push(Breach::new(RULE, &source.path, comment.line, message));
                }
            }
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    fn count(text: &str) -> usize {
        let c = with_source(krate("phx-cli", Layer::Apps), "src/main.rs", text);
        run(&Workspace::new(vec![c])).len()
    }

    #[test]
    fn comments_refuse_references() {
        assert_eq!(count("// see REP.8\n// per Law 6\n/// as N8.5 says\nfn f() {}"), 3);
        assert_eq!(count("// the ring settles together\n// inspect the special case\n// CPU architecture"), 0);
        assert_eq!(count("let s = \"REP.8\"; // TODO"), 1);
    }
}
