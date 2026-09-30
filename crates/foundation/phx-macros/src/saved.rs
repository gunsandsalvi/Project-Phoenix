use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Fields, Index, Type};

/// How a field is saved: whole, or left out as a derived index, with the function that rebuilds it after a load.
enum Kept {
    Saved,
    Skipped(Option<syn::Path>),
}

/// A field's `#[saved(...)]`: none, `skip`, or `skip, rebuild = path`; any other key is refused, and a rebuild names a
/// field left out.
fn kept(attrs: &[Attribute]) -> syn::Result<Kept> {
    let (mut skip, mut rebuild) = (false, None);
    for attr in attrs.iter().filter(|a| a.path().is_ident("saved")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                Ok(())
            } else if meta.path.is_ident("rebuild") {
                rebuild = Some(meta.value()?.parse::<syn::Path>()?);
                Ok(())
            } else {
                Err(meta.error("a saved field is marked `skip`, `skip, rebuild = path`, or not at all"))
            }
        })?;
        if rebuild.is_some() && !skip {
            return Err(syn::Error::new_spanned(
                attr,
                "`rebuild` names the rebuild of a field left out: `skip, rebuild = path`",
            ));
        }
    }
    Ok(if skip { Kept::Skipped(rebuild) } else { Kept::Saved })
}

/// One set of fields: how each is written from its binding and read back into it, and the bounds its types need.
struct Parts {
    save: Vec<TokenStream>,
    load: Vec<TokenStream>,
    bounds: Vec<TokenStream>,
    /// In field order, a saved field's own rebuilds and a skipped field's rebuild.
    rebuild: Vec<TokenStream>,
}

fn parts(fields: &Fields, bind: &dyn Fn(usize) -> syn::Ident) -> syn::Result<Parts> {
    let mut p = Parts { save: Vec::new(), load: Vec::new(), bounds: Vec::new(), rebuild: Vec::new() };
    for (i, f) in fields.iter().enumerate() {
        let (b, ty): (syn::Ident, &Type) = (bind(i), &f.ty);
        let member: syn::Member = match &f.ident {
            Some(ident) => syn::Member::Named(ident.clone()),
            None => syn::Member::Unnamed(Index::from(i)),
        };
        match kept(&f.attrs)? {
            Kept::Skipped(rebuild) => {
                p.load.push(quote! { let #b: #ty = ::core::default::Default::default(); });
                p.bounds.push(quote! { #ty: ::core::default::Default });
                if let Some(path) = rebuild {
                    p.rebuild.push(quote! { #path(self); });
                }
            }
            Kept::Saved => {
                p.save.push(quote! { ::phx_store::Saved::save(#b, w); });
                p.load.push(quote! { let #b: #ty = ::phx_store::Saved::load(r)?; });
                p.bounds.push(quote! { #ty: ::phx_store::Saved });
                p.rebuild.push(quote! { ::phx_store::Saved::rebuild_skipped(&mut self.#member); });
            }
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
    let (save, load, bounds, rebuild) = match &input.data {
        Data::Struct(s) => {
            let p = parts(&s.fields, &bind)?;
            let (pat, build) = shape(&quote! { #name }, &s.fields, &bind);
            let (save, load, rebuild) = (&p.save, &p.load, &p.rebuild);
            (
                quote! { let #pat = self; #( #save )* },
                quote! { #( #load )* ::core::result::Result::Ok(#build) },
                p.bounds,
                quote! {
                    fn rebuild_skipped(&mut self) {
                        #( #rebuild )*
                    }
                },
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
                TokenStream::new(),
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

            #rebuild
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
        assert!(
            expand(quote! { struct S { #[saved(rebuild = Self::f)] a: u8 } }).is_err(),
            "a rebuild of a saved field"
        );
        let text = expand(quote! { struct S { a: T, #[saved(skip, rebuild = Self::reindex)] b: Vec<u8> } }).unwrap();
        let text = text.to_string();
        assert!(text.contains("rebuild_skipped (& mut self . a)") && text.contains("Self :: reindex (self)"), "{text}");
    }
}
