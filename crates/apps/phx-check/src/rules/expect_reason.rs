use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-11";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in &c.sources {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            for attr in attrs::all(file) {
                for args in attrs::lint_levels(attr, "expect") {
                    if !attrs::has_ident(&args, "reason") {
                        let line = attrs::line(attr.pound_token.span);
                        breaches.push(Breach::new(RULE, &source.path, line, "an `expect` without a reason"));
                    }
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
    fn expect_needs_reason() {
        let text = "#[expect(clippy::too_many_lines, reason = \"one table\")]\nfn a() {}\n\
                    #[expect(clippy::too_many_lines)]\nfn b() {}\n\
                    #[cfg_attr(test, expect(dead_code))]\nfn c() {}";
        let c = with_source(krate("phx-core", Layer::Kernel), "src/lib.rs", text);
        let lines: Vec<usize> = run(&Workspace::new(vec![c])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![3, 5]);
    }
}
