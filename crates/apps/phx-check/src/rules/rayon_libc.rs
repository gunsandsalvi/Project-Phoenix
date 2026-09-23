use super::Breach;
use crate::workspace::Workspace;

const RULE: &str = "PC-04";

/// Threads and the operating system are reached from these crates alone; `rayon` itself from none.
const PLACES: &[(&str, &[&str])] =
    &[("rayon", &[]), ("rayon-core", &["phx-exec"]), ("libc", &["phx-exec", "phx-store"])];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in &ws.crates {
        for dep in &c.deps {
            let Some((_, places)) = PLACES.iter().find(|(name, _)| *name == dep.name) else {
                continue;
            };
            if !places.contains(&c.name.as_str()) {
                let message = format!("`{}` may not depend on `{}`", c.name, dep.name);
                breaches.push(Breach::new(RULE, &c.manifest_path(), c.dependency_line(&dep.name), message));
            }
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
    fn rayon_only_through_phx_exec() {
        let ok = with_dep(krate("phx-exec", Layer::Kernel), "rayon-core", false);
        let bad = with_dep(krate("phx-exec", Layer::Kernel), "rayon", false);
        let also_bad = with_dep(krate("phx-core", Layer::Kernel), "libc", false);
        assert_eq!(run(&Workspace::new(vec![ok, bad, also_bad])).len(), 2);
    }
}
