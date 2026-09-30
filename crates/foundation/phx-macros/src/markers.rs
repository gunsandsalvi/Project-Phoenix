use proc_macro2::TokenStream;
use syn::{Data, DeriveInput, Expr, Field, Meta, Type};

/// The integer types a maintained aggregate may be: the machine's, and the number types that are integers inside.
pub const INTEGERS: &[&str] = &[
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "Amount", "Count", "Money", "Qty", "QtyRaw", "PriceRaw",
    "Fixed",
];

/// Nothing, once every field marked `#[maintained(writer = path)]` names its one writer and is an integer, so a rebuilt
/// sum equals the one maintained; anything else is refused where it stands.
pub fn maintained(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(&input.ident, "`Maintained` marks a struct's fields"));
    };
    for field in &data.fields {
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("maintained")) {
            check_writer(&attr.meta)?;
            check_integer(field)?;
        }
    }
    Ok(TokenStream::new())
}

fn check_writer(meta: &Meta) -> syn::Result<()> {
    let refused = || syn::Error::new_spanned(meta, "name the aggregate's one writer: `#[maintained(writer = path)]`");
    let Meta::List(list) = meta else { return Err(refused()) };
    let Meta::NameValue(nv) = list.parse_args::<Meta>().map_err(|_| refused())? else { return Err(refused()) };
    if nv.path.is_ident("writer") && matches!(nv.value, Expr::Path(_)) { Ok(()) } else { Err(refused()) }
}

fn check_integer(field: &Field) -> syn::Result<()> {
    let integer = match &field.ty {
        Type::Path(p) => p.path.segments.last().is_some_and(|s| INTEGERS.iter().any(|n| s.ident == n)),
        _ => false,
    };
    if integer {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            &field.ty,
            "a maintained aggregate is an integer: no float, no `i128`, no composite",
        ))
    }
}
