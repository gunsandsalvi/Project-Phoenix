use super::Breach;
use crate::workspace::{Crate, Layer, Workspace};

const RULE: &str = "PC-01";

/// The orders inside the foundation, the kernel, the interfaces, the assembly and the applications: a crate may use
/// only those before it. The kernel's lists the core's crates, those not yet created among them, so each is created in
/// its place.
pub const FOUNDATION: &[&str] = &["phx-macros", "phx-num", "phx-rand", "phx-id"];
pub const KERNEL: &[&str] = &[
    "phx-store",
    "phx-exec",
    "phx-core",
    "phx-geo",
    "phx-pop",
    "phx-record",
    "phx-agenda",
    "phx-ledger",
    "phx-contract",
    "phx-hold",
    "phx-market",
    "phx-acct",
    "phx-val",
    "phx-risk",
    "phx-end",
    "phx-mind",
    "phx-audit",
];
/// The applications' order: the run calls the finished-volume measure, and the play surface reads the world through
/// the run's entry points; the checker stands apart and last.
pub const APPS: &[&str] = &["phx-fin", "phx-cli", "phx-play", "phx-check"];
/// Applications that may read only the layers named: the finished-volume measure fills bases, never a world.
pub const LOWER_ONLY: &[(&str, &[Layer])] = &[("phx-fin", &[Layer::Foundation, Layer::Kernel])];
/// The assembly's order: the observer reads the world, which never reads the observer.
pub const ASSEMBLY: &[&str] = &["phx-world", "phx-obs"];
pub const INTERFACES: &[&str] = &[
    "if-base",
    "if-pop",
    "if-labour",
    "if-property",
    "if-firm",
    "if-banking",
    "if-credit",
    "if-securities",
    "if-risk",
    "if-energy",
    "if-open",
    "if-state",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in &ws.crates {
        if let Some(message) = unordered(c.layer, &c.name) {
            breaches.push(Breach::new(RULE, &c.manifest_path(), 1, message));
        }
        for dep in c.deps.iter().filter(|d| d.internal) {
            let Some(target) = ws.crates.iter().find(|t| t.name == dep.name) else {
                continue;
            };
            if let Err(message) = allowed(c, target) {
                breaches.push(Breach::new(RULE, &c.manifest_path(), c.dependency_line(&dep.name), message));
            }
        }
    }
    breaches
}

fn order(layer: Layer) -> Option<&'static [&'static str]> {
    match layer {
        Layer::Foundation => Some(FOUNDATION),
        Layer::Kernel => Some(KERNEL),
        Layer::Interfaces => Some(INTERFACES),
        Layer::Assembly => Some(ASSEMBLY),
        Layer::Apps => Some(APPS),
        Layer::Systems => None,
    }
}

fn unordered(layer: Layer, name: &str) -> Option<String> {
    let order = order(layer)?;
    if order.contains(&name) { None } else { Some(format!("`{name}` is not in its layer's order")) }
}

