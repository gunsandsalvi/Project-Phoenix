//! PC-92: the modules a day's work runs through hold no structure that scatters its reads or hides its calls — no map,
//! no list of lists, no trait object, no per-party column wider than its value or optional, and no threading outside
//! the store's own lists. The hot set is derived from the crates, never listed: every module of the core's crates,
//! the world's day modules and the systems' daily rules, less the cold ones named with their reasons.

use syn::visit::{self, Visit};
use syn::{
    Fields, GenericArgument, ImplItemFn, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemTrait, PathArguments,
    Type, TypePath, TypeTraitObject,
};

use super::layering::KERNEL;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::{Crate, Source, Workspace};

pub const RULE: &str = "PC-92";

/// Modules of hot crates the day never runs through, each with why: assembly, the opening and saving.
const COLD: &[(&str, &str, &str)] = &[
    ("phx-core", "src/register/", "the register is read at assembly"),
    ("phx-core", "src/contribution.rs", "contributions are laid down at the opening"),
    ("phx-store", "src/save.rs", "saving"),
    ("phx-store", "src/save_values.rs", "saving"),
    ("phx-store", "src/encode.rs", "saving"),
    ("phx-store", "src/descriptor.rs", "saving"),
    ("phx-store", "src/rebuild.rs", "a load's rebuild pass"),
    ("phx-store", "src/roundtrip.rs", "saving"),
    ("phx-store", "src/hash.rs", "hashing a save or the register"),
    ("phx-world", "src/registry.rs", "assembly"),
    ("phx-world", "src/compile.rs", "assembly"),
    ("phx-world", "src/save/", "saving"),
];

/// Any hot crate's modules under an `opening/` directory are the opening's.
const OPENING: &str = "/opening/";

/// The maps a hot module may not name.
const MAPS: &[&str] = &["BTreeMap", "BTreeSet", "HashMap", "HashSet", "KernelMap", "PartyMap"];
/// Field names that thread rows into lists, which only the store's edge tables and block lists may do.
const THREADS: &[&str] = &["next", "prev", "heads"];
const THREAD_PREFIX: &str = "next_";
/// The crate whose lists thread rows.
const STORE: &str = "phx-store";

/// Whether a source is on the day's path: a module of a core crate, one of the world's day modules or a system's daily
/// rule, and none of the cold ones.
#[must_use]
pub fn is_hot(c: &Crate, source: &Source) -> bool {
    let Some(rel) = source.path.strip_prefix(&format!("{}/", c.dir)) else { return false };
    if !rel.starts_with("src/") || source.is_test_or_bench() || source.path.contains(OPENING) {
        return false;
    }
    if COLD.iter().any(|(krate, prefix, _)| c.name == *krate && rel.starts_with(prefix)) {
        return false;
    }
    let file = source.file_name();
    if KERNEL.contains(&c.name.as_str()) {
        return true;
    }
    if c.name == "phx-world" {
        return rel.split('/').count() == 2 && (file == "day.rs" || file.starts_with("core_"));
    }
    c.name.starts_with("sys-") && rel.starts_with("src/rules/")
}

/// Every site PC-92 finds in the hot set, and a breach for each hot source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| is_hot(c, s)) {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            if attrs::is_test(&file.attrs) {
                continue;
            }
            let mut finder = Finder { found: Vec::new(), items: Vec::new(), threads_allowed: c.name == STORE };
            finder.visit_file(file);
            sites.extend(finder.found.into_iter().map(|(line, item, what, message)| Found {
                path: source.path.clone(),
                item,
                what,
                line,
                message,
            }));
        }
    }
    (sites, breaches)
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let (sites, mut breaches) = found(ws);
    match Exceptions::load(ws, RULE) {
        Ok(ex) => breaches.extend(ex.judge(&ws.ratchets, sites)),
        Err(b) => breaches.push(b),
    }
    breaches
}

