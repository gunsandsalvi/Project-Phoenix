use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, Lit, Meta};

/// The function unchanged, where it says why an absent value it reads is truly zero — a count of an entry that is not
/// there — in a reason that is not empty; anything else, or no reason, is refused.
pub fn expand(args: &TokenStream, item: TokenStream) -> TokenStream {
    match check(args, &item) {
        Ok(()) => item,
        Err(error) => {
            let error = error.into_compile_error();
            quote! { #error #item }
        }
    }
}

fn check(args: &TokenStream, item: &TokenStream) -> syn::Result<()> {
    if syn::parse2::<syn::ItemFn>(item.clone()).is_err() {
        return Err(syn::Error::new_spanned(item, "`#[absent_is_zero]` marks only a function"));
    }
    let refused = || syn::Error::new_spanned(args, "say why the absence is zero: `#[absent_is_zero(reason = \"…\")]`");
    let meta: Meta = syn::parse2(quote! { absent_is_zero(#args) }).map_err(|_| refused())?;
    let Meta::List(list) = meta else { return Err(refused()) };
    let named: Meta = list.parse_args().map_err(|_| refused())?;
    match named {
        Meta::NameValue(nv) if nv.path.is_ident("reason") => match nv.value {
            Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) if !s.value().trim().is_empty() => Ok(()),
            _ => Err(refused()),
        },
        _ => Err(refused()),
    }
}
