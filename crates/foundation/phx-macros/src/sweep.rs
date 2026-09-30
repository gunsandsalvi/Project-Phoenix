use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Lit, MetaNameValue, Token};

/// The function unchanged, where it names the store it walks and either the cycle its slices share the store over or
/// the reason it runs on; anything else is refused. The checks read the mark and admit the function's walks.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    match check(args, &item) {
        Ok(()) => item,
        Err(error) => {
            let error = error.into_compile_error();
            quote! { #error #item }
        }
    }
}

const USAGE: &str = "declare a sweep as `#[sweep(store = …, cycle = …)]` or `#[sweep(store = …, reason = \"…\")]`";

fn check(args: TokenStream, item: &TokenStream) -> syn::Result<()> {
    if syn::parse2::<syn::ItemFn>(item.clone()).is_err() {
        return Err(syn::Error::new_spanned(item, "`#[sweep]` marks only a function"));
    }
    let named = Punctuated::<MetaNameValue, Token![,]>::parse_terminated.parse2(args.clone())?;
    let (mut store, mut when) = (0, 0);
    for nv in &named {
        match nv.path.get_ident().map(ToString::to_string).as_deref() {
            Some("store") => store += 1,
            Some("cycle") => when += 1,
            Some("reason") => match &nv.value {
                Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) if !s.value().trim().is_empty() => when += 1,
                _ => return Err(syn::Error::new_spanned(&nv.value, "a sweep's reason is said in words")),
            },
            _ => return Err(syn::Error::new_spanned(&nv.path, USAGE)),
        }
    }
    if store == 1 && when == 1 { Ok(()) } else { Err(syn::Error::new_spanned(args, USAGE)) }
}
