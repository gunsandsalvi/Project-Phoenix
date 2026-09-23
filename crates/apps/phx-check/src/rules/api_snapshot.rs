use super::Breach;
use crate::workspace::{API_SNAPSHOT, Crate, Layer, Workspace};

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

/// The first line where the committed snapshot and the current API differ, or none when they agree.
pub fn first_difference(snapshot: &str, current: &str) -> Option<(usize, String)> {
    let (mut old, mut new) = (snapshot.lines(), current.lines());
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

#[cfg(test)]
mod tests {
    use super::{first_difference, run};
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
    fn differences_name_their_line() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
        assert_eq!(first_difference("a\nb\n", "a\nc\n").map(|d| d.0), Some(2));
        assert_eq!(first_difference("a\n", "a\nb\n").map(|d| d.0), Some(2));
    }
}
