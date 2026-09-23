use syn::visit::{self, Visit};
use syn::{ExprCall, ExprMethodCall, ExprStruct, Field, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-23";
const GEO: &str = "phx-geo";
/// The map's geometry: what only `phx-geo` builds, and what no other crate keeps a copy of.
const GEOMETRY: &[&str] = &["Tile", "Zone", "Region", "Grid", "Map", "Segment", "GeoState"];
/// The map's builders.
const BUILDERS: &[&str] = &["generate", "grow", "join_nearest", "pick_seeds", "sea_distance"];
/// Ways of moving a site after its party is placed.
const SITE_MOVES: &[&str] = &["set_site", "write_site", "move_site"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name != GEO) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
            breaches.extend(check(source));
        }
    }
    breaches
}

fn check(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { imported: imported(file), found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

struct Finder {
    imported: Vec<String>,
    found: Vec<(usize, String)>,
}

/// The names a file imports from `phx-geo`, as the file calls them.
fn imported(file: &syn::File) -> Vec<String> {
    fn walk(tree: &syn::UseTree, under_geo: bool, out: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(p) => walk(&p.tree, under_geo || p.ident == "phx_geo", out),
            syn::UseTree::Name(n) if under_geo => out.push(n.ident.to_string()),
            syn::UseTree::Rename(r) if under_geo => out.push(r.rename.to_string()),
            syn::UseTree::Group(g) => g.items.iter().for_each(|t| walk(t, under_geo, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for item in &file.items {
        if let syn::Item::Use(u) = item {
            walk(&u.tree, false, &mut out);
        }
    }
    out
}

impl Finder {
    /// Whether a path names the map's geometry: through `phx_geo`, or by a name imported from it.
    fn names_geometry(&self, path: &syn::Path) -> Option<String> {
        let id = last(path)?;
        let through = path.segments.iter().any(|s| s.ident == "phx_geo") || self.imported.iter().any(|n| id == n);
        (through && GEOMETRY.iter().any(|g| id == g)).then(|| id.to_string())
    }
}

fn last(path: &syn::Path) -> Option<&syn::Ident> {
    path.segments.last().map(|s| &s.ident)
}

/// Whether a type names the map's geometry anywhere in it.
fn geometric(finder: &Finder, ty: &syn::Type) -> Option<String> {
    struct Names<'a>(&'a Finder, Option<String>);
    impl<'ast> Visit<'ast> for Names<'_> {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if let Some(name) = self.0.names_geometry(path) {
                self.1 = Some(name);
            }
            visit::visit_path(self, path);
        }
    }
    let mut names = Names(finder, None);
    names.visit_type(ty);
    names.1
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_struct(&mut self, e: &'ast ExprStruct) {
        if let Some(name) = self.names_geometry(&e.path) {
            let line = last(&e.path).map_or(1, |id| attrs::line(id.span()));
            self.found.push((line, format!("`{name}` is built only by phx-geo")));
        }
        visit::visit_expr_struct(self, e);
    }

    fn visit_expr_call(&mut self, e: &'ast ExprCall) {
        if let syn::Expr::Path(p) = &*e.func {
            let segments: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let tile = segments.iter().any(|s| s == "phx_geo") || self.imported.iter().any(|n| n == "Tile");
            let builds_tile = tile && segments.windows(2).any(|w| matches!(w, [t, n] if t == "Tile" && n == "new"));
            let builds_map = segments.iter().any(|s| s == "phx_geo")
                && segments.last().is_some_and(|s| BUILDERS.contains(&s.as_str()));
            if builds_tile || builds_map {
                let line = p.path.segments.last().map_or(1, |s| attrs::line(s.ident.span()));
                self.found.push((line, format!("`{}` is called only inside phx-geo", segments.join("::"))));
            }
        }
        visit::visit_expr_call(self, e);
    }

    fn visit_expr_method_call(&mut self, e: &'ast ExprMethodCall) {
        if SITE_MOVES.iter().any(|m| e.method == m) {
            self.found
                .push((attrs::line(e.method.span()), format!("`{}` moves a site; ownership never does", e.method)));
        }
        visit::visit_expr_method_call(self, e);
    }

    fn visit_field(&mut self, f: &'ast Field) {
        if let Some(name) = geometric(self, &f.ty)
            && name != "GeoState"
        {
            let line = f.ident.as_ref().map_or(1, |i| attrs::line(i.span()));
            self.found.push((line, format!("a field keeps map geometry (`{name}`) outside phx-geo")));
        }
        visit::visit_field(self, f);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if SITE_MOVES.iter().any(|m| item.sig.ident == m) {
            self.found.push((attrs::line(item.sig.ident.span()), format!("`{}` moves a site", item.sig.ident)));
        }
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if SITE_MOVES.iter().any(|m| item.sig.ident == m) {
            self.found.push((attrs::line(item.sig.ident.span()), format!("`{}` moves a site", item.sig.ident)));
        }
        if !attrs::is_test(&item.attrs) {
            visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Finder;
    use syn::visit::Visit;

    fn found(code: &str) -> usize {
        let file = syn::parse_file(&format!("use phx_geo::{{grid::Grid, tile::Tile, GeoState}};\n{code}")).unwrap();
        let mut f = Finder { imported: super::imported(&file), found: Vec::new() };
        f.visit_file(&file);
        f.found.len()
    }

    #[test]
    fn places_belong_to_geo() {
        assert_eq!(found("fn f() { let t = Tile::new(0, 1, 0, 0, None); }"), 1, "a tile built outside");
        assert_eq!(found("fn f() { let g = Grid { width: 1, height: 1, tile_m: 1 }; }"), 1, "a grid built outside");
        assert_eq!(found("fn f() { phx_geo::generate::generate(&p, &c, &d); }"), 1, "a map generated outside");
        assert_eq!(found("struct S { tiles: Vec<Tile> }"), 1, "geometry kept outside");
        assert_eq!(found("fn f(t: &mut T) { t.set_site(3); }"), 1, "a site moved");
        assert_eq!(found("struct S { geo: Arc<GeoState>, site: TileId }"), 0, "GEO's own state and a site are fine");
        assert_eq!(found("struct S { r: Region }"), 0, "a name not from phx-geo is someone else's");
    }
}
