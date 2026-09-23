use super::{Breach, attrs, unparsed};
use crate::ratchets;
use crate::workspace::{RATCHETS, Workspace};

const RULE: &str = "PC-10";

const COUNTERS: &[(&str, &str)] = &[("allow", "phx_check.allow_count"), ("expect", "phx_check.expect_count")];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let ratchets = match ratchets::parse(&ws.ratchets) {
        Ok(r) => r,
        Err(error) => return vec![Breach::new(RULE, RATCHETS, 1, error)],
    };
    let mut breaches = Vec::new();
    let mut counts = [0_usize; 2];
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
                for ((level, _), count) in COUNTERS.iter().zip(counts.iter_mut()) {
                    *count += attrs::lint_levels(attr, level).len();
                }
            }
        }
    }
    for ((level, counter), count) in COUNTERS.iter().zip(counts) {
        match ratchets.find(counter) {
            None => breaches.push(Breach::new(RULE, RATCHETS, 1, format!("no ratchet for `{counter}`"))),
            Some(r) if u64::try_from(count).is_ok_and(|n| r.breached_by(n)) => {
                let message = format!("{count} `{level}` attributes in world crates; the ratchet allows {}", r.value);
                breaches.push(Breach::new(RULE, RATCHETS, 1, message));
            }
            Some(_) => {}
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const RATCHETS: &str = "[[ratchet]]\ncounter = \"phx_check.allow_count\"\nvalue = 0\ndirection = \"down\"\n\
                            [[ratchet]]\ncounter = \"phx_check.expect_count\"\nvalue = 1\ndirection = \"down\"\n";

    #[test]
    fn counts_held_to_their_ratchets() {
        let one = "#[expect(dead_code, reason = \"x\")]\nfn a() {}";
        let two = "#[cfg_attr(test, expect(dead_code, reason = \"x\"))]\nfn b() {}";
        let mut ws = Workspace::new(vec![with_source(krate("phx-core", Layer::Kernel), "src/a.rs", one)]);
        ws.ratchets = RATCHETS.to_owned();
        assert!(run(&ws).is_empty());
        ws.crates.push(with_source(krate("phx-geo", Layer::Kernel), "src/b.rs", two));
        assert_eq!(run(&ws).len(), 1);
        ws.ratchets = String::from("ratchet = []");
        assert_eq!(run(&ws).len(), 2);
    }
}
