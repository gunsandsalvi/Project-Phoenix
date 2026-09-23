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

/// A named stream as a type, `pub Name = "SYS.process" { purpose: …, clause: "…" }`, one per purpose.
#[proc_macro]
pub fn declare_stream(input: TokenStream) -> TokenStream {
    decl::stream_decl(input.into()).into()
}

/// A message kind as a type: what it reaches, who answers each kind and in which sub-step, and what it opens.
#[proc_macro]
pub fn declare_message(input: TokenStream) -> TokenStream {
    decl::message_decl(input.into()).into()
}

/// A hazard process, `pub NAME = "SYS.name" { acts_on: …, rate: "SYS.table", axes: […], outcome: "…", scheme: …,
/// stream: "SYS.stream", source: "…", clause: "…" }`.
#[proc_macro]
pub fn declare_hazard(input: TokenStream) -> TokenStream {
    decl::hazard_decl(input.into()).into()
}

/// A decision point, `pub NAME: Input => Output = "SYS.name" { rule: f, schedule: "…" or wakes: […], … }`.
#[proc_macro]
pub fn declare_decision(input: TokenStream) -> TokenStream {
    decl::decision_decl(input.into()).into()
}

/// A rule handle's signature, `pub NAME: Input => Output = "SYS.name"`, implemented by the system it names.
#[proc_macro]
pub fn declare_rule(input: TokenStream) -> TokenStream {
    decl::rule_decl(input.into()).into()
}

/// A record kind, `pub NAME = "SYS.name" { audience: …, horizon: Days(n) | Months(n), clause: "…" }`.
#[proc_macro]
pub fn declare_record(input: TokenStream) -> TokenStream {
    decl::record_decl(input.into()).into()
}

/// An audit family, `pub NAME = "SYS.name" { mode: Streaming | Incremental | Rolling { cycle_days: n }, clause:
/// "…" }`, whose mode is required.
#[proc_macro]
pub fn declare_family(input: TokenStream) -> TokenStream {
    decl::family_decl(input.into()).into()
}

/// A handler's declaration as a type: its sub-step, table, and the facts, intents and streams it reads, writes,
/// emits and draws, each of which its context grants and nothing else.
#[proc_macro]
pub fn declare_handler(input: TokenStream) -> TokenStream {
    decl::handler_decl(input.into()).into()
}