#[derive(Debug)]
struct Finder {
    found: Vec<(usize, String, String, String)>,
    /// The items open around the visit, outermost first.
    items: Vec<String>,
    threads_allowed: bool,
}

impl Finder {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
    }

    fn push(&mut self, line: usize, what: &str, message: String) {
        self.found.push((line, self.item(), what.to_owned(), message));
    }

    fn within<T>(&mut self, name: String, visit: impl FnOnce(&mut Finder) -> T) -> T {
        self.items.push(name);
        let out = visit(self);
        self.items.pop();
        out
    }

    /// A field's type as R1 refuses it: a list of lists, or a per-party list or column wider than its value or
    /// optional.
    fn field_type(&mut self, line: usize, name: &str, ty: &Type) {
        let Some((outer, inner)) = outer_and_inner(ty) else { return };
        if outer != "Vec" && outer != "Column" {
            return;
        }
        let what = match inner.as_deref() {
            Some("Vec") if outer == "Vec" => "Vec<Vec>",
            Some("i128") => {
                if outer == "Vec" {
                    "Vec<i128>"
                } else {
                    "Column<i128>"
                }
            }
            Some("Option") => {
                if outer == "Vec" {
                    "Vec<Option>"
                } else {
                    "Column<Option>"
                }
            }
            _ => return,
        };
        self.push(
            line,
            what,
            format!("field `{name}` a `{what}` on the day's path: a column dense by slot, one value wide"),
        );
    }
}

/// A type's last path segment and its first generic argument's, if it is a path.
fn outer_and_inner(ty: &Type) -> Option<(String, Option<String>)> {
    let Type::Path(TypePath { path, .. }) = ty else { return None };
    let seg = path.segments.last()?;
    let inner = match &seg.arguments {
        PathArguments::AngleBracketed(args) => args.args.iter().find_map(|a| match a {
            GenericArgument::Type(Type::Path(p)) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        }),
        _ => None,
    };
    Some((seg.ident.to_string(), inner))
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p.path.segments.last().map_or_else(String::new, |s| s.ident.to_string()),
        _ => "impl".to_owned(),
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.within(format!("mod {}", item.ident), |f| visit::visit_item_mod(f, item));
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_item_fn(f, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if !attrs::is_test(&item.attrs) {
            self.within(type_name(&item.self_ty), |f| visit::visit_item_impl(f, item));
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !attrs::is_test(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_impl_item_fn(f, item));
        }
    }

    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        self.within(item.ident.to_string(), |f| visit::visit_item_trait(f, item));
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        self.within(item.ident.to_string(), |f| visit::visit_item_enum(f, item));
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        self.within(item.ident.to_string(), |f| {
            if let Fields::Named(named) = &item.fields {
                for field in &named.named {
                    let Some(ident) = &field.ident else { continue };
                    let (name, line) = (ident.to_string(), attrs::line(ident.span()));
                    f.field_type(line, &name, &field.ty);
                    let threads = THREADS.contains(&name.as_str()) || name.starts_with(THREAD_PREFIX);
                    if threads && !f.threads_allowed {
                        let message = format!("field `{name}` threads rows outside the store's lists");
                        f.push(line, "thread", message);
                    }
                }
            }
            visit::visit_item_struct(f, item);
        });
    }

    fn visit_type_path(&mut self, ty: &'ast TypePath) {
        if let Some(seg) = ty.path.segments.iter().find(|s| MAPS.contains(&s.ident.to_string().as_str())) {
            let what = seg.ident.to_string();
            self.push(attrs::line(seg.ident.span()), &what, format!("`{what}` in a module of the day's hot path"));
        }
        visit::visit_type_path(self, ty);
    }

    fn visit_type_trait_object(&mut self, ty: &'ast TypeTraitObject) {
        if let Some(token) = ty.dyn_token {
            self.push(attrs::line(token.span), "dyn", "a trait object in a module of the day's hot path".to_owned());
        }
        visit::visit_type_trait_object(self, ty);
    }
}

