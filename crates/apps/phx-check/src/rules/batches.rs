use syn::{GenericArgument, ItemStruct, PathArguments, Type};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-27";

/// Settlement's passes, which keep per-party and per-account records and never the day's batch: the crate, its
/// passes' files, what a batch is made of, and the lists the architecture names.
struct Passes {
    krate: &'static str,
    files: &'static [&'static str],
    items: &'static [&'static str],
    named: &'static [(&'static str, &'static str)],
}

const PASSES: &[Passes] = &[
    // The ledger's stage 7: its payments, their legs, the rows they were read from and instructions; the day's
    // payments 7a hands 7c, and the buffer that keeps its room from one day to the next.
    Passes {
        krate: "phx-ledger",
        files: &["stream.rs", "fixed_point.rs", "apply_batch.rs", "batch.rs"],
        items: &["Payment", "LegRec", "RowView", "Instruction", "DueRow"],
        named: &[("DayRecords", "made"), ("DayBuffers", "made")],
    },
    // The core's settlement: the day's flows stay in the buffers that made them; only the failed and held are
    // handed back.
    Passes {
        krate: "phx-core",
        files: &["settle.rs"],
        items: &["Flow", "Credit"],
        named: &[("Outcome", "failed"), ("Outcome", "held")],
    },
];

/// The collections a field could hold them in.
const COLLECTIONS: &[&str] = &["Vec", "VecDeque", "BTreeMap", "BTreeSet", "HashMap", "HashSet", "SmallVec"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for passes in PASSES {
        for c in ws.world_crates().filter(|c| c.name == passes.krate) {
            for source in c.sources.iter().filter(|s| !s.is_test_or_bench() && passes.files.contains(&s.file_name())) {
                breaches.extend(check(source, passes));
            }
        }
    }
    breaches
}

fn check(source: &Source, passes: &Passes) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    let mut found = Vec::new();
    for item in &file.items {
        if let syn::Item::Struct(s) = item {
            found.extend(fields(s, passes));
        }
    }
    found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

/// A struct's fields that keep a batch's items in a collection.
fn fields(item: &ItemStruct, passes: &Passes) -> Vec<(usize, String)> {
    if attrs::is_test(&item.attrs) {
        return Vec::new();
    }
    item.fields
        .iter()
        .filter(|f| holds(&f.ty, false, passes.items))
        .filter(|f| {
            let name = f.ident.as_ref().map_or_else(String::new, ToString::to_string);
            !passes.named.contains(&(item.ident.to_string().as_str(), name.as_str()))
        })
        .map(|f| {
            let name = f.ident.as_ref().map_or_else(String::new, ToString::to_string);
            let message = format!("`{}.{name}` keeps the day's batch; keep per-party records instead", item.ident);
            (attrs::line(item.ident.span()), message)
        })
        .collect()
}

/// Whether a type names a batch item inside a collection, at any depth.
fn holds(ty: &Type, within: bool, items: &[&str]) -> bool {
    match ty {
        Type::Path(p) => p.path.segments.iter().any(|s| {
            let name = s.ident.to_string();
            if within && items.contains(&name.as_str()) {
                return true;
            }
            let inner = within || COLLECTIONS.contains(&name.as_str());
            match &s.arguments {
                PathArguments::AngleBracketed(a) => a.args.iter().any(|g| match g {
                    GenericArgument::Type(t) => holds(t, inner, items),
                    _ => false,
                }),
                _ => false,
            }
        }),
        Type::Tuple(t) => t.elems.iter().any(|e| holds(e, within, items)),
        Type::Array(a) => holds(&a.elem, within, items),
        Type::Slice(s) => holds(&s.elem, within, items),
        Type::Reference(r) => holds(&r.elem, within, items),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{PASSES, fields};

    fn found_in(code: &str, passes: usize) -> usize {
        let file = syn::parse_file(code).unwrap();
        file.items
            .iter()
            .filter_map(|i| match i {
                syn::Item::Struct(s) => Some(fields(s, &PASSES[passes]).len()),
                _ => None,
            })
            .sum()
    }

    fn found(code: &str) -> usize {
        found_in(code, 0)
    }

    #[test]
    fn settlement_keeps_no_flows() {
        assert_eq!(found_in("struct Settle { net: Vec<Vec<i64>>, removed: Vec<u64> }", 1), 0);
        assert_eq!(found_in("struct Settle { day: Vec<Flow>, credits: Vec<Vec<Credit>> }", 1), 2);
        assert_eq!(found_in("struct Outcome { failed: Vec<(Flow, Cause)>, held: Vec<Flow> }", 1), 0, "handed back");
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
