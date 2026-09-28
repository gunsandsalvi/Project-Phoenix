use syn::visit::{self, Visit};
use syn::{ItemMod, TypePath, TypeTraitObject};

use super::{Breach, attrs, unparsed};
use crate::workspace::Workspace;

const RULE: &str = "PC-92";

/// The core's modules a day's work runs through, by crate and file: none may reach a map, whose layout scatters the
/// day's reads, or a trait object, whose calls the compiler cannot see through.
const HOT: &[(&str, &str)] = &[
    ("phx-store", "src/parties.rs"),
    ("phx-store", "src/edges.rs"),
    ("phx-exec", "src/partition.rs"),
    ("phx-core", "src/flows.rs"),
    ("phx-core", "src/wheel.rs"),
    ("phx-core", "src/settle.rs"),
    ("phx-core", "src/column_facts.rs"),
    ("phx-core", "src/goods.rs"),
    ("phx-core", "src/units.rs"),
    ("phx-market", "src/meet.rs"),
];

/// The maps a hot module may not name.
const MAPS: &[&str] = &["BTreeMap", "BTreeSet", "HashMap", "HashSet", "KernelMap"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates() {
        for source in &c.sources {
            let hot = HOT.iter().any(|(krate, file)| c.name == *krate && source.path == format!("{}/{file}", c.dir));
            if !hot {
                continue;
            }
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            let mut finder = Finder { found: Vec::new() };
            finder.visit_file(file);
            breaches.extend(finder.found.into_iter().map(|(line, m)| Breach::new(RULE, &source.path, line, m)));
        }
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_type_path(&mut self, ty: &'ast TypePath) {
        if let Some(seg) = ty.path.segments.iter().find(|s| MAPS.contains(&s.ident.to_string().as_str())) {
            self.found
                .push((attrs::line(seg.ident.span()), format!("`{}` in a module of the day's hot path", seg.ident)));
        }
        visit::visit_type_path(self, ty);
    }

    fn visit_type_trait_object(&mut self, ty: &'ast TypeTraitObject) {
        if let Some(token) = ty.dyn_token {
            self.found.push((attrs::line(token.span), "a trait object in a module of the day's hot path".to_owned()));
        }
        visit::visit_type_trait_object(self, ty);
    }
}

#[cfg(test)]
mod tests {
    use super::Finder;
    use syn::visit::Visit;

    #[test]
    fn hot_paths_refuse_maps_and_trait_objects() {
        let file: syn::File = syn::parse_str(
            "struct A { m: std::collections::BTreeMap<u32, u32>, f: Box<dyn Fn()> }\n\
             fn ok(v: &[u32]) -> usize { v.len() }\n\
             #[cfg(test)] mod tests { struct T { m: HashMap<u8, u8> } }",
        )
        .unwrap();
        let mut f = Finder { found: Vec::new() };
        f.visit_file(&file);
        assert_eq!(f.found.len(), 2, "a map and a trait object outside tests: {:?}", f.found);
    }
}
