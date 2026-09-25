use super::Breach;
use crate::workspace::{Crate, DepKind, Layer, Workspace};

const RULE: &str = "PC-02";

/// Where each external crate may be a direct dependency: by crate name, or by layer as `foundation`, `kernel` or
/// `systems`.
pub const ALLOWED: &[(&str, &[&str])] = &[
    ("hashbrown", &["phx-core", "phx-store"]),
    ("foldhash", &["phx-core", "phx-store"]),
    ("libm", &["foundation", "kernel", "systems"]),
    ("rayon-core", &["phx-exec"]),
    ("libc", &["phx-exec", "phx-store"]),
    ("zstd", &["phx-store"]),
    ("serde", &["phx-core", "phx-world", "phx-obs", "phx-cli", "phx-ffi", "phx-check"]),
    ("toml", &["phx-core", "phx-world", "phx-obs", "phx-cli", "phx-ffi", "phx-check"]),
    ("serde_json", &["phx-cli", "phx-check", "phx-world"]),
    ("clap", &["phx-cli", "phx-check"]),
    ("regex", &["phx-check"]),
    ("uniffi", &["phx-ffi"]),
    ("ndk-sys", &["phx-ffi"]),
    ("syn", &["phx-macros", "phx-check"]),
    ("quote", &["phx-macros", "phx-check"]),
    ("proc-macro2", &["phx-macros", "phx-check"]),
    ("cargo_metadata", &["phx-check"]),
];

/// Allowed anywhere, as development dependencies only.
pub const DEV_ANYWHERE: &[&str] = &["proptest", "trybuild", "gungraun"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in &ws.crates {
        for dep in c.deps.iter().filter(|d| !d.internal) {
            if dep.kind == DepKind::Dev && DEV_ANYWHERE.contains(&dep.name.as_str()) {
                continue;
            }
            if !allowed_in(&dep.name, c) {
                let message = format!("`{}` is not allowed as a direct dependency of `{}`", dep.name, c.name);
                breaches.push(Breach::new(RULE, &c.manifest_path(), c.dependency_line(&dep.name), message));
            }
        }
    }
    breaches
}

pub fn allowed_in(dep: &str, c: &Crate) -> bool {
    let layer = match c.layer {
        Layer::Foundation => "foundation",
        Layer::Kernel => "kernel",
        Layer::Systems => "systems",
        Layer::Interfaces | Layer::Assembly | Layer::Apps => "",
    };
    ALLOWED.iter().any(|(name, places)| *name == dep && places.iter().any(|p| *p == c.name || *p == layer))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_dep};
    use crate::workspace::{DepKind, Dependency, Layer, Workspace};

    #[test]
    fn dependencies_follow_the_allow_list() {
        let ok = with_dep(krate("phx-num", Layer::Foundation), "libm", false);
        let bad = with_dep(krate("sys-hh", Layer::Systems), "serde", false);
        assert_eq!(run(&Workspace::new(vec![ok, bad])).len(), 1);
    }

    #[test]
    fn dev_tools_allowed_only_as_dev() {
        let mut c = krate("phx-num", Layer::Foundation);
        c.deps.push(Dependency { name: "proptest".to_owned(), kind: DepKind::Dev, internal: false });
        assert!(run(&Workspace::new(vec![c.clone()])).is_empty());
        c.deps.push(Dependency { name: "proptest".to_owned(), kind: DepKind::Normal, internal: false });
        assert_eq!(run(&Workspace::new(vec![c])).len(), 1);
    }
}
