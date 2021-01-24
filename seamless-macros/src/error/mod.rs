mod attrs;

use quote::{ quote, quote_spanned };
use proc_macro2::{ TokenStream as TokenStream2, Span };
use attrs::ApiErrorAttrs;

pub fn parse_struct(s: syn::ItemStruct) -> TokenStream2 {

    let struct_name = &s.ident;
    let crate_name = syn::Ident::new("seamless", Span::call_site());

    // get top level attrs:
    let attrs = match ApiErrorAttrs::parse(&s.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error()
    };

    // finalise them since no other attrs to merge with:
    let attrs = match attrs.finalise() {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error()
    };

    // For structs with 1 unnamed field, we can delegate to the inner ApiError, else error:
    if attrs.delegate_to_child {
        if let Err(e) = one_unnamed_field(&s.ident, &s.fields) {
            return e.to_compile_error();
        }
        return quote! {
            impl From<#struct_name> for #crate_name::error::ApiError {
                fn from(s: #struct_name) -> #crate_name::error::ApiError {
                    s.0.into()
                }
            }
        }
    }

    // We don't know how to handle generics (prolly not needed for errors..):
    if !s.generics.params.is_empty() || s.generics.where_clause.is_some() {
        return quote_spanned! {
            s.ident.span() =>
            compile_error!("ApiError: Generics are not currently supported");
        }
    }


    // What we'll set as the external message:
    let external_msg_tok = if let Some(msg) = attrs.external_message {
        quote!{ #msg.to_owned() }
    } else {
        quote!{ format!("{}", s) }
    };

    let code = syn::LitInt::new(&attrs.code.to_string(), Span::call_site());

    quote!{
        impl From<#struct_name> for #crate_name::error::ApiError {
            fn from(s: #struct_name) -> #crate_name::error::ApiError {
                #crate_name::error::ApiError {
                    code: #code,
                    internal_message: format!("{}", s),
                    external_message: #external_msg_tok,
                    value: None
                }
            }
        }
    }
}

pub fn parse_enum(e: syn::ItemEnum) -> TokenStream2 {

    let top_level_attrs = match ApiErrorAttrs::parse(&e.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error()
    };

    let struct_name = &e.ident;
    let crate_name = syn::Ident::new("seamless", Span::call_site());

    if e.variants.is_empty() {
        return syn::Error::new_spanned(e.ident, "ApiError: Enums without variants are not supported")
                          .to_compile_error();
    }

    let mut enum_items = TokenStream2::new();
    for variant in e.variants {

        let inner_attrs = match ApiErrorAttrs::parse(&variant.attrs) {
            Ok(attrs) => attrs,
            Err(err) => return err.to_compile_error()
        };

        let attrs = match inner_attrs.finalise_with_parent_attrs(&top_level_attrs) {
            Ok(attrs) => attrs,
            Err(e) => return e.to_compile_error()
        };

        let ident = &variant.ident;

        // rely on the inner implementation if attrs not provided and there is one to rely on:
        if attrs.delegate_to_child {
            if let Err(e) = one_unnamed_field(&ident, &variant.fields) {
                return e.to_compile_error()
            }
            enum_items.extend(quote! {
                #struct_name::#ident (inner) => inner.into(),
            })
        }

        let full_ident = match variant.fields {
            syn::Fields::Named(..) => quote!{ #ident {..} },
            syn::Fields::Unnamed(..) => quote!{ #ident (..) },
            syn::Fields::Unit => quote!{ #ident }
        };
        let code = syn::LitInt::new(&attrs.code.to_string(), Span::call_site());
        let external_msg_tok = if let Some(msg) = attrs.external_message {
            quote!{ #msg.to_owned() }
        } else {
            quote!{ format!("{}", s) }
        };

        enum_items.extend(quote! {
            #struct_name::#full_ident => #crate_name::error::ApiError {
                code: #code,
                internal_message: format!("{}", s),
                external_message: #external_msg_tok,
                value: None
            },
        })

    }

    quote! {
        impl From<#struct_name> for #crate_name::error::ApiError {
            fn from(s: #struct_name) -> #crate_name::error::ApiError {
                match s {
                    #enum_items
                }
            }
        }
    }
}

fn one_unnamed_field(ident: &syn::Ident, fields: &syn::Fields) -> syn::Result<()> {
    let fields: Vec<_> = match fields {
        syn::Fields::Unnamed(fields) => fields.unnamed.iter().collect(),
        _ => return Err(syn::Error::new_spanned(ident,
                        "One of '#[api_error(internal)]' or '#[api_error(external)]' or \
                        '#[api_error(external = \"foo\")]' is required (1)"))
    };
    if fields.len() != 1 {
        return Err(syn::Error::new_spanned(ident,
                   "One of '#[api_error(internal)]' or '#[api_error(external)]' or \
                   '#[api_error(external = \"foo\")]' is required (2)"))
    }
    Ok(())
}