use syn::{GenericArgument, ItemStruct, PathArguments, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-27";
const LEDGER: &str = "phx-ledger";
/// The files of stage 7's passes, which keep per-party and per-account records and never the day's batch.
const PASSES: &[&str] = &["stream.rs", "fixed_point.rs", "apply_batch.rs", "batch.rs"];
/// What a batch is made of: its payments, their legs, the rows they were read from, and instructions.
const ITEMS: &[&str] = &["Payment", "LegRec", "RowView", "Instruction", "DueRow"];
/// The collections a field could hold them in.
const COLLECTIONS: &[&str] = &["Vec", "VecDeque", "BTreeMap", "BTreeSet", "HashMap", "HashSet", "SmallVec"];
/// The one list the architecture keeps: the day's payments 7a hands 7c, so 7c does not reckon them again.
const NAMED: &[(&str, &str)] = &[("DayRecords", "made")];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name == LEDGER) {
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench() && PASSES.contains(&s.file_name())) {
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
    let mut found = Vec::new();
    for item in &file.items {
        if let syn::Item::Struct(s) = item {
            found.extend(fields(s));
        }
    }
    found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// A struct's fields that keep a batch's items in a collection.
fn fields(item: &ItemStruct) -> Vec<(usize, String)> {
    if attrs::is_test(&item.attrs) {
        return Vec::new();
    }
    item.fields
        .iter()
        .filter(|f| holds(&f.ty, false))
        .filter(|f| {
            let name = f.ident.as_ref().map_or_else(String::new, ToString::to_string);
            !NAMED.contains(&(item.ident.to_string().as_str(), name.as_str()))
        })
        .map(|f| {
            let name = f.ident.as_ref().map_or_else(String::new, ToString::to_string);
            let message = format!("`{}.{name}` keeps the day's batch; keep per-party records instead", item.ident);
            (attrs::line(item.ident.span()), message)
        })
        .collect()
}

/// Whether a type names a batch item inside a collection, at any depth.
fn holds(ty: &Type, within: bool) -> bool {
    match ty {
        Type::Path(p) => p.path.segments.iter().any(|s| {
            let name = s.ident.to_string();
            if within && ITEMS.contains(&name.as_str()) {
                return true;
            }
            let inner = within || COLLECTIONS.contains(&name.as_str());
            match &s.arguments {
                PathArguments::AngleBracketed(a) => a.args.iter().any(|g| match g {
                    GenericArgument::Type(t) => holds(t, inner),
                    _ => false,
                }),
                _ => false,
            }
        }),
        Type::Tuple(t) => t.elems.iter().any(|e| holds(e, within)),
        Type::Array(a) => holds(&a.elem, within),
        Type::Slice(s) => holds(&s.elem, within),
        Type::Reference(r) => holds(&r.elem, within),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::fields;

    fn found(code: &str) -> usize {
        let file = syn::parse_file(code).unwrap();
        file.items
            .iter()
            .filter_map(|i| match i {
                syn::Item::Struct(s) => Some(fields(s).len()),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn no_materialised_batch_in_the_passes() {
        assert_eq!(found("struct R { records: BTreeMap<PartyId, Record>, failed: BTreeSet<(LineId, PartyId)> }"), 0);
        assert_eq!(found("struct R { payments: Vec<Payment> }"), 1);
        assert_eq!(found("struct R { held: Vec<Vec<LegRec>>, by: BTreeMap<LineId, Vec<Instruction>> }"), 2);
        assert_eq!(found("struct R { one: Payment, rows: Vec<(u16, RowView)> }"), 1, "a single item is no batch");
        assert_eq!(found("struct DayRecords { made: Vec<Payment>, also: Vec<Payment> }"), 1, "only the named list");
    }
}
