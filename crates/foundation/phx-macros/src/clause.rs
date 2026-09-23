use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{LitStr, Token};

use crate::consts::SYSTEM_CODE_MAX_LETTERS;

/// The item unchanged, after every named clause is checked to be an identifier and not free text.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    match check(args) {
        Ok(()) => item,
        Err(error) => {
            let error = error.into_compile_error();
            quote! { #error #item }
        }
    }
}

fn check(args: TokenStream) -> syn::Result<()> {
    let ids = Punctuated::<LitStr, Token![,]>::parse_terminated.parse2(args.clone())?;
    if ids.is_empty() {
        return Err(syn::Error::new_spanned(args, "name at least one clause"));
    }
    for id in &ids {
        if !valid(&id.value()) {
            let message = "not a clause: expected `SYS.n`, `Law n`, `Nn` or `Nn.m`, or `Ln`";
            return Err(syn::Error::new_spanned(id, message));
        }
    }
    Ok(())
}

/// A system's clause (a code, a dot and a number), a law, a measurement with an optional part, or a transmission
/// chain.
pub fn valid(id: &str) -> bool {
    system_clause(id) || law(id) || measurement(id) || chain(id)
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn system_clause(id: &str) -> bool {
    id.split_once('.').is_some_and(|(code, n)| {
        (2..=SYSTEM_CODE_MAX_LETTERS).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_uppercase()) && digits(n)
    })
}

fn law(id: &str) -> bool {
    id.strip_prefix("Law ").is_some_and(|n| digits(n) && n.len() <= 2)
}

fn measurement(id: &str) -> bool {
    id.strip_prefix('N').is_some_and(|rest| match rest.split_once('.') {
        Some((head, tail)) => head.len() == 1 && digits(head) && digits(tail),
        None => rest.len() == 1 && digits(rest),
    })
}

fn chain(id: &str) -> bool {
    id.strip_prefix('L').is_some_and(|n| digits(n) && n.len() <= 2)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{expand, valid};

    #[test]
    fn clause_forms_are_identifiers() {
        for id in ["REP.8", "TIME.11", "Law 7", "Law 17", "N8", "N8.5", "L3", "L12"] {
            assert!(valid(id), "{id}");
        }
        for id in ["rep8", "REP-8", "REP.", "R.8", "ABCDE.1", "Law", "Law 123", "N81", "N8.", "L", "L123", "free text"]
        {
            assert!(!valid(id), "{id}");
        }
    }

    #[test]
    fn clause_emits_the_item_unchanged() {
        let item = quote! { fn f() {} };
        assert_eq!(expand(quote! { "REP.8", "Law 7" }, item.clone()).to_string(), item.to_string());
        assert!(expand(quote! {}, item.clone()).to_string().contains("compile_error"));
        assert!(expand(quote! { REP }, item).to_string().contains("compile_error"));
    }
}
