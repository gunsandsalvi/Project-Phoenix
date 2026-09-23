mod clause;
mod consts;
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
