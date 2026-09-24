use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Fields, Index, Type};

/// Whether a field is marked `#[saved(skip)]`: a derived index its owner rebuilds after a load, read back empty.
fn skipped(attrs: &[Attribute]) -> syn::Result<bool> {
    let mut skip = false;
    for attr in attrs.iter().filter(|a| a.path().is_ident("saved")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                Ok(())
            } else {
                Err(meta.error("a saved field is marked `skip` or not at all"))
            }
        })?;
    }
    Ok(skip)
}

/// One set of fields: how each is written from its binding and read back into it, and the bounds its types need.
struct Parts {
    save: Vec<TokenStream>,
    load: Vec<TokenStream>,
    bounds: Vec<TokenStream>,
}

fn parts(fields: &Fields, bind: &dyn Fn(usize) -> syn::Ident) -> syn::Result<Parts> {
    let mut p = Parts { save: Vec::new(), load: Vec::new(), bounds: Vec::new() };
    for (i, f) in fields.iter().enumerate() {
        let (b, ty): (syn::Ident, &Type) = (bind(i), &f.ty);
        if skipped(&f.attrs)? {
            p.load.push(quote! { let #b: #ty = ::core::default::Default::default(); });
            p.bounds.push(quote! { #ty: ::core::default::Default });
        } else {
            p.save.push(quote! { ::phx_store::Saved::save(#b, w); });
            p.load.push(quote! { let #b: #ty = ::phx_store::Saved::load(r)?; });
            p.bounds.push(quote! { #ty: ::phx_store::Saved });
        }
    }
    Ok(p)
}

/// The pattern that binds a set of fields, and the expression that builds them back.
fn shape(path: &TokenStream, fields: &Fields, bind: &dyn Fn(usize) -> syn::Ident) -> (TokenStream, TokenStream) {
    let binds: Vec<syn::Ident> = (0..fields.len()).map(bind).collect();
    match fields {
        Fields::Named(named) => {
            let idents: Vec<&syn::Ident> = named.named.iter().filter_map(|f| f.ident.as_ref()).collect();
            (quote! { #path { #( #idents: #binds ),* } }, quote! { #path { #( #idents: #binds ),* } })
        }
        Fields::Unnamed(_) => {
            let idx: Vec<Index> = (0..fields.len()).map(Index::from).collect();
            (quote! { #path { #( #idx: #binds ),* } }, quote! { #path( #( #binds ),* ) })
        }
        Fields::Unit => (quote! { #path }, quote! { #path }),
    }
}

/// `Saved` for a struct or an enum: its fields in order, an enum's variant first as its index.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let bind = |i: usize| format_ident!("__f{i}");
    let (save, load, bounds) = match &input.data {
        Data::Struct(s) => {
            let p = parts(&s.fields, &bind)?;
            let (pat, build) = shape(&quote! { #name }, &s.fields, &bind);
            let (save, load) = (&p.save, &p.load);
            (
                quote! { let #pat = self; #( #save )* },
                quote! { #( #load )* ::core::result::Result::Ok(#build) },
                p.bounds,
            )
        }
        Data::Enum(e) => {
            let mut saves = Vec::new();
            let mut loads = Vec::new();
            let mut bounds = Vec::new();
            for (i, v) in e.variants.iter().enumerate() {
                let Ok(index) = u32::try_from(i) else {
                    return Err(syn::Error::new_spanned(v, "an enum of more variants than a save counts"));
                };
                let vname = &v.ident;
                let p = parts(&v.fields, &bind)?;
                let (pat, build) = shape(&quote! { #name::#vname }, &v.fields, &bind);
                let (save, load) = (&p.save, &p.load);
                saves.push(quote! { #pat => { ::phx_store::Saved::save(&#index, w); #( #save )* } });
                loads.push(quote! { #index => { #( #load )* ::core::result::Result::Ok(#build) } });
                bounds.extend(p.bounds);
            }
            let unknown = format!("`{name}` has no variant of that index");
            (
                quote! { match self { #( #saves )* } },
                quote! {
                    let variant: u32 = ::phx_store::Saved::load(r)?;
                    match variant {
                        #( #loads )*
                        _ => ::core::result::Result::Err(::phx_store::LoadError::Invalid(#unknown.to_owned())),
                    }
                },
                bounds,
            )
        }
        Data::Union(u) => return Err(syn::Error::new_spanned(u.union_token, "a union is not saved")),
    };
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    let mut preds: Vec<TokenStream> =
        where_g.map(|w| w.predicates.iter().map(|p| quote! { #p }).collect()).unwrap_or_default();
    preds.extend(bounds);
    Ok(quote! {
        impl #impl_g ::phx_store::Saved for #name #ty_g where #( #preds ),* {
            fn save(&self, w: &mut ::phx_store::Writer<'_>) {
                #save
            }

            fn load(r: &mut ::phx_store::Reader<'_>) -> ::core::result::Result<Self, ::phx_store::LoadError> {
                #load
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::expand;

    #[test]
    fn saved_writes_fields_in_order_and_enums_by_index() {
        let text = expand(quote! { struct S { a: u64, #[saved(skip)] b: Vec<u8> } }).unwrap().to_string();
        assert!(text.contains("u64 : :: phx_store :: Saved"), "{text}");
        assert!(text.contains("Vec < u8 > : :: core :: default :: Default"), "{text}");
        let text = expand(quote! { enum E { A, B(u8) } }).unwrap().to_string();
        assert!(text.contains("0u32 =>") && text.contains("1u32 =>"), "{text}");
        assert!(expand(quote! { struct S { #[saved(keep)] a: u8 } }).is_err());
    }
}
