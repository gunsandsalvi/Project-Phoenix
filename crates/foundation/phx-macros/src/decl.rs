use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprCall, ExprLit, Ident, Lit, LitStr, Token, Type, Visibility, braced};

use crate::clause;
use crate::consts::SYSTEM_CODE_MAX_LETTERS;

/// `key: value`, one field of a declaration.
struct Field {
    key: Ident,
    value: Expr,
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> syn::Result<Field> {
        let key = input.parse()?;
        input.parse::<Token![:]>()?;
        Ok(Field { key, value: input.parse()? })
    }
}

/// `#[doc] pub NAME = "id" { key: value, … }`, or with `on "kind"` in place of the braces; a decision point or rule
/// handle names its input and output types, `pub NAME: In => Out = "id" { … }`.
struct Decl {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    types: Option<(Type, Type)>,
    id: LitStr,
    fields: Vec<Field>,
    on: Option<LitStr>,
}

impl Parse for Decl {
    fn parse(input: ParseStream<'_>) -> syn::Result<Decl> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let name = input.parse()?;
        let types = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let from: Type = input.parse()?;
            input.parse::<Token![=>]>()?;
            Some((from, input.parse()?))
        } else {
            None
        };
        input.parse::<Token![=]>()?;
        let id = input.parse()?;
        if input.peek(syn::token::Brace) {
            let body;
            braced!(body in input);
            let fields = Punctuated::<Field, Token![,]>::parse_terminated(&body)?.into_iter().collect();
            return Ok(Decl { attrs, vis, name, types, id, fields, on: None });
        }
        if input.is_empty() {
            return Ok(Decl { attrs, vis, name, types, id, fields: Vec::new(), on: None });
        }
        let on: Ident = input.parse()?;
        if on != "on" {
            return Err(syn::Error::new_spanned(on, "expected `on \"kind\"` or a braced list of fields"));
        }
        Ok(Decl { attrs, vis, name, types, id, fields: Vec::new(), on: Some(input.parse()?) })
    }
}

impl Decl {
    /// The fields, each named once and all among `allowed`.
    fn fields(&self, allowed: &[&str]) -> syn::Result<Vec<(String, &Expr)>> {
        let mut out: Vec<(String, &Expr)> = Vec::new();
        for f in &self.fields {
            let key = f.key.to_string();
            if !allowed.contains(&key.as_str()) {
                return Err(syn::Error::new_spanned(&f.key, format!("no field `{key}`; the fields are {allowed:?}")));
            }
            if out.iter().any(|(k, _)| *k == key) {
                return Err(syn::Error::new_spanned(&f.key, format!("`{key}` given twice")));
            }
            out.push((key, &f.value));
        }
        Ok(out)
    }
}

fn get<'a>(fields: &[(String, &'a Expr)], key: &str) -> Option<&'a Expr> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
}

fn required<'a>(fields: &[(String, &'a Expr)], key: &str, span: Span) -> syn::Result<&'a Expr> {
    get(fields, key).ok_or_else(|| syn::Error::new(span, format!("a declaration needs `{key}`")))
}

fn string(e: &Expr) -> syn::Result<LitStr> {
    match e {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Ok(s.clone()),
        other => Err(syn::Error::new_spanned(other, "expected a string")),
    }
}

/// A bare variant name among `allowed`.
fn variant(e: &Expr, allowed: &[&str]) -> syn::Result<Ident> {
    if let Expr::Path(p) = e
        && let Some(ident) = p.path.get_ident()
        && allowed.contains(&ident.to_string().as_str())
    {
        return Ok(ident.clone());
    }
    Err(syn::Error::new_spanned(e, format!("expected one of {allowed:?}")))
}

/// `name("text")`, for the shapes of a SHAPE and the placeholder of a writer.
fn call(e: &Expr) -> Option<(String, LitStr)> {
    let Expr::Call(ExprCall { func, args, .. }) = e else { return None };
    let Expr::Path(p) = func.as_ref() else { return None };
    let name = p.path.get_ident()?.to_string();
    match args.iter().collect::<Vec<_>>().as_slice() {
        [arg] => string(arg).ok().map(|s| (name, s)),
        _ => None,
    }
}

fn system_code(code: &str) -> bool {
    (2..=SYSTEM_CODE_MAX_LETTERS).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_uppercase())
}

