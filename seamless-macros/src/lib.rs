mod error;
mod body;

use proc_macro::TokenStream;
use quote::{ quote_spanned };
use syn::{ spanned::Spanned };

/// Use this macro to generate serde `Serialize`/`Deserialize` impls in addition
/// to an `ApiBody` impl that can hand back information about the shape of the
/// type.
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn ApiBody(attrs: TokenStream, input: TokenStream) -> TokenStream {

    let item = syn::parse_macro_input!(input as syn::Item);
    let attrs = body::parse_top_attrs(attrs);

    let s = match item {
        syn::Item::Struct(s) => {
            match body::parse_struct(s, attrs) {
                Ok(res) => res,
                Err(e) => e.to_compile_error()
            }
        },
        syn::Item::Enum(e) => {
            match body::parse_enum(e, attrs) {
                Ok(res) => res,
                Err(e) => e.to_compile_error()
            }
        },
        _ => {
            // Not applied to struct or enum! Produce compile error at
            // position of the non-struct item it's applied to
            quote_spanned! {
                item.span() =>
                compile_error!("TypeScript can only be used on structs and enums");
            }
        }
    };

    TokenStream::from(s)
}

#[proc_macro_derive(ApiError, attributes(api_error))]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input).expect("Item");

    let s = match item {
        syn::Item::Struct(s) => {
            error::parse_struct(s)
        },
        syn::Item::Enum(e) => {
            error::parse_enum(e)
        },
        _ => {
            // Not applied to struct or enum! Produce compile error at
            // position of the non-struct item it's applied to
            quote_spanned! {
                item.span() =>
                compile_error!("ApiError can only be used on structs and enums");
            }
        }
    };

    TokenStream::from(s)
}