#[cfg(test)]
mod tests {
    use super::{Finder, is_hot, run};
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};
    use syn::visit::Visit;

    fn finds(text: &str, threads_allowed: bool) -> Vec<(String, String)> {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut f = Finder { found: Vec::new(), items: Vec::new(), threads_allowed };
        f.visit_file(&file);
        f.found.into_iter().map(|(_, item, what, _)| (item, what)).collect()
    }

    #[test]
    fn hot_paths_refuse_maps_threads_and_wide_columns() {
        let text = "struct A { m: std::collections::BTreeMap<u32, u32>, f: Box<dyn Fn()>, l: Vec<Vec<u32>>, \
                    w: Vec<i128>, c: Column<i128, B>, o: Vec<Option<u32>>, p: Column<Option<u8>, B>, next: Vec<u32>, \
                    total: i128, fine: Vec<u64> }\n\
                    impl A { fn g(&self) -> PartyMap<u8> { todo!() } }\n\
                    fn ok(v: &[u32]) -> usize { v.len() }\n\
                    #[cfg(test)] mod tests { struct T { m: HashMap<u8, u8> } }";
        let found = finds(text, false);
        let whats: Vec<&str> = found.iter().map(|(_, w)| w.as_str()).collect();
        assert_eq!(
            whats,
            [
                "Vec<Vec>",
                "Vec<i128>",
                "Column<i128>",
                "Vec<Option>",
                "Column<Option>",
                "thread",
                "BTreeMap",
                "dyn",
                "PartyMap"
            ],
            "each refused form once; a scalar i128 total and a plain Vec admitted: {found:?}"
        );
        assert!(found.iter().any(|(item, what)| item == "A::g" && what == "PartyMap"), "the enclosing item is named");
        assert!(!finds("struct S { next: Vec<u32> }", true).iter().any(|(_, w)| w == "thread"), "the store threads");
    }

    #[test]
    fn hot_set_is_derived() {
        let core = with_source(krate("phx-core", Layer::Kernel), "src/brand_new.rs", "fn a() {}");
        let core = with_source(core, "src/register/forms.rs", "fn a() {}");
        let hot: Vec<bool> = core.sources.iter().map(|s| is_hot(&core, s)).collect();
        assert_eq!(hot, [true, false], "a new file of a core crate is hot; the register is cold");
        let world = with_source(krate("phx-world", Layer::Assembly), "src/core_goods.rs", "fn a() {}");
        let world = with_source(world, "src/registry.rs", "fn a() {}");
        let world = with_source(world, "src/opening/firms.rs", "fn a() {}");
        assert_eq!(world.sources.iter().map(|s| is_hot(&world, s)).collect::<Vec<_>>(), [true, false, false]);
        let sys = with_source(krate("sys-lab", Layer::Systems), "src/rules/search.rs", "fn a() {}");
        let sys = with_source(sys, "src/law.rs", "fn a() {}");
        assert_eq!(sys.sources.iter().map(|s| is_hot(&sys, s)).collect::<Vec<_>>(), [true, false]);
    }

    #[test]
    fn interfaces_are_not_hot() {
        let c = with_source(krate("if-pop", Layer::Interfaces), "src/lib.rs", "struct A { m: BTreeMap<u8, u8> }");
        assert!(run(&Workspace::new(vec![c])).is_empty());
    }

    #[test]
    fn test_modules_are_skipped() {
        let text = "#![cfg(test)]\nstruct A { m: BTreeMap<u8, u8> }";
        let c = with_source(krate("phx-core", Layer::Kernel), "src/goods_tests.rs", text);
        let c = with_source(c, "src/goods.rs", "#[cfg(test)] mod t { struct A { m: BTreeMap<u8, u8> } }");
        assert!(run(&Workspace::new(vec![c])).is_empty());
    }
}
