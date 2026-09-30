mod absent;
mod clause;
mod consts;
mod decl;
mod markers;
mod opening;
mod pod;
mod saved;
mod sweep;

use proc_macro::TokenStream;

/// Marks the item that carries a clause of the world; the item is emitted unchanged, and the checks read the mark.
#[proc_macro_attribute]
pub fn clause(args: TokenStream, item: TokenStream) -> TokenStream {
    clause::expand(args.into(), item.into()).into()
}

/// Marks a function that reads an absent value as zero because the absence truly is zero — a count of an entry that is
/// not there — saying why; the function is emitted unchanged, and the checks admit its reads.
#[proc_macro_attribute]
pub fn absent_is_zero(args: TokenStream, item: TokenStream) -> TokenStream {
    absent::expand(&args.into(), item.into()).into()
}

/// Marks a function that runs only while the world is assembled or opened, where names are bound to handles; the
/// function is emitted unchanged, and the checks admit its reads by name.
#[proc_macro_attribute]
pub fn opening(args: TokenStream, item: TokenStream) -> TokenStream {
    opening::expand(args.into(), item.into()).into()
}

/// Marks the function that runs a declared sweep — the few passes a clause needs over a whole store — naming its
/// store and the cycle its slices share the store over, or the reason it runs on; the function is emitted unchanged,
/// and the checks admit its walks.
#[proc_macro_attribute]
pub fn sweep(args: TokenStream, item: TokenStream) -> TokenStream {
    sweep::expand(args.into(), item.into()).into()
}

/// Marks a function whose value is computed once a day per party; it is named only as what the day's cache computes,
/// never called. The function is emitted unchanged.
#[proc_macro_attribute]
pub fn per_day(args: TokenStream, item: TokenStream) -> TokenStream {
    markers::per_day(&args.into(), item.into()).into()
}

/// Marks the aggregates a struct maintains, each `#[maintained(writer = path)]` on its field, naming its one writer; a
/// field so marked is an integer, so the audit's recount from source rows equals it. Emits nothing.
#[proc_macro_derive(Maintained, attributes(maintained))]
pub fn derive_maintained(input: TokenStream) -> TokenStream {
    markers::maintained(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Makes a `#[repr(C)]` struct of storable fields, with no padding and no float, storable itself.
#[proc_macro_derive(Pod, attributes(save))]
pub fn derive_pod(input: TokenStream) -> TokenStream {
    pod::expand(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Saves a struct's or an enum's fields in order, an enum's variant first; a field marked `#[saved(skip)]` is an index
/// its owner rebuilds after a load, and reads back as its default.
#[proc_macro_derive(Saved, attributes(saved))]
pub fn derive_saved(input: TokenStream) -> TokenStream {
    saved::expand(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// A primitive's declaration, `pub NAME = "SYS.name" { kind: …, value: …, clause: "…", scope: …, … }`, checked as it
/// is written: its identity's form, a policy's decider, a SHAPE's shape, and no field twice or unknown.
#[proc_macro]
pub fn declare_prim(input: TokenStream) -> TokenStream {
    decl::prim_decl(input.into()).into()
}

/// A fact's interface item, `pub NAME = "SYS.name" { value: …, kinds: […], writer: …, audience: …, repr: …, clause:
/// "…" }`, written by the system its name begins with.
#[proc_macro]
pub fn declare_fact(input: TokenStream) -> TokenStream {
    decl::fact_decl(input.into()).into()
}

/// A kind of party, `pub NAME = "kind" { legal_form: "…", place: Site { word: 0 }, clause: "…" }`: its place one of
/// `Site`, `Region` or `Country` with its record word, or `Sited`.
#[proc_macro]
pub fn declare_kind(input: TokenStream) -> TokenStream {
    decl::kind_decl(input.into()).into()
}

/// A named stream as a type, `pub Name = "SYS.process" { purpose: …, clause: "…" }`, one per purpose.
#[proc_macro]
pub fn declare_stream(input: TokenStream) -> TokenStream {
    decl::stream_decl(input.into()).into()
}

/// A hazard process, `pub NAME = "SYS.name" { acts_on: …, rate: "SYS.table", axes: […], outcome: "…", scheme: …,
/// stream: "SYS.stream", source: "…", clause: "…" }`.
#[proc_macro]
pub fn declare_hazard(input: TokenStream) -> TokenStream {
    decl::hazard_decl(input.into()).into()
}
