use proc_macro2::TokenStream;
use quote::ToTokens;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use syn::parse::Parser;
use syn::spanned::Spanned;

static CACHE_FIELDS: LazyLock<Mutex<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn register_cache_fields(
    ty: &syn::Type,
    cache_fields: Vec<TokenStream>,
) -> syn::Result<()> {
    let ty = ty.to_token_stream().to_string();
    let cache_fields = cache_fields
        .into_iter()
        .map(|field| field.to_string())
        .collect();

    if CACHE_FIELDS.lock().unwrap().contains_key(&ty) {
        return Err(syn::Error::new(
            ty.span(),
            "type name conflicted, cache fields arleady registered. maybe someone forgot to add `#[struct_cache_field::add_cache_field]`?",
        ));
    }

    CACHE_FIELDS.lock().unwrap().insert(ty, cache_fields);

    Ok(())
}

pub(crate) fn withdraw_cache_fields(ty: &proc_macro2::Ident) -> Option<Vec<syn::Field>> {
    let ty = ty.to_token_stream().to_string();
    let mut map = CACHE_FIELDS.lock().unwrap();
    let cache_fields = map.remove(&ty)?;
    let cache_fields = cache_fields
        .iter()
        .map(|field| {
            syn::Field::parse_named
                .parse2(field.parse().unwrap())
                .unwrap()
        })
        .collect();
    Some(cache_fields)
}
