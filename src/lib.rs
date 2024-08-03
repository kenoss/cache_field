mod storage;

use itertools::{multiunzip, Itertools};
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::parse::Parser;
use syn::parse_macro_input;
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn impl_cached_method(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::Item);

    match impl_cached_method_aux(&args.into(), &input) {
        Ok(x) => x.into(),
        Err(e) => TokenStream::from_iter([e.into_compile_error(), (quote! { #input })]).into(),
    }
}

fn impl_cached_method_aux(args: &TokenStream, input: &syn::Item) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "arguments must be empty `cache_field::impl_cached_method`",
        ));
    }

    let syn::Item::Impl(impl_) = input else {
        return Err(syn::Error::new(input.span(), "expected `impl ...`"));
    };
    match &impl_.trait_ {
        Some((_, path, for_)) => {
            let mut spans = TokenStream::new();
            spans.append_all([path]);
            spans.append_all([for_]);
            return Err(syn::Error::new_spanned(
                spans,
                "expected `impl ...` without trait",
            ));
        }
        _ => {}
    }

    let (items, fields): (Vec<syn::ImplItem>, Vec<Option<TokenStream>>) = multiunzip(
        impl_
            .items
            .iter()
            .map(|item| rewrite_cached_method(item))
            .collect::<syn::Result<Vec<_>>>()?,
    );
    let mut impl_ = impl_.clone();
    impl_.items = items;
    let fields = fields.into_iter().filter_map(|x| x).collect_vec();
    storage::register_cache_fields(&impl_.self_ty, fields)?;

    Ok(quote! {
        #impl_
    })
}

fn rewrite_cached_method(
    item: &syn::ImplItem,
) -> syn::Result<(syn::ImplItem, Option<TokenStream>)> {
    let syn::ImplItem::Fn(fn_) = item else {
        return Ok((item.clone(), None));
    };
    let ident = &fn_.sig.ident;
    let block = &fn_.block;
    let syn::ReturnType::Type(_, return_ty) = &fn_.sig.output else {
        return Err(syn::Error::new_spanned(
            fn_.sig.clone(),
            "cache-generator method must have return type",
        ));
    };
    let mut new_fn = fn_.clone();
    new_fn.block = syn::parse2(quote! {{
        self.__cache_fields__.#ident.get_or_init(|| {
            #block
        })
    }})
    .unwrap();
    new_fn.sig.output = syn::parse2(quote! { -> &#return_ty }).unwrap();
    let field = quote! {
        #ident: ::core::cell::OnceCell<#return_ty>
    };
    Ok((new_fn.into(), Some(field)))
}

#[proc_macro_attribute]
pub fn add_cache_field(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::Item);

    match add_cache_field_aux(&args.into(), &input) {
        Ok(x) => x.into(),
        Err(e) => TokenStream::from_iter([e.into_compile_error(), (quote! { #input })]).into(),
    }
}

fn add_cache_field_aux(args: &TokenStream, input: &syn::Item) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "arguments must be empty `cache_field::add_cache_field`",
        ));
    }

    let syn::Item::Struct(struct_) = input else {
        return Err(syn::Error::new(input.span(), "expected `struct ...`"));
    };
    let syn::Fields::Named(fields) = &struct_.fields else {
        return Err(syn::Error::new(
            struct_.fields.span(),
            "expected named fields",
        ));
    };

    // Define a new struct holding caches. This makes initialization easy.
    let cache_fields_struct_name = syn::Ident::new(
        &format!("__cache_field__{}CacheFields", &struct_.ident.to_string()),
        Span::call_site(),
    );
    let Some(cache_fields) = storage::get_cache_fields(&struct_.ident) else {
        return Err(syn::Error::new(
            struct_.fields.span(),
            "cached methods not defined. maybe forgot to `#[cache_field::impl_cached_method]`?",
        ));
    };
    let cache_fields_struct = quote! {
        #[derive(Default)]
        struct #cache_fields_struct_name {
            #(#cache_fields,)*
        }
    };

    // Add the above struct to original struct.
    let embedding = syn::Field::parse_named
        .parse2(quote! { __cache_fields__: #cache_fields_struct_name })
        .unwrap();
    let mut fields = fields.clone();
    fields.named.push(embedding);
    let mut struct_ = struct_.clone();
    struct_.fields = syn::Fields::Named(fields);

    Ok(quote! {
        #struct_

        #cache_fields_struct
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_cached_method() -> syn::Result<()> {
        use quote::ToTokens;

        let item = syn::parse2(quote! {
            pub fn two_times_x() -> u64 {
                2 * self.x
            }
        })?;

        let expected_item: syn::ImplItem = syn::parse2(quote! {
            pub fn two_times_x() -> &u64 {
                self.__cache_fields__.two_times_x.get_or_init(|| {{
                    2 * self.x
                }})
            }
        })?;
        let expected_cache_field = quote! {
            two_times_x: ::core::cell::OnceCell<u64>
        };

        let Ok((got_item, Some(got_cache_field))) = rewrite_cached_method(&item) else {
            panic!();
        };
        dbg!(got_item.clone().into_token_stream().to_string());
        dbg!(expected_item.clone().into_token_stream().to_string());
        assert_eq!(
            (got_item, Some(got_cache_field.to_string())),
            (expected_item, Some(expected_cache_field.to_string()))
        );

        Ok(())
    }
}
