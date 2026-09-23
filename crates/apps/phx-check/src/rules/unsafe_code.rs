use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-03";

/// The storage, the pool and the foreign boundary are the only crates that may lower the `unsafe_code` lint.
const ALLOWED: &[&str] = &["phx-store", "phx-exec", "phx-ffi"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.crates.iter().filter(|c| !ALLOWED.contains(&c.name.as_str())) {
        for source in &c.sources {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            for attr in attrs::all(file) {
                let lowered = ["allow", "expect"]
                    .iter()
                    .flat_map(|level| attrs::lint_levels(attr, level))
                    .any(|args| attrs::has_ident(&args, "unsafe_code"));
                if lowered {
                    let line = attrs::line(attr.pound_token.span);
                    breaches.push(Breach::new(RULE, &source.path, line, "`unsafe_code` lowered outside its crates"));
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

    #[test]
    fn unsafe_lowered_only_in_its_crates() {
        let text = "#![allow(unsafe_code)]\n#[cfg_attr(test, expect(unsafe_code, reason = \"x\"))]\nfn f() {}";
        let store = with_source(krate("phx-store", Layer::Kernel), "src/lib.rs", text);
        let core = with_source(krate("phx-core", Layer::Kernel), "src/lib.rs", text);
        assert_eq!(run(&Workspace::new(vec![store, core])).len(), 2);
    }
}
