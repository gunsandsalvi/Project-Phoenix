mod clause;
mod consts;
mod decl;
mod pod;

use proc_macro::TokenStream;

/// Marks the item that carries a clause of the world; the item is emitted unchanged, and the checks read the mark.
#[proc_macro_attribute]
pub fn clause(args: TokenStream, item: TokenStream) -> TokenStream {
    clause::expand(args.into(), item.into()).into()
}

/// Makes a `#[repr(C)]` struct of storable fields, with no padding and no float, storable itself.
#[proc_macro_derive(Pod)]
pub fn derive_pod(input: TokenStream) -> TokenStream {
    pod::expand(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
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

/// A kind of party, `pub NAME = "kind" { legal_form: "…", table: Individuals | Cells, clause: "…" }`.
#[proc_macro]
pub fn declare_kind(input: TokenStream) -> TokenStream {
    decl::kind_decl(input.into()).into()
}

/// A fact stored as a column of a kind's table of individuals, `pub NAME = "SYS.fact" on "kind"`.
#[proc_macro]
pub fn declare_facet(input: TokenStream) -> TokenStream {
    decl::facet_decl(input.into()).into()
}
