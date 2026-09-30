//! PC-100: identities and absences are kept honestly. State that outlives a day names a party or a contract by a
//! reference its generation checks, never a bare slot, so a slot reused after its party ended is never read as that
//! party: a saved struct's field is refused where it is a map or set keyed by a bare slot or a party's key, or a list
//! of pairs led by a party's key. And an absent value is never read as zero unless the function says why the absence
//! truly is zero, through `#[absent_is_zero(reason = "…")]`.

use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprMethodCall, Fields, GenericArgument, ImplItemFn, ItemFn, ItemImpl, ItemMod, ItemStruct, Lit,
    PathArguments, Type,
};

use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::Workspace;

pub const RULE: &str = "PC-100";

/// Maps and sets, and the map keyed by a party's key whatever its value.
const MAPS: &[&str] = &["BTreeMap", "BTreeSet", "HashMap", "HashSet", "KernelMap"];
const PARTY_MAP: &str = "PartyMap";
/// Keys that name a row by where it is now rather than by what it is.
const BARE: &[&str] = &["u32", "PartyKey", "PartyId", "Slot", "EdgeSlot"];
/// The derive that saves a struct, and the mark of a field left out of the save.
const SAVED: &str = "Saved";

/// Every bare-slot key PC-100 finds in a saved struct, and a breach for each source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench()) {
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
            let mut finder = Finder::default();
            finder.visit_file(file);
            sites.extend(finder.found.into_iter().map(|(line, item, what)| Found {
                path: source.path.clone(),
                message: format!("{what} in saved state: key by a generation-checked reference, or make it a column"),
                item,
                what,
                line,
            }));
            let mut zeros = Zeros::default();
            zeros.visit_file(file);
            sites.extend(zeros.found.into_iter().map(|(line, item, what)| Found {
                path: source.path.clone(),
                message: format!(
                    "{what} reads an absent value as zero: refuse it, or say why with `#[absent_is_zero]`"
                ),
                item,
                what,
                line,
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

#[derive(Debug, Default)]
struct Finder {
    found: Vec<(usize, String, String)>,
}

fn derives_saved(attrs_: &[Attribute]) -> bool {
    attrs_.iter().any(|a| {
        a.path().is_ident("derive") && {
            let mut saved = false;
            let _ = a.parse_nested_meta(|m| {
                saved |= m.path.segments.last().is_some_and(|s| s.ident == SAVED);
                Ok(())
            });
            saved
        }
    })
}

fn skipped(attrs_: &[Attribute]) -> bool {
    attrs_.iter().any(|a| {
        a.path().is_ident("saved") && {
            let mut skip = false;
            let _ = a.parse_nested_meta(|m| {
                skip |= m.path.is_ident("skip");
                Ok(())
            });
            skip
        }
    })
}

fn last_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// A field type's outer name and its generic arguments' types.
fn generics(ty: &Type) -> Option<(String, Vec<&Type>)> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    let args = match &seg.arguments {
        PathArguments::AngleBracketed(a) => {
            a.args.iter().filter_map(|g| if let GenericArgument::Type(t) = g { Some(t) } else { None }).collect()
        }
        _ => Vec::new(),
    };
    Some((seg.ident.to_string(), args))
}

/// Why a field's type keys by a bare slot, if it does.
fn bare_key(ty: &Type) -> Option<String> {
    let (outer, args) = generics(ty)?;
    if outer == PARTY_MAP {
        return Some(format!("`{PARTY_MAP}`"));
    }
    if MAPS.contains(&outer.as_str()) {
        // A key of several parts is bare where the part leading it is.
        let key = match args.first()? {
            Type::Tuple(t) => t.elems.first().and_then(last_name)?,
            k => last_name(k)?,
        };
        return BARE.contains(&key.as_str()).then(|| format!("`{outer}` keyed by `{key}`"));
    }
    if outer == "Vec"
        && let Some(Type::Tuple(t)) = args.first()
        && let Some(first) = t.elems.first().and_then(last_name)
        && first == "PartyKey"
    {
        return Some("`Vec<(PartyKey, _)>`".to_owned());
    }
    None
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if attrs::is_test(&item.attrs) || !derives_saved(&item.attrs) {
            return;
        }
        if let Fields::Named(named) = &item.fields {
            for field in named.named.iter().filter(|f| !skipped(&f.attrs)) {
                let Some(ident) = &field.ident else { continue };
                if let Some(why) = bare_key(&field.ty) {
                    let what = format!("field `{ident}`: {why}");
                    self.found.push((attrs::line(ident.span()), item.ident.to_string(), what));
                }
            }
        }
        visit::visit_item_struct(self, item);
    }
}

/// The mark of a function whose absences are truly zero.
const ABSENT_IS_ZERO: &str = "absent_is_zero";
/// The zero of the number types.
const ZERO: &str = "ZERO";

/// Reads of an absent value as zero, outside functions that say why the absence is zero.
#[derive(Debug, Default)]
struct Zeros {
    found: Vec<(usize, String, String)>,
    items: Vec<String>,
}

fn is_zero(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => match &l.lit {
            Lit::Int(i) => i.base10_digits() == "0",
            Lit::Float(f) => f.base10_digits().chars().all(|c| c == '0' || c == '.'),
            _ => false,
        },
        Expr::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == ZERO),
        _ => false,
    }
}