fn allowed(from: &Crate, to: &Crate) -> Result<(), String> {
    if let Some((_, layers)) = LOWER_ONLY.iter().find(|(name, _)| *name == from.name)
        && !layers.contains(&to.layer)
    {
        return Err(format!("`{}` reads only the foundation and the kernel, never `{}`", from.name, to.name));
    }
    if to.layer < from.layer {
        return Ok(());
    }
    if to.layer == from.layer
        && let Some(order) = order(from.layer)
    {
        let position = |name: &str| order.iter().position(|n| *n == name);
        if let (Some(a), Some(b)) = (position(&from.name), position(&to.name))
            && b < a
        {
            return Ok(());
        }
    }
    Err(format!("`{}` may not depend on `{}`", from.name, to.name))
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_dep};
    use crate::workspace::{Layer, Workspace};

    fn breaches(from: (&str, Layer), to: (&str, Layer)) -> usize {
        let ws = Workspace::new(vec![with_dep(krate(from.0, from.1), to.0, true), krate(to.0, to.1)]);
        run(&ws).len()
    }

    #[test]
    fn layering_refuses_system_on_system() {
        assert_eq!(breaches(("sys-hh", Layer::Systems), ("sys-lab", Layer::Systems)), 1);
    }

    #[test]
    fn layering_refuses_l0_order() {
        assert_eq!(breaches(("phx-num", Layer::Foundation), ("phx-rand", Layer::Foundation)), 1);
        assert_eq!(breaches(("phx-rand", Layer::Foundation), ("phx-num", Layer::Foundation)), 0);
    }

    #[test]
    fn interfaces_may_use_l1() {
        assert_eq!(breaches(("if-pop", Layer::Interfaces), ("phx-ledger", Layer::Kernel)), 0);
    }

    #[test]
    fn layering_allows_earlier_interface() {
        assert_eq!(breaches(("if-pop", Layer::Interfaces), ("if-base", Layer::Interfaces)), 0);
    }

    #[test]
    fn layering_refuses_later_interface() {
        assert_eq!(breaches(("if-base", Layer::Interfaces), ("if-pop", Layer::Interfaces)), 1);
    }

    #[test]
    fn kernel_order_is_the_core_map() {
        assert_eq!(breaches(("phx-hold", Layer::Kernel), ("phx-ledger", Layer::Kernel)), 0);
        assert_eq!(breaches(("phx-ledger", Layer::Kernel), ("phx-hold", Layer::Kernel)), 1);
        assert_eq!(
            breaches(("phx-ledger", Layer::Kernel), ("phx-pop", Layer::Kernel)),
            0,
            "the population below money"
        );
    }

    #[test]
    fn phx_fin_reads_only_the_kernel() {
        assert_eq!(breaches(("phx-fin", Layer::Apps), ("phx-world", Layer::Assembly)), 1);
        assert_eq!(breaches(("phx-fin", Layer::Apps), ("if-pop", Layer::Interfaces)), 1);
        assert_eq!(breaches(("phx-fin", Layer::Apps), ("phx-store", Layer::Kernel)), 0);
    }

    #[test]
    fn apps_follow_their_order() {
        assert_eq!(breaches(("phx-cli", Layer::Apps), ("phx-fin", Layer::Apps)), 0);
        assert_eq!(breaches(("phx-fin", Layer::Apps), ("phx-cli", Layer::Apps)), 1);
        assert_eq!(breaches(("phx-play", Layer::Apps), ("phx-cli", Layer::Apps)), 0);
    }

    #[test]
    fn systems_keep_no_order() {
        assert_eq!(breaches(("sys-lab", Layer::Systems), ("sys-hh", Layer::Systems)), 1);
        let ws = Workspace::new(vec![krate("sys-new", Layer::Systems)]);
        assert!(run(&ws).is_empty(), "a system needs no place in an order");
    }

    #[test]
    fn sweep_hooks_keep_the_order() {
        assert_eq!(
            breaches(("phx-ledger", Layer::Kernel), ("phx-acct", Layer::Kernel)),
            1,
            "a sweep naming a higher base"
        );
        let hooked = Workspace::new(vec![krate("phx-ledger", Layer::Kernel), krate("phx-acct", Layer::Kernel)]);
        assert!(run(&hooked).is_empty(), "the same sweep through its own hook parameter names no higher crate");
    }

    #[test]
    fn layering_refuses_upward_and_unordered() {
        assert_eq!(breaches(("phx-core", Layer::Kernel), ("sys-hh", Layer::Systems)), 1);
        assert_eq!(breaches(("phx-cli", Layer::Apps), ("phx-world", Layer::Assembly)), 0);
        let ws = Workspace::new(vec![krate("phx-extra", Layer::Kernel)]);
        assert_eq!(run(&ws).len(), 1);
    }
}
