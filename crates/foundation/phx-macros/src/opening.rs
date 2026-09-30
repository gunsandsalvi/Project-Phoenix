use proc_macro2::TokenStream;
use quote::quote;

/// The function unchanged: it runs only while the world is assembled or opened, which the checks read. It takes no
/// arguments and marks nothing but a function.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        let error = syn::Error::new_spanned(args, "`#[opening]` takes no arguments").into_compile_error();
        return quote! { #error #item };
    }
    if syn::parse2::<syn::ItemFn>(item.clone()).is_ok() {
        return item;
    }
    let error = syn::Error::new_spanned(&item, "`#[opening]` marks only a function").into_compile_error();
    quote! { #error #item }
}