fn declares_zero(attrs_: &[Attribute]) -> bool {
    attrs::is_test(attrs_) || attrs_.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == ABSENT_IS_ZERO))
}

impl Zeros {
    fn within(&mut self, name: String, visit: impl FnOnce(&mut Zeros)) {
        self.items.push(name);
        visit(self);
        self.items.pop();
    }
}

impl<'ast> Visit<'ast> for Zeros {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.within(format!("mod {}", item.ident), |f| visit::visit_item_mod(f, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        let name = last_name(&item.self_ty).unwrap_or_else(|| "impl".to_owned());
        self.within(name, |f| visit::visit_item_impl(f, item));
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !declares_zero(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_item_fn(f, item));
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !declares_zero(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_impl_item_fn(f, item));
        }
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();
        let zero_first = call.args.first().is_some_and(is_zero);
        let read = match method.as_str() {
            "unwrap_or" | "map_or" => zero_first,
            "unwrap_or_default" => call.args.is_empty(),
            _ => false,
        };
        if read {
            let item = if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") };
            self.found.push((attrs::line(call.method.span()), item, format!("`.{method}`")));
        }
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::{Finder, Zeros};
    use syn::visit::Visit;

    fn whats(text: &str) -> Vec<String> {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut f = Finder::default();
        f.visit_file(&file);
        f.found.into_iter().map(|(_, _, w)| w).collect()
    }

    #[test]
    fn bare_slot_keys_are_refused() {
        let found = whats(
            "#[derive(Debug, phx_macros::Saved)] struct Labour { noticed: BTreeMap<u32, Day>, since: BTreeMap<PartyKey, u32>, \
             open: BTreeSet<PartyId>, held: Vec<(PartyKey, i64)>, by: PartyMap<u8>, ok: BTreeMap<GenRef<Firm>, Day>, \
             refs: BTreeMap<PartyRef, u8>, #[saved(skip)] index: BTreeMap<u32, u32>, plain: Vec<u32>, \
             pairs: BTreeMap<(PartyKey, u16), i64> }",
        );
        assert_eq!(
            found,
            [
                "field `noticed`: `BTreeMap` keyed by `u32`",
                "field `since`: `BTreeMap` keyed by `PartyKey`",
                "field `open`: `BTreeSet` keyed by `PartyId`",
                "field `held`: `Vec<(PartyKey, _)>`",
                "field `by`: `PartyMap`",
                "field `pairs`: `BTreeMap` keyed by `PartyKey`"
            ]
        );
    }

    fn zeros(text: &str) -> Vec<String> {
        let file: syn::File = syn::parse_str(text).unwrap();
        let mut z = Zeros::default();
        z.visit_file(&file);
        z.found.into_iter().map(|(_, item, w)| format!("{item} {w}")).collect()
    }

    #[test]
    fn absent_as_zero_needs_a_reason() {
        let found = zeros(
            "impl Firm { fn sales(&self) -> i64 { let a = self.x.unwrap_or(0); let b = self.y.unwrap_or(0.0); \
             let c = self.z.unwrap_or(Money::ZERO); let d = self.w.map_or(0, |v| v + 1); let e = self.v.unwrap_or_default(); \
             let ok = self.u.unwrap_or(7); let m = self.t.map_or(1, |v| v); a } }",
        );
        assert_eq!(
            found,
            [
                "Firm::sales `.unwrap_or`",
                "Firm::sales `.unwrap_or`",
                "Firm::sales `.unwrap_or`",
                "Firm::sales `.map_or`",
                "Firm::sales `.unwrap_or_default`"
            ]
        );
    }

    #[test]
    fn declared_zero_count_is_admitted() {
        let text = "#[absent_is_zero(reason = \"a firm with no sale today sold none\")] fn sold(of: Option<u32>) -> u32 { of.unwrap_or(0) }\n\
                    #[cfg(test)] mod tests { fn t() { let _ = None::<u8>.unwrap_or(0); } }";
        assert!(zeros(text).is_empty());
    }

    #[test]
    fn unsaved_maps_are_not_read() {
        assert!(whats("#[derive(Debug, Default)] struct Scratch { by: BTreeMap<u32, u32> }").is_empty());
    }
}
