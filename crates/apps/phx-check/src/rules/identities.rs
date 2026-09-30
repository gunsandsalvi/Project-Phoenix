//! PC-100: state that outlives a day names a party or a contract by a reference its generation checks, never a bare
//! slot, so a slot reused after its party ended is never read as that party. A saved struct's field is refused where
//! it is a map or set keyed by a bare slot or a party's key, or a list of pairs led by a party's key.

use syn::visit::{self, Visit};
use syn::{Attribute, Fields, GenericArgument, ItemStruct, PathArguments, Type};

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

#[cfg(test)]
mod tests {
    use super::Finder;
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

    #[test]
    fn unsaved_maps_are_not_read() {
        assert!(whats("#[derive(Debug, Default)] struct Scratch { by: BTreeMap<u32, u32> }").is_empty());
    }
}
