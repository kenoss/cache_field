#![allow(clippy::needless_doctest_main)]

//! # `struct_cache_field`
//!
//! `struct_cache_field` provides procedual macros to declare/manage cache fields for methods.
//!
//! ## Usage
//!
//! ```rust
//! #[struct_cache_field::impl_cached_method]
//! impl Hoge {
//!     pub fn two_times_x(&self) -> u64 {
//!         2 * self.x
//!     }
//!
//!     fn x_plus_1(&mut self) -> u64 {
//!         self.x = self.x + 1;
//!         self.x
//!     }
//! }
//!
//! #[struct_cache_field::add_cache_field]
//! struct Hoge {
//!     x: u64,
//! }
//!
//! fn main() {
//!     let mut hoge = Hoge {
//!         x: 1,
//!         __cache_fields__: Default::default(),
//!     };
//!
//!     assert_eq!(hoge.two_times_x(), &2);
//!     assert_eq!(hoge.two_times_x(), &2);
//!     hoge.x = 2;
//!     assert_eq!(hoge.two_times_x(), &2);
//!
//!     assert_eq!(hoge.x_plus_1(), &3);
//!     assert_eq!(hoge.x_plus_1(), &3);
//!     hoge.x = 3;
//!     assert_eq!(hoge.x_plus_1(), &3);
//! }
//! ```
//!
//! `#[impl_cached_method]` generates a struct to hold caches for methods.
//!
//! ```rust
//! struct __struct_cache_field__HogeCacheFields {
//!     two_times_x: ::core::cell::OnceCell<u64>,
//!     x_plus_1: ::core::cell::OnceCell<u64>,
//! }
//! ```
//!
//! `#[add_cache_field]` adds it to the original struct definition.
//!
//! ```rust
//! # struct __struct_cache_field__HogeCacheFields;
//! struct Hoge {
//!     x: u64,
//!     __cache_fields__: __struct_cache_field__HogeCacheFields,
//! }
//! ```
//!
//! Note that currently procedural macro in expression position is currently not supported.
//! So, you need to initialize `__cache_fields__` with `Default::default()` by yourself.
//!
//! You MUST use both `#[impl_cached_method]` and `#[add_cache_field]` together.
//! If you use only `#[impl_cached_method]`, it can cause a compile error in other crates.
//! Because this crate uses type-name-keyed compile time storage.
//! In the above example, `#[impl_cached_method]` registeres data with key `"Hoge"`, and
//! `#[add_cache_field]` consumes it.

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
            "arguments must be empty `struct_cache_field::impl_cached_method`",
        ));
    }

    let syn::Item::Impl(impl_) = input else {
        return Err(syn::Error::new(input.span(), "expected `impl ...`"));
    };
    if let Some((_, path, for_)) = &impl_.trait_ {
        let mut spans = TokenStream::new();
        spans.append_all([path]);
        spans.append_all([for_]);
        return Err(syn::Error::new_spanned(
            spans,
            "expected `impl ...` without trait",
        ));
    }

    let (items, fields): (Vec<syn::ImplItem>, Vec<Option<TokenStream>>) = multiunzip(
        impl_
            .items
            .iter()
            .map(rewrite_cached_method)
            .collect::<syn::Result<Vec<_>>>()?,
    );
    let mut impl_ = impl_.clone();
    impl_.items = items;
    let fields = fields.into_iter().flatten().collect_vec();
    storage::register_cache_fields(&impl_.self_ty, &impl_.generics, fields)?;

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
            "arguments must be empty `struct_cache_field::add_cache_field`",
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
        &format!(
            "__struct_cache_field__{}CacheFields",
            &struct_.ident.to_string()
        ),
        Span::call_site(),
    );
    let cache_fields = storage::withdraw_cache_fields(&struct_.ident, &struct_.generics)?;
    // Extract type parameter and and make phantom fields for the struct.
    //
    // It is easier to use phantom fields rather than checking each type parameter is actually used.
    // We use them only for `syn::GenericParam::Type`.
    let mut generics = struct_.generics.clone();
    generics.params = generics
        .params
        .into_iter()
        .filter(|param| match param {
            syn::GenericParam::Lifetime(_) | syn::GenericParam::Const(_) => false,
            syn::GenericParam::Type(_) => true,
        })
        .collect();
    let phantom_fields = generics
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let ident = syn::Ident::new(&format!("_phantom{i}"), Span::call_site());
            quote! {
                #ident: ::core::marker::PhantomData<#param>
            }
        })
        .collect_vec();
    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let cache_fields_struct = quote! {
        #[derive(Default)]
        struct #cache_fields_struct_name #ty_generics #where_clause {
            #(#cache_fields,)*
            #(#phantom_fields,)*
        }
    };

    // Add the above struct to original struct.
    let embedding = syn::Field::parse_named
        .parse2(quote! { __cache_fields__: #cache_fields_struct_name #ty_generics })
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
    fn test_rewrite_cached_method_1() -> syn::Result<()> {
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
        let expected_struct_cache_field = quote! {
            two_times_x: ::core::cell::OnceCell<u64>
        };

        let Ok((got_item, Some(got_cache_field))) = rewrite_cached_method(&item) else {
            panic!();
        };
        dbg!(got_item.clone().into_token_stream().to_string());
        dbg!(expected_item.clone().into_token_stream().to_string());
        assert_eq!(
            (got_item, Some(got_cache_field.to_string())),
            (expected_item, Some(expected_struct_cache_field.to_string()))
        );

        Ok(())
    }

    #[test]
    fn test_rewrite_cached_method_2() -> syn::Result<()> {
        use quote::ToTokens;

        let item = syn::parse2(quote! {
            fn x_plus_1(&mut self) -> u64 {
                self.x = self.x + 1;
                self.x
            }
        })?;

        let expected_item: syn::ImplItem = syn::parse2(quote! {
            fn x_plus_1(&mut self) -> &u64 {
                self.__cache_fields__.x_plus_1.get_or_init(|| {{
                    self.x = self.x + 1;
                    self.x
                }})
            }
        })?;
        let expected_struct_cache_field = quote! {
            x_plus_1: ::core::cell::OnceCell<u64>
        };

        let Ok((got_item, Some(got_cache_field))) = rewrite_cached_method(&item) else {
            panic!();
        };
        dbg!(got_item.clone().into_token_stream().to_string());
        dbg!(expected_item.clone().into_token_stream().to_string());
        assert_eq!(
            (got_item, Some(got_cache_field.to_string())),
            (expected_item, Some(expected_struct_cache_field.to_string()))
        );

        Ok(())
    }
}
