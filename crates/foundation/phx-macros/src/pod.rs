use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Type};

/// The layout checks and the two marker impls; every refusal names what the type must change.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&input.generics, "a stored type has no generic parameters"));
    }
    if !repr_c(&input.attrs)? {
        return Err(syn::Error::new_spanned(name, "a stored type is `#[repr(C)]`, so its layout is declared"));
    }
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(name, "only a struct is stored"));
    };
    let types: Vec<&Type> = data.fields.iter().map(|f| &f.ty).collect();
    if let Some(float) = types.iter().find(|t| has_float(t)) {
        return Err(syn::Error::new_spanned(float, "stored state holds no floating-point number"));
    }
    Ok(quote! {
        const _: () = {
            const fn field_is_pod<F: ::phx_store::Pod>() {}
            const _: fn() = || { #( field_is_pod::<#types>(); )* };
            assert!(
                ::core::mem::size_of::<#name>() == 0 #( + ::core::mem::size_of::<#types>() )*,
                "padding: the fields' sizes do not sum to the type's size",
            );
        };
        #[expect(unsafe_code, reason = "the derive has checked the layout: repr(C), stored fields, no padding")]
        unsafe impl ::phx_store::Pod for #name {}
        impl ::phx_store::__seal::Sealed for #name {}
    })
}

fn repr_c(attrs: &[Attribute]) -> syn::Result<bool> {
    let mut found = false;
    for attr in attrs.iter().filter(|a| a.path().is_ident("repr")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("C") {
                found = true;
            } else if meta.input.peek(syn::token::Paren) {
                let _: TokenStream = meta.input.parse::<proc_macro2::Group>()?.stream();
            }
            Ok(())
        })?;
    }
    Ok(found)
}

fn has_float(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == "f32" || s.ident == "f64"),
        Type::Array(a) => has_float(&a.elem),
        Type::Tuple(t) => t.elems.iter().any(has_float),
        Type::Group(g) => has_float(&g.elem),
        Type::Paren(p) => has_float(&p.elem),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::expand;

    fn error(input: proc_macro2::TokenStream) -> String {
        expand(input).err().map(|e| e.to_string()).unwrap_or_default()
    }

    #[test]
    fn pod_requires_repr_c() {
        assert!(error(quote! { struct S { a: u64 } }).contains("repr(C)"));
        assert!(error(quote! { #[repr(align(64))] struct S { a: u64 } }).contains("repr(C)"));
        assert!(expand(quote! { #[repr(C, align(64))] struct S { a: u64 } }).is_ok());
    }

    #[test]
    fn pod_refuses_float() {
        assert!(error(quote! { #[repr(C)] struct S { a: u64, b: f64 } }).contains("floating-point"));
        assert!(error(quote! { #[repr(C)] struct S([f32; 4]); }).contains("floating-point"));
    }

    #[test]
    fn pod_refuses_padding() {
        let text = expand(quote! { #[repr(C)] struct S { a: u8, b: u64 } }).unwrap().to_string();
        let sizes = ":: core :: mem :: size_of :: < S > () == 0 + :: core :: mem :: size_of :: < u8 > () \
                     + :: core :: mem :: size_of :: < u64 > ()";
        assert!(text.contains(sizes), "{text}");
        assert!(text.contains("field_is_pod :: < u8 > () ; field_is_pod :: < u64 > ()"), "{text}");
        assert!(text.contains("unsafe impl :: phx_store :: Pod for S"), "{text}");
    }

    #[test]
    fn pod_refuses_enums_and_generics() {
        assert!(error(quote! { #[repr(C)] enum E { A } }).contains("only a struct"));
        assert!(error(quote! { #[repr(C)] struct S<T> { a: T } }).contains("generic"));
    }
}
