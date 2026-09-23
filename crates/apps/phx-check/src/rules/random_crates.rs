use super::Breach;
use crate::workspace::Workspace;

const RULE: &str = "PC-13";

/// Chance comes only from the world's own generator, and hashing only from its fixed-seed hasher.
const REFUSED: &[&str] = &["rand", "rand_core", "getrandom", "ahash", "fxhash"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for dep in c.deps.iter().filter(|d| REFUSED.contains(&d.name.as_str())) {
            let message = format!("`{}` may not depend on `{}`", c.name, dep.name);
            breaches.push(Breach::new(RULE, &c.manifest_path(), c.dependency_line(&dep.name), message));
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_dep};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn random_crates_refused_in_world_crates_only() {
        let bad = with_dep(krate("phx-rand", Layer::Foundation), "rand_core", false);
        let app = with_dep(krate("phx-cli", Layer::Apps), "getrandom", false);
        assert_eq!(run(&Workspace::new(vec![bad, app])).len(), 1);
    }
}
