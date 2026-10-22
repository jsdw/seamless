mod attrs;

use attrs::{ApiErrorAttrs, ErrorImpl};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn parse_struct(s: syn::ItemStruct) -> syn::Result<TokenStream2> {
    let name = &s.ident;

    let body = match ApiErrorAttrs::parse(&s.attrs)?.finalise() {
        ErrorImpl::Delegate => {
            expect_one_unnamed_field(name, &s.fields)?;
            quote! { s.0.into() }
        }
        ErrorImpl::Convert {
            code,
            external_message,
        } => {
            // We don't know how to handle generics (probably not needed for errors):
            if !s.generics.params.is_empty() || s.generics.where_clause.is_some() {
                return Err(syn::Error::new_spanned(
                    name,
                    "ApiError: Generics are not currently supported",
                ));
            }
            api_error_expr(&code, external_message.as_ref())
        }
    };

    Ok(from_impl(name, body))
}

pub fn parse_enum(e: syn::ItemEnum) -> syn::Result<TokenStream2> {
    let name = &e.ident;
    let top_level_attrs = ApiErrorAttrs::parse(&e.attrs)?;

    if e.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "ApiError: Enums without variants are not supported",
        ));
    }

    let mut match_arms = Vec::new();
    for variant in &e.variants {
        let ident = &variant.ident;
        let error_impl =
            ApiErrorAttrs::parse(&variant.attrs)?.finalise_with_parent_attrs(&top_level_attrs)?;

        let arm = match error_impl {
            ErrorImpl::Delegate => {
                expect_one_unnamed_field(ident, &variant.fields)?;
                quote! { #name::#ident(inner) => inner.into(), }
            }
            ErrorImpl::Convert {
                code,
                external_message,
            } => {
                let pattern = match variant.fields {
                    syn::Fields::Named(..) => quote! { #ident { .. } },
                    syn::Fields::Unnamed(..) => quote! { #ident(..) },
                    syn::Fields::Unit => quote! { #ident },
                };
                let api_error = api_error_expr(&code, external_message.as_ref());
                quote! { #name::#pattern => #api_error, }
            }
        };
        match_arms.push(arm);
    }

    Ok(from_impl(name, quote! { match s { #(#match_arms)* } }))
}

/// `impl From<#name> for ApiError`, where `body` converts the value `s` into an `ApiError`.
fn from_impl(name: &syn::Ident, body: TokenStream2) -> TokenStream2 {
    let api = crate::seamless_api();
    quote! {
        impl From<#name> for #api::ApiError {
            fn from(s: #name) -> #api::ApiError {
                #body
            }
        }
    }
}

/// An `ApiError` built from the `Display` impl of the value `s`.
fn api_error_expr(code: &syn::Expr, external_message: Option<&syn::Expr>) -> TokenStream2 {
    let api = crate::seamless_api();
    // Without an explicit external message, the `Display` impl is shown externally too:
    let external_message = match external_message {
        Some(msg) => quote! { #msg.to_owned() },
        None => quote! { s.to_string() },
    };
    quote! {
        #api::ApiError {
            code: #code,
            internal_message: s.to_string(),
            external_message: #external_message,
            value: None,
        }
    }
}

/// Delegating to an inner error is only possible when there is exactly one unnamed field.
fn expect_one_unnamed_field(ident: &syn::Ident, fields: &syn::Fields) -> syn::Result<()> {
    match fields {
        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(()),
        _ => Err(syn::Error::new_spanned(
            ident,
            "One of '#[api_error(internal)]' or '#[api_error(external)]' or \
             '#[api_error(external = \"foo\")]' is required",
        )),
    }
}