fn snake(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// `<SYS>.<snake_case>`, returning the code.
fn qualified(id: &LitStr) -> syn::Result<String> {
    let value = id.value();
    match value.split_once('.') {
        Some((code, name)) if system_code(code) && snake(name) => Ok(code.to_owned()),
        _ => Err(syn::Error::new_spanned(id, "expected `<SYS>.<snake_case>`")),
    }
}

fn clause_of(fields: &[(String, &Expr)], span: Span) -> syn::Result<LitStr> {
    let c = string(required(fields, "clause", span)?)?;
    if !clause::valid(&c.value()) {
        return Err(syn::Error::new_spanned(&c, "not a clause"));
    }
    Ok(c)
}

fn missing(value: Option<TokenStream>) -> TokenStream {
    value.map_or_else(
        || quote! { ::phx_core::register::Missing::Absent },
        |v| quote! { ::phx_core::register::Missing::Present(#v) },
    )
}

fn expand_with(input: TokenStream, body: fn(&Decl) -> syn::Result<(TokenStream, TokenStream)>) -> TokenStream {
    match syn::parse2::<Decl>(input).and_then(|d| body(&d).map(|b| (d, b))) {
        Ok((d, (ty, value))) => {
            let (attrs, vis, name) = (&d.attrs, &d.vis, &d.name);
            quote! { #(#attrs)* #vis const #name: #ty = #value; }
        }
        Err(e) => e.into_compile_error(),
    }
}

fn expand_items(input: TokenStream, body: fn(&Decl) -> syn::Result<TokenStream>) -> TokenStream {
    syn::parse2::<Decl>(input).and_then(|d| body(&d)).unwrap_or_else(syn::Error::into_compile_error)
}

const PRIM_KINDS: [&str; 6] = ["Technology", "Preference", "Policy", "Endowment", "Resolution", "Shape"];
const PERIODS: [&str; 5] = ["Day", "Week", "Month", "Quarter", "Year"];
const VALUE_TYPES: [&str; 17] = [
    "Fixed",
    "Rate",
    "Money",
    "Qty",
    "Count",
    "Date",
    "Table1",
    "Table2",
    "Distribution",
    "PointTable",
    "Calendar",
    "LegalForms",
    "Products",
    "NewsRule",
    "CarryingBases",
    "Profile",
    "Partition",
];

/// A variant written bare, `Rate`, or with its fields, `Fixed { exp: 4 }`, as a path under `ty`.
fn variant_of(e: &Expr, ty: &TokenStream, allowed: &[&str]) -> syn::Result<TokenStream> {
    match e {
        Expr::Path(p) if p.path.get_ident().is_some_and(|h| allowed.contains(&h.to_string().as_str())) => {
            let h = p.path.get_ident();
            Ok(quote! { #ty::#h })
        }
        Expr::Struct(s) if s.path.get_ident().is_some_and(|h| allowed.contains(&h.to_string().as_str())) => {
            let (h, fields) = (s.path.get_ident(), s.fields.iter());
            Ok(quote! { #ty::#h { #(#fields),* } })
        }
        _ => Err(syn::Error::new_spanned(e, format!("expected one of {allowed:?}"))),
    }
}

/// The value type, `Rate` or `Fixed { exp: 4 }`, as the register's type.
fn value_type(e: &Expr) -> syn::Result<TokenStream> {
    variant_of(e, &quote! { ::phx_core::register::values::ValueType }, &VALUE_TYPES)
}

fn prim(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let fields = d.fields(&["kind", "unit", "period", "decided_by", "value", "clause", "shape", "scope"])?;
    qualified(&d.id)?;
    let kind = variant(required(&fields, "kind", span)?, &PRIM_KINDS)?;
    let policy = kind == "Policy";
    let shape_kind = kind == "Shape";
    let decided_by = get(&fields, "decided_by").map(string).transpose()?;
    if policy != decided_by.is_some() {
        return Err(syn::Error::new(span, "`decided_by` is for policies, and every policy has one"));
    }
    let shape = match get(&fields, "shape") {
        Some(e) => match call(e) {
            Some((form, text)) if form == "placeholder" && system_code(&text.value()) => {
                Some(quote! { ::phx_core::register::ShapeInfo::Placeholder { retired_by: #text } })
            }
            Some((form, text)) if form == "standing" && !text.value().trim().is_empty() => {
                Some(quote! { ::phx_core::register::ShapeInfo::Standing { reason: #text } })
            }
            _ => return Err(syn::Error::new_spanned(e, "expected `placeholder(\"SYS\")` or `standing(\"reason\")`")),
        },
        None => None,
    };
    if shape_kind != shape.is_some() {
        return Err(syn::Error::new(span, "`shape` is for SHAPEs, and every SHAPE has one"));
    }
    let unit = get(&fields, "unit").map(string).transpose()?.map(|u| quote! { #u });
    let period = get(&fields, "period")
        .map(|p| variant(p, &PERIODS))
        .transpose()?
        .map(|p| quote! { ::phx_core::register::PrimPeriod::#p });
    let scope = variant(required(&fields, "scope", span)?, &["Shared", "PerCountry"])?;
    let value = value_type(required(&fields, "value", span)?)?;
    let clause = clause_of(&fields, span)?;
    let (id, unit, period) = (&d.id, missing(unit), missing(period));
    let decided_by = missing(decided_by.map(|r| quote! { ::phx_core::register::RoleId(#r) }));
    let shape = missing(shape);
    Ok((
        quote! { ::phx_core::register::PrimDecl },
        quote! {
            ::phx_core::register::PrimDecl {
                id: #id,
                kind: ::phx_core::register::PrimKind::#kind,
                unit: #unit,
                period: #period,
                decided_by: #decided_by,
                value: #value,
                clause: #clause,
                shape: #shape,
                scope: ::phx_core::register::Scope::#scope,
            }
        },
    ))
}

fn strings(e: &Expr) -> syn::Result<Vec<LitStr>> {
    match e {
        Expr::Array(a) => a.elems.iter().map(string).collect(),
        other => Err(syn::Error::new_spanned(other, "expected a list of strings")),
    }
}

fn audience(e: &Expr) -> syn::Result<TokenStream> {
    if let Ok(v) = variant(e, &["Party", "Public"]) {
        return Ok(quote! { ::phx_core::facts::Audience::#v });
    }
    if let Some((form, kind)) = call(e)
        && form == "Authority"
        && snake(&kind.value())
    {
        return Ok(quote! { ::phx_core::facts::Audience::Authority(#kind) });
    }
    if let Expr::Call(ExprCall { func, args, .. }) = e
        && matches!(func.as_ref(), Expr::Path(p) if p.path.is_ident("PublicAfter"))
        && let [Expr::Call(ExprCall { func: lag, args: n, .. })] = args.iter().collect::<Vec<_>>().as_slice()
        && let Expr::Path(lag) = lag.as_ref()
        && let Some(lag) = lag.path.get_ident()
        && (lag == "Days" || lag == "Months")
        && let [Expr::Lit(ExprLit { lit: Lit::Int(n), .. })] = n.iter().collect::<Vec<_>>().as_slice()
        && n.base10_parse::<u16>().is_ok_and(|v| v > 0)
    {
        return Ok(quote! { ::phx_core::facts::Audience::PublicAfter(::phx_core::facts::Lag::#lag(#n)) });
    }
    Err(syn::Error::new_spanned(
        e,
        "expected `Party`, `Authority(\"kind\")`, `Public` or `PublicAfter(Days(n))` / `PublicAfter(Months(n))`",
    ))
}

const FACT_TYPES: [&str; 9] = ["Flag", "Count", "Money", "Qty", "Rate", "Fixed", "Day", "Party", "Type"];

fn fact(d: &Decl) -> syn::Result<TokenStream> {
    let span = d.name.span();
    let fields = d.fields(&["value", "unit", "kinds", "writer", "audience", "repr", "clause"])?;
    let code = qualified(&d.id)?;
    let value = variant_of(required(&fields, "value", span)?, &quote! { ::phx_core::facts::FactType }, &FACT_TYPES)?;
    let kinds = strings(required(&fields, "kinds", span)?)?;
    if kinds.is_empty() || kinds.iter().any(|k| !snake(&k.value())) {
        return Err(syn::Error::new(span, "a fact is of one or more kinds, each in snake_case"));
    }
    let writer_expr = required(&fields, "writer", span)?;
    let (writer, writer_code) = match (string(writer_expr), call(writer_expr)) {
        (Ok(s), _) => (quote! { ::phx_core::facts::Writer::System(#s) }, s.value()),
        (_, Some((form, s))) if form == "placeholder" => {
            (quote! { ::phx_core::facts::Writer::Placeholder { retired_by: #s } }, s.value())
        }
        _ => return Err(syn::Error::new_spanned(writer_expr, "expected `\"SYS\"` or `placeholder(\"SYS\")`")),
    };
    if writer_code != code {
        return Err(syn::Error::new_spanned(writer_expr, format!("the fact is {code}'s, so {code} writes it")));
    }
    let audience = audience(required(&fields, "audience", span)?)?;
    let repr = variant(required(&fields, "repr", span)?, &["Key", "Position", "Profile", "Individual"])?;
    let unit = missing(get(&fields, "unit").map(string).transpose()?.map(|u| quote! { #u }));
    let clause = clause_of(&fields, span)?;
    let id = &d.id;
    let (attrs, vis, name) = (&d.attrs, &d.vis, &d.name);
    Ok(quote! {
        #(#attrs)*
        #[derive(Clone, Copy, Debug)]
        #vis struct #name;
        impl ::phx_core::facts::FactDef for #name {
            const ITEM: ::phx_core::facts::ItemDecl = ::phx_core::facts::ItemDecl {
                name: #id,
                kind: ::phx_core::facts::ItemKind::Fact(::phx_core::facts::FactDecl {
                    value: #value,
                    unit: #unit,
                    kinds: &[#(#kinds),*],
                    audience: #audience,
                    repr: ::phx_core::facts::ReprClass::#repr,
                }),
                writer: #writer,
                clause: #clause,
            };
        }
    })
}

fn kind(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let fields = d.fields(&["legal_form", "table", "clause"])?;
    if !snake(&d.id.value()) {
        return Err(syn::Error::new_spanned(&d.id, "a kind is named in snake_case"));
    }
    let form = string(required(&fields, "legal_form", span)?)?;
    let table = variant(required(&fields, "table", span)?, &["Individuals", "Cells"])?;
    let clause = clause_of(&fields, span)?;
    let id = &d.id;
    Ok((
        quote! { ::phx_core::kinds::KindDecl },
        quote! {
            ::phx_core::kinds::KindDecl {
                name: #id,
                legal_form: #form,
                table: ::phx_core::kinds::KindTableRef::#table,
                clause: #clause,
            }
        },
    ))
}

fn facet(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    qualified(&d.id)?;
    let Some(kind) = &d.on else {
        return Err(syn::Error::new(d.name.span(), "a facet is `NAME = \"SYS.fact\" on \"kind\"`"));
    };
    if !snake(&kind.value()) {
        return Err(syn::Error::new_spanned(kind, "a kind is named in snake_case"));
    }
    let id = &d.id;
    Ok((
        quote! { ::phx_core::kind_tables::FacetDecl },
        quote! { ::phx_core::kind_tables::FacetDecl { fact: #id, kind: #kind } },
    ))
}

pub fn prim_decl(input: TokenStream) -> TokenStream {
    expand_with(input, prim)
}

pub fn fact_decl(input: TokenStream) -> TokenStream {
    expand_items(input, fact)
}

pub fn kind_decl(input: TokenStream) -> TokenStream {
    expand_with(input, kind)
}

pub fn facet_decl(input: TokenStream) -> TokenStream {
    expand_with(input, facet)
}

const PURPOSES: [&str; 21] = [
    "Mortality",
    "Illness",
    "Birthday",
    "Conception",
    "Accident",
    "Damage",
    "ThirdPartyHarm",
    "Catastrophe",
    "EquipmentFailure",
    "Discovery",
    "Meeting",
    "Weather",
    "TypeAtBirth",
    "SchedulePhase",
    "Occasion",
    "Taste",
    "Pairing",
    "Sample",
    "Opening",
    "Observer",
    "Lot",
];

const SUB_STEPS: [&str; 46] = [
    "S1a", "S1b", "S1c", "S2a", "S2b", "S2c", "S2d", "S2e", "S2f", "S3a", "S3b", "S3c", "S3d", "S3e", "S4a", "S4b",
    "S5a", "S5b", "S5c", "S5d", "S6a", "S6b", "S6c", "S6d", "S7a", "S7b", "S7c", "S7d", "S7e", "S8a", "S8b", "S8c",
    "S8d", "S8e", "S8f", "S9a", "S9b", "S9c", "S9d", "S9e", "S10a", "S10b", "S10c", "S10d", "S10e", "S10f",
];

fn boolean(e: &Expr) -> syn::Result<bool> {
    match e {
        Expr::Lit(ExprLit { lit: Lit::Bool(b), .. }) => Ok(b.value),
        other => Err(syn::Error::new_spanned(other, "expected `true` or `false`")),
    }
}

fn flag(fields: &[(String, &Expr)], key: &str, span: Span) -> syn::Result<bool> {
    boolean(required(fields, key, span)?)
}

/// A list of type paths, as a handler's reads.
fn types(e: Option<&Expr>) -> syn::Result<Vec<syn::Path>> {
    match e {
        None => Ok(Vec::new()),
        Some(Expr::Array(a)) => a
            .elems
            .iter()
            .map(|t| match t {
                Expr::Path(p) => Ok(p.path.clone()),
                other => Err(syn::Error::new_spanned(other, "expected a type")),
            })
            .collect(),
        Some(other) => Err(syn::Error::new_spanned(other, "expected a list of types")),
    }
}

fn stream(d: &Decl) -> syn::Result<TokenStream> {
    let span = d.name.span();
    let fields = d.fields(&["purpose", "keyed", "clause"])?;
    qualified(&d.id)?;
    let purpose = variant(required(&fields, "purpose", span)?, &PURPOSES)?;
    let keyed = flag(&fields, "keyed", span)?;
    let clause = clause_of(&fields, span)?;
    let (attrs, vis, name, id) = (&d.attrs, &d.vis, &d.name, &d.id);
    Ok(quote! {
        #(#attrs)*
        #[derive(Clone, Copy, Debug)]
        #vis struct #name;
        impl ::phx_core::streams::StreamDef for #name {
            const DECL: ::phx_core::streams::StreamDecl = ::phx_core::streams::StreamDecl {
                name: #id,
                purpose: ::phx_core::streams::Purpose::#purpose,
                keyed: #keyed,
                clause: #clause,
            };
        }
    })
}

/// `[("kind", "SYS", S5c), …]` as a message's answerers.
fn answering(e: &Expr) -> syn::Result<Vec<TokenStream>> {
    let Expr::Array(a) = e else { return Err(syn::Error::new_spanned(e, "expected a list of answerers")) };
    a.elems
        .iter()
        .map(|t| {
            let Expr::Tuple(t) = t else {
                return Err(syn::Error::new_spanned(t, "expected `(\"kind\", \"SYS\", S5c)`"));
            };
            match t.elems.iter().collect::<Vec<_>>().as_slice() {
                [kind, system, step] => {
                    let (kind, system) = (string(kind)?, string(system)?);
                    if !snake(&kind.value()) || !system_code(&system.value()) {
                        return Err(syn::Error::new_spanned(t, "expected a kind in snake_case and a system code"));
                    }
                    let step = variant(step, &SUB_STEPS)?;
                    Ok(quote! {
                        ::phx_core::messages::Answering {
                            addressee: #kind,
                            system: #system,
                            substep: ::phx_core::substep::SubStep::#step,
                        }
                    })
                }
                _ => Err(syn::Error::new_spanned(t, "expected `(\"kind\", \"SYS\", S5c)`")),
            }
        })
        .collect()
}

fn message(d: &Decl) -> syn::Result<TokenStream> {
    let span = d.name.span();
    let fields =
        d.fields(&["lives_across_days", "reaches", "answering", "acceptance", "opens_commitment", "pins", "clause"])?;
    qualified(&d.id)?;
    let lives = flag(&fields, "lives_across_days", span)?;
    let reaches = strings(required(&fields, "reaches", span)?)?;
    let answers = answering(required(&fields, "answering", span)?)?;
    let acceptance = match get(&fields, "acceptance") {
        Some(Expr::Tuple(t)) => match t.elems.iter().map(string).collect::<syn::Result<Vec<_>>>()?.as_slice() {
            [system, handler] => Some(quote! { (#system, #handler) }),
            _ => return Err(syn::Error::new_spanned(t, "expected `(\"SYS\", \"handler\")`")),
        },
        Some(other) => return Err(syn::Error::new_spanned(other, "expected `(\"SYS\", \"handler\")`")),
        None => None,
    };
    let (opens, pins) = (flag(&fields, "opens_commitment", span)?, flag(&fields, "pins", span)?);
    let clause = clause_of(&fields, span)?;
    let acceptance = missing(acceptance);
    let (attrs, vis, name, id) = (&d.attrs, &d.vis, &d.name, &d.id);
    Ok(quote! {
        #(#attrs)*
        #[derive(Clone, Copy, Debug)]
        #vis struct #name;
        impl ::phx_core::messages::MessageDef for #name {
            const DECL: ::phx_core::messages::MessageKindDecl = ::phx_core::messages::MessageKindDecl {
                name: #id,
                lives_across_days: #lives,
                reaches: &[#(#reaches),*],
                answering: &[#(#answers),*],
                acceptance: #acceptance,
                opens_commitment: #opens,
                pins: #pins,
                clause: #clause,
            };
        }
    })
}

fn acts_on(e: &Expr) -> syn::Result<TokenStream> {
    if let Ok(v) = variant(e, &["Tile", "Region", "Country"]) {
        return Ok(quote! { ::phx_core::hazards::ActsOn::#v });
    }
    match call(e) {
        Some((form, kind)) if form == "Party" && snake(&kind.value()) => {
            Ok(quote! { ::phx_core::hazards::ActsOn::Party { kind: #kind } })
        }
        Some((form, kind)) if form == "Persons" && snake(&kind.value()) => {
            Ok(quote! { ::phx_core::hazards::ActsOn::Persons { kind: #kind } })
        }
        Some((form, class)) if form == "Holding" && !class.value().is_empty() => {
            Ok(quote! { ::phx_core::hazards::ActsOn::Holding { class: #class } })
        }
        _ => {
            if let Expr::Call(ExprCall { func, args, .. }) = e
                && matches!(func.as_ref(), Expr::Path(p) if p.path.is_ident("Role"))
                && let [kind, role] = args.iter().collect::<Vec<_>>().as_slice()
            {
                let (kind, role) = (string(kind)?, string(role)?);
                return Ok(quote! { ::phx_core::hazards::ActsOn::Role { kind: #kind, role: #role } });
            }
            Err(syn::Error::new_spanned(
                e,
                "expected `Tile`, `Region`, `Country`, `Party(\"kind\")`, `Persons(\"kind\")`, `Holding(\"class\")` or \
                 `Role(\"kind\", \"role\")`",
            ))
        }
    }
}

/// The dates a rate can change without a visit: `YearStart`, `MonthStart`, `Policy("…")` or `Review("…")`.
fn rate_changes(e: &Expr) -> syn::Result<Vec<TokenStream>> {
    let Expr::Array(a) = e else {
        return Err(syn::Error::new_spanned(e, "expected a list of the dates the rate can change"));
    };
    a.elems
        .iter()
        .map(|c| match c {
            Expr::Path(p) if p.path.is_ident("YearStart") => Ok(quote! { ::phx_core::hazards::RateChange::YearStart }),
            Expr::Path(p) if p.path.is_ident("MonthStart") => {
                Ok(quote! { ::phx_core::hazards::RateChange::MonthStart })
            }
            Expr::Path(p) if p.path.is_ident("Birthday") => Ok(quote! { ::phx_core::hazards::RateChange::Birthday }),
            Expr::Call(ExprCall { func, args, .. }) => {
                let name = match args.iter().collect::<Vec<_>>().as_slice() {
                    [one] => string(one)?,
                    _ => return Err(syn::Error::new_spanned(c, "one name")),
                };
                match func.as_ref() {
                    Expr::Path(p) if p.path.is_ident("Policy") => {
                        Ok(quote! { ::phx_core::hazards::RateChange::Policy(#name) })
                    }
                    Expr::Path(p) if p.path.is_ident("Review") => {
                        Ok(quote! { ::phx_core::hazards::RateChange::Review(#name) })
                    }
                    _ => Err(syn::Error::new_spanned(c, "expected `Policy(\"…\")` or `Review(\"…\")`")),
                }
            }
            _ => Err(syn::Error::new_spanned(
                c,
                "expected `YearStart`, `MonthStart`, `Birthday`, `Policy(\"…\")` or `Review(\"…\")`",
            )),
        })
        .collect()
}

fn hazard(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let fields =
        d.fields(&["acts_on", "rate", "axes", "changes", "outcome", "scheme", "stream", "clause", "source"])?;
    qualified(&d.id)?;
    let acts_on = acts_on(required(&fields, "acts_on", span)?)?;
    let rate = string(required(&fields, "rate", span)?)?;
    qualified(&rate)?;
    let axes = strings(required(&fields, "axes", span)?)?;
    let changes = rate_changes(required(&fields, "changes", span)?)?;
    let outcome = string(required(&fields, "outcome", span)?)?;
    let scheme = match variant(required(&fields, "scheme", span)?, &["Scheduled", "Daily"])? {
        s if s == "Scheduled" => quote! { ::phx_core::hazards::DrawScheme::Scheduled },
        _ => quote! { ::phx_core::hazards::DrawScheme::Daily },
    };
    let stream = string(required(&fields, "stream", span)?)?;
    qualified(&stream)?;
    let source = string(required(&fields, "source", span)?)?;
    if source.value().trim().is_empty() || outcome.value().trim().is_empty() {
        return Err(syn::Error::new(span, "a hazard names its outcome and its source"));
    }
    let clause = clause_of(&fields, span)?;
    let id = &d.id;
    Ok((
        quote! { ::phx_core::hazards::HazardDecl },
        quote! {
            ::phx_core::hazards::HazardDecl {
                name: #id,
                acts_on: #acts_on,
                rate: ::phx_core::hazards::RateFn { table: #rate, axes: &[#(#axes),*], changes: &[#(#changes),*] },
                outcome: #outcome,
                scheme: #scheme,
                stream: #stream,
                clause: #clause,
                source: #source,
            }
        },
    ))
}

const WAKES: [&str; 5] = ["Message", "Surprise", "PlayerIntent", "KinkDay", "EventConcerning"];

fn decision(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let Some((input, output)) = &d.types else {
        return Err(syn::Error::new(span, "a decision point is `NAME: Input => Output = \"SYS.name\" { … }`"));
    };
    let fields = d.fields(&["rule", "schedule", "wakes", "runs_on_non_business", "clause"])?;
    let system = qualified(&d.id)?;
    let rule = required(&fields, "rule", span)?;
    let schedule = get(&fields, "schedule").map(string).transpose()?;
    let wakes = match get(&fields, "wakes") {
        Some(Expr::Array(a)) => a.elems.iter().map(|w| variant(w, &WAKES)).collect::<syn::Result<Vec<_>>>()?,
        Some(other) => return Err(syn::Error::new_spanned(other, "expected a list of wakes")),
        None => Vec::new(),
    };
    if schedule.is_none() && wakes.is_empty() {
        return Err(syn::Error::new(span, "a decision point has a schedule or wakes"));
    }
    let non_business = flag(&fields, "runs_on_non_business", span)?;
    let clause = clause_of(&fields, span)?;
    let (id, schedule) = (&d.id, missing(schedule.map(|s| quote! { #s })));
    Ok((
        quote! { ::phx_core::decisions::DecisionPointDecl<#input, #output> },
        quote! {
            ::phx_core::decisions::DecisionPointDecl {
                name: #id,
                system: #system,
                rule: #rule,
                schedule: #schedule,
                wakes: &[#(::phx_core::schedule::WakeKind::#wakes),*],
                runs_on_non_business: #non_business,
                clause: #clause,
            }
        },
    ))
}

fn rule_sig(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let Some((input, output)) = &d.types else {
        return Err(syn::Error::new(d.name.span(), "a rule handle is `NAME: Input => Output = \"SYS.name\"`"));
    };
    if d.on.is_some() || !d.fields.is_empty() {
        return Err(syn::Error::new(d.name.span(), "a rule handle takes no fields"));
    }
    let system = qualified(&d.id)?;
    let id = &d.id;
    Ok((
        quote! { ::phx_core::rules::RuleSig<#input, #output> },
        quote! { ::phx_core::rules::RuleSig::new(#id, #system) },
    ))
}

fn lag(e: &Expr) -> syn::Result<TokenStream> {
    if let Expr::Call(ExprCall { func, args, .. }) = e
        && let Expr::Path(lag) = func.as_ref()
        && let Some(lag) = lag.path.get_ident()
        && (lag == "Days" || lag == "Months")
        && let [Expr::Lit(ExprLit { lit: Lit::Int(n), .. })] = args.iter().collect::<Vec<_>>().as_slice()
        && n.base10_parse::<u16>().is_ok_and(|v| v > 0)
    {
        return Ok(quote! { ::phx_core::facts::Lag::#lag(#n) });
    }
    Err(syn::Error::new_spanned(e, "expected `Days(n)` or `Months(n)`, n above zero"))
}

fn record(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let fields = d.fields(&["audience", "horizon", "clause"])?;
    let writer = qualified(&d.id)?;
    let audience = audience(required(&fields, "audience", span)?)?;
    let horizon = lag(required(&fields, "horizon", span)?)?;
    let clause = clause_of(&fields, span)?;
    let id = &d.id;
    Ok((
        quote! { ::phx_core::records::RecordKindDecl },
        quote! {
            ::phx_core::records::RecordKindDecl {
                name: #id, audience: #audience, horizon: #horizon, writer: #writer, clause: #clause,
            }
        },
    ))
}

fn family(d: &Decl) -> syn::Result<(TokenStream, TokenStream)> {
    let span = d.name.span();
    let fields = d.fields(&["mode", "clause"])?;
    let owner = qualified(&d.id)?;
    let mode = required(&fields, "mode", span)?;
    let mode = match mode {
        Expr::Struct(s) if s.path.is_ident("Rolling") => {
            let days = s.fields.iter().find(|f| matches!(&f.member, syn::Member::Named(m) if m == "cycle_days"));
            match days.map(|f| &f.expr) {
                Some(Expr::Lit(ExprLit { lit: Lit::Int(n), .. })) if n.base10_parse::<u16>().is_ok_and(|v| v > 0) => {
                    quote! { ::phx_core::family::FamilyMode::Rolling { cycle_days: #n } }
                }
                _ => return Err(syn::Error::new_spanned(s, "expected `Rolling { cycle_days: n }`, n above zero")),
            }
        }
        other => {
            let v = variant(other, &["Streaming", "Incremental"])?;
            quote! { ::phx_core::family::FamilyMode::#v }
        }
    };
    let clause = clause_of(&fields, span)?;
    let id = &d.id;
    Ok((
        quote! { ::phx_core::family::FamilyDecl },
        quote! { ::phx_core::family::FamilyDecl { name: #id, owner: #owner, clause: #clause, mode: #mode } },
    ))
}

fn handler(d: &Decl) -> syn::Result<TokenStream> {
    let span = d.name.span();
    let fields = d.fields(&["substep", "table", "reads", "writes", "intents", "streams", "clause", "body"])?;
    qualified(&d.id)?;
    let substep = variant(required(&fields, "substep", span)?, &SUB_STEPS)?;
    let table = string(required(&fields, "table", span)?)?;
    let reads = types(get(&fields, "reads"))?;
    let writes = types(get(&fields, "writes"))?;
    let intents = types(get(&fields, "intents"))?;
    let streams = types(get(&fields, "streams"))?;
    let clause = clause_of(&fields, span)?;
    let (attrs, vis, name, id) = (&d.attrs, &d.vis, &d.name, &d.id);
    let run = if let Some(body) = get(&fields, "body") {
        quote! {
            ::phx_num::Missing::Present({
                fn run(
                    parts: ::phx_core::handler::CtxParts<'_, dyn ::phx_core::handler::FactStore>,
                    rows: ::core::ops::Range<u32>,
                ) {
                    let mut ctx = ::phx_core::handler::Ctx::<#name, dyn ::phx_core::handler::FactStore>::new(parts);
                    for row in rows {
                        #body(&mut ctx, ::phx_id::Slot::new(row));
                    }
                }
                run
            })
        }
    } else {
        quote! { ::phx_num::Missing::Absent }
    };
    let read_or_written: Vec<&syn::Path> = reads.iter().chain(writes.iter().filter(|w| !reads.contains(w))).collect();
    Ok(quote! {
        #(#attrs)*
        #[derive(Clone, Copy, Debug)]
        #vis struct #name;
        impl ::phx_core::handler::HandlerDecl for #name {
            const NAME: &'static str = #id;
            const SUBSTEP: ::phx_core::substep::SubStep = ::phx_core::substep::SubStep::#substep;
            const TABLE: &'static str = #table;
            const READS: &'static [&'static str] = &[#(<#reads as ::phx_core::facts::FactDef>::ITEM.name),*];
            const WRITES: &'static [&'static str] = &[#(<#writes as ::phx_core::facts::FactDef>::ITEM.name),*];
            const INTENTS: &'static [&'static str] = &[#(<#intents as ::phx_core::handler::IntentDef>::NAME),*];
            const STREAMS: &'static [&'static str] = &[#(<#streams as ::phx_core::streams::StreamDef>::DECL.name),*];
            const CLAUSE: &'static str = #clause;
            const RUN: ::phx_num::Missing<::phx_core::handler::RunChunk> = #run;
        }
        #(impl ::phx_core::handler::Reads<#read_or_written> for #name {})*
        #(impl ::phx_core::handler::Writes<#writes> for #name {})*
        #(impl ::phx_core::handler::Emits<#intents> for #name {})*
        #(impl ::phx_core::handler::DrawsFrom<#streams> for #name {})*
    })
}

pub fn stream_decl(input: TokenStream) -> TokenStream {
    expand_items(input, stream)
}

pub fn message_decl(input: TokenStream) -> TokenStream {
    expand_items(input, message)
}

pub fn hazard_decl(input: TokenStream) -> TokenStream {
    expand_with(input, hazard)
}

pub fn decision_decl(input: TokenStream) -> TokenStream {
    expand_with(input, decision)
}

pub fn rule_decl(input: TokenStream) -> TokenStream {
    expand_with(input, rule_sig)
}

pub fn record_decl(input: TokenStream) -> TokenStream {
    expand_with(input, record)
}

pub fn family_decl(input: TokenStream) -> TokenStream {
    expand_with(input, family)
}

pub fn handler_decl(input: TokenStream) -> TokenStream {
    expand_items(input, handler)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{fact_decl, kind_decl, prim_decl};

    fn refused(out: &proc_macro2::TokenStream) -> bool {
        out.to_string().contains("compile_error")
    }

    #[test]
    fn declarations_are_checked_as_they_are_written() {
        let good = quote! { pub RATE = "CB.policy_rate" { kind: Policy, decided_by: "central_bank", period: Year,
        unit: "per year", value: Rate, clause: "CB.1", scope: PerCountry } };
        assert!(!refused(&prim_decl(good)));
        assert!(
            refused(&prim_decl(quote! { pub RATE = "CB.policy_rate" { kind: Policy, period: Year, value: Rate,
            clause: "CB.1", scope: PerCountry } })),
            "a policy without decided_by"
        );
        assert!(
            refused(&prim_decl(quote! { pub X = "cb.rate" { kind: Technology, value: Rate, clause: "CB.1",
            scope: Shared } })),
            "not <SYS>.<snake_case>"
        );
        assert!(
            refused(&prim_decl(quote! { pub X = "CB.rate" { kind: Shape, value: Rate, clause: "CB.1",
            scope: Shared } })),
            "a SHAPE without its shape"
        );
        assert!(
            refused(&prim_decl(quote! { pub X = "CB.rate" { kind: Technology, value: Money, clause: "CB.1",
            scope: Shared, colour: "red" } })),
            "an unknown field"
        );
        assert!(!refused(&fact_decl(quote! { pub S = "LAB.status" { value: Flag, kinds: ["household"],
        writer: "LAB", audience: PublicAfter(Months(3)), repr: Key, clause: "LAB.1" } })));
        assert!(
            refused(&fact_decl(quote! { pub S = "LAB.status" { value: Flag, kinds: ["household"],
            writer: "HH", audience: Party, repr: Key, clause: "LAB.1" } })),
            "another system writes LAB's fact"
        );
        assert!(!refused(&kind_decl(quote! { pub BANK = "bank" { legal_form: "bank", table: Individuals,
        clause: "BNK.1" } })));
        assert!(refused(&kind_decl(quote! { pub BANK = "Bank" { legal_form: "bank", table: Individuals,
        clause: "BNK.1" } })));
    }
}
