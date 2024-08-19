use big_s::S;
use indoc::indoc;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use syn::parse::Parser;
use syn::spanned::Spanned;

#[derive(PartialEq, Eq, Hash)]
struct TypeAsString(String);

struct Value {
    generics: String,
    where_clause: Option<String>,
    cache_fields: Vec<String>,
}

static STORAGE: LazyLock<Mutex<HashMap<TypeAsString, Value>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn register_cache_fields(
    ty: &syn::Type,
    generics: &syn::Generics,
    cache_fields: Vec<TokenStream>,
) -> syn::Result<()> {
    let syn::Type::Path(ty) = ty else {
        return Err(syn::Error::new(
            ty.span(),
            "expected TypePath like `path::to::Type`",
        ));
    };
    // Overwrite type parameters to get idents.
    let mut ty_ = ty.clone();
    ty_.path.segments.last_mut().unwrap().arguments = syn::PathArguments::None;

    let key = TypeAsString(ty_.to_token_stream().to_string());
    let generics_ = generics.to_token_stream().to_string();
    let where_clause = generics
        .where_clause
        .as_ref()
        .map(|x| x.to_token_stream().to_string());
    let cache_fields = cache_fields
        .into_iter()
        .map(|field| field.to_string())
        .collect();
    let value = Value {
        generics: generics_,
        where_clause,
        cache_fields,
    };

    if STORAGE.lock().unwrap().contains_key(&key) {
        return Err(syn::Error::new(
            ty.span(),
            "type name conflicted, cache fields arleady registered. maybe someone forgot to add `#[struct_cache_field::add_cache_field]`?",
        ));
    }

    STORAGE.lock().unwrap().insert(key, value);

    Ok(())
}

pub(crate) fn withdraw_cache_fields(
    ty: &proc_macro2::Ident,
    generics: &syn::Generics,
) -> syn::Result<Vec<syn::Field>> {
    let key = TypeAsString(ty.to_token_stream().to_string());
    let mut map = STORAGE.lock().unwrap();
    let Some(value) = map.remove(&key) else {
        return Err(syn::Error::new(
            Span::call_site(),
            "cached methods not defined. maybe forgot to `#[struct_cache_field::impl_cached_method]`?",
        ));
    };

    let generics_ = generics.to_token_stream().to_string();
    let where_clause = generics
        .where_clause
        .as_ref()
        .map(|x| x.to_token_stream().to_string());
    if !(generics_ == value.generics && where_clause == value.where_clause) {
        return Err(syn::Error::new_spanned(
            generics.to_token_stream(),
            format!(
                indoc! {r#"
                    generics differ, which must coincide as string:
                        in impl cached methods: {} {}
                        in struct definition:   {} {}
                "#},
                value.generics,
                value.where_clause.as_ref().unwrap_or(&S("")),
                generics_,
                where_clause.as_ref().unwrap_or(&S("")),
            ),
        ));
    };

    let cache_fields = value
        .cache_fields
        .iter()
        .map(|field| {
            syn::Field::parse_named
                .parse2(field.parse().unwrap())
                .unwrap()
        })
        .collect();

    Ok(cache_fields)
}
