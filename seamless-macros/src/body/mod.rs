mod attrs;
mod fields;

use fields::{Field, Fields};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{ext::IdentExt, parse::Parser, punctuated::Punctuated};

const DEFAULT_TAG: &str = "kind";
const VARIANT_DESCRIPTION: &str = "Variant tag";

/// The name serde uses for this identifier on the wire. Raw identifiers
/// lose their prefix, so `r#type` becomes `"type"`. Only use this for strings
/// describing the JSON; generated Rust code still needs the original `Ident`.
fn wire_name(ident: &syn::Ident) -> String {
    ident.unraw().to_string()
}

#[derive(Debug)]
pub struct Attrs {
    pub deserialize: bool,
    pub serialize: bool,
}

pub fn parse_top_attrs(attrs: TokenStream) -> Attrs {
    let idents = Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated
        .parse(attrs)
        .expect("Invalid Api attributes provided");

    // If Serialize or Deserialize attrs given, only do that one. If none given, do both.
    let serialize = idents.iter().any(|ident| ident == "Serialize");
    let deserialize = idents.iter().any(|ident| ident == "Deserialize");
    if !serialize && !deserialize {
        return Attrs {
            serialize: true,
            deserialize: true,
        };
    }

    Attrs {
        serialize,
        deserialize,
    }
}

pub fn parse_enum(e: syn::ItemEnum, attrs: Attrs) -> syn::Result<TokenStream2> {
    let api = crate::seamless_api();
    let ident = e.ident.clone();

    let top_level_props = attrs::parse(&e.attrs)?;
    let serde_tag = top_level_props
        .tag
        .unwrap_or_else(|| DEFAULT_TAG.to_owned());
    let top_level_docs = top_level_props.docs;

    // Enums with only unit variants are (de)serialized to plain strings, and other enums are
    // tagged like { "kind": "Bar", ...otherfields }, so the two kinds of variant can't be mixed.
    let is_unit = |variant: &syn::Variant| matches!(variant.fields, syn::Fields::Unit);
    let all_unit = e.variants.iter().all(is_unit);
    if !all_unit && e.variants.iter().any(is_unit) {
        return Err(syn::Error::new_spanned(
            &ident,
            "Unit enum fields can't be mixed with named fields",
        ));
    }

    // Describe each variant:
    let mut variant_infos = Vec::new();
    for variant in &e.variants {
        let variant_name = wire_name(&variant.ident);
        let variant_docs = attrs::parse(&variant.attrs)?.docs;

        // The type of the tag field, eg "kind": "Bar":
        let tag_info = quote! {
            #api::ApiBodyInfo {
                description: #VARIANT_DESCRIPTION.to_owned(),
                ty: #api::ApiBodyType::StringLiteral { literal: #variant_name.to_owned() },
            }
        };

        let variant_info = match Fields::from_syn(variant.fields.clone())? {
            // Tuple variants aren't allowed, because how would we tag them with
            // an inner prop eg "kind": "bar"?
            Fields::Unnamed(..) => {
                return Err(syn::Error::new_spanned(
                    &ident,
                    "Enum tuple variants are not allowed",
                ));
            }
            // Unit variants are just the string literal:
            Fields::Unit => quote! {
                #api::ApiBodyInfo {
                    description: #variant_docs.to_owned(),
                    ty: #api::ApiBodyType::StringLiteral { literal: #variant_name.to_owned() },
                }
            },
            // Single fields are treated like the inner struct, with our tag added to it.
            // Only inner types that are structs are allowed.
            Fields::Single(f) => {
                let ty = &f.field.ty;
                // If no variant docs, use the inner struct docs instead:
                let description = if variant_docs.is_empty() {
                    quote! { s.description }
                } else {
                    quote! { #variant_docs.to_owned() }
                };
                quote! {{
                    let s = <#ty as #api::ApiBodyStruct>::api_body_struct_info();
                    let mut keys = s.fields;
                    keys.insert(#serde_tag.to_owned(), #tag_info);
                    #api::ApiBodyInfo {
                        description: #description,
                        ty: #api::ApiBodyType::Object { keys },
                    }
                }}
            }
            // Named fields are merged with the tag:
            Fields::Named(fields) => {
                let entries = fields.iter().map(|f| {
                    let name = wire_name(f.field.ident.as_ref().unwrap());
                    let info = quote_field(f);
                    quote! { keys.insert(#name.to_owned(), #info); }
                });
                quote! {{
                    let mut keys = std::collections::HashMap::new();
                    keys.insert(#serde_tag.to_owned(), #tag_info);
                    #(#entries)*
                    #api::ApiBodyInfo {
                        description: #variant_docs.to_owned(),
                        ty: #api::ApiBodyType::Object { keys },
                    }
                }}
            }
        };
        variant_infos.push(variant_info);
    }

    let serde_tag_attr = (!all_unit).then(|| quote! { #[serde(tag = #serde_tag)] });

    // Replace our "api_body" attrs with corresponding serde attrs:
    let mut item = e;
    item.attrs.retain(|attr| !attr.path().is_ident(attrs::NAME));
    for field in item.variants.iter_mut().flat_map(|v| v.fields.iter_mut()) {
        let props = attrs::parse(&field.attrs)?;
        if props.flatten {
            let msg = "#[api_body(flatten)] can't be used on enum variant fields";
            return Err(syn::Error::new_spanned(field, msg));
        }
        replace_with_serde_attrs(field, &props);
    }

    let serde_attrs = serde_attrs(&attrs);

    Ok(quote! {
        #serde_attrs
        #serde_tag_attr
        #item

        impl #api::ApiBody for #ident {
            fn api_body_info() -> #api::ApiBodyInfo {
                #api::ApiBodyInfo {
                    description: #top_level_docs.to_owned(),
                    ty: #api::ApiBodyType::OneOf {
                        values: vec![#(#variant_infos),*],
                    },
                }
            }
        }
    })
}

pub fn parse_struct(s: syn::ItemStruct, attrs: Attrs) -> syn::Result<TokenStream2> {
    let api = crate::seamless_api();
    let ident = &s.ident;
    let docs = attrs::parse(&s.attrs)?.docs;

    let api_body_impl = match Fields::from_syn(s.fields.clone())? {
        // Newtype structs (de)serialize to their inner value:
        Fields::Single(f) => {
            let info = with_docs(quote_field(&f), &docs);
            quote! {
                impl #api::ApiBody for #ident {
                    fn api_body_info() -> #api::ApiBodyInfo {
                        #info
                    }
                }
            }
        }
        // Tuple structs (de)serialize to [val1, val2, ..]:
        Fields::Unnamed(fields) => {
            let values = fields.iter().map(quote_field);
            quote! {
                impl #api::ApiBody for #ident {
                    fn api_body_info() -> #api::ApiBodyInfo {
                        #api::ApiBodyInfo {
                            description: #docs.to_owned(),
                            ty: #api::ApiBodyType::TupleOf {
                                values: vec![#(#values),*],
                            },
                        }
                    }
                }
            }
        }
        // Structs with named fields (de)serialize to { key: value, .. }. We also implement
        // ApiBodyStruct for them, so that they can be flattened into other structs and used
        // in newtype enum variants.
        Fields::Named(fields) => {
            let entries = fields.iter().map(|f| {
                if f.attr_props.flatten {
                    // Merge the fields of the flattened struct into ours:
                    let ty = &f.field.ty;
                    quote! {
                        fields.extend(<#ty as #api::ApiBodyStruct>::api_body_struct_info().fields);
                    }
                } else {
                    let name = wire_name(f.field.ident.as_ref().unwrap());
                    let info = quote_field(f);
                    quote! { fields.insert(#name.to_owned(), #info); }
                }
            });
            quote! {
                impl #api::ApiBodyStruct for #ident {
                    fn api_body_struct_info() -> #api::ApiBodyStructInfo {
                        let mut fields = std::collections::HashMap::new();
                        #(#entries)*
                        #api::ApiBodyStructInfo {
                            description: #docs.to_owned(),
                            fields,
                        }
                    }
                }
                impl #api::ApiBody for #ident {
                    fn api_body_info() -> #api::ApiBodyInfo {
                        let s = <Self as #api::ApiBodyStruct>::api_body_struct_info();
                        #api::ApiBodyInfo {
                            description: s.description,
                            ty: #api::ApiBodyType::Object { keys: s.fields },
                        }
                    }
                }
            }
        }
        Fields::Unit => quote_spanned! {ident.span() =>
            compile_error!("ApiBody: unit structs are not supported");
        },
    };

    // Replace our "api_body" attrs with corresponding serde attrs:
    let mut item = s;
    for field in item.fields.iter_mut() {
        let props = attrs::parse(&field.attrs)?;
        replace_with_serde_attrs(field, &props);
    }

    let serde_attrs = serde_attrs(&attrs);

    Ok(quote! {
        #serde_attrs
        #item

        #api_body_impl
    })
}

/// The serde derives asked for via `#[ApiBody(..)]`. We also tell serde to use our re-export
/// of it (otherwise it expects `serde::*` to exist, which it might not).
fn serde_attrs(attrs: &Attrs) -> TokenStream2 {
    let serialize = attrs
        .serialize
        .then(|| quote! { #[derive(::seamless::serde::Serialize)] });
    let deserialize = attrs
        .deserialize
        .then(|| quote! { #[derive(::seamless::serde::Deserialize)] });
    quote! {
        #serialize
        #deserialize
        #[serde(crate = "::seamless::serde")]
    }
}

/// The `ApiBodyInfo` for a single field, with any docs on the field used as its description.
fn quote_field(f: &Field) -> TokenStream2 {
    let api = crate::seamless_api();
    let ty = &f.field.ty;
    // `#[api_body(optional)]` fields are described as `{ key?: T }`. OptionalField is only
    // implemented for Option<T>, so this fails to compile if the field isn't an Option.
    let info = if f.attr_props.optional {
        quote! {
            #api::ApiBodyInfo {
                description: String::new(),
                ty: #api::ApiBodyType::Optional {
                    value: Box::new(<<#ty as #api::OptionalField>::Inner as #api::ApiBody>::api_body_info()),
                },
            }
        }
    } else {
        quote! { <#ty as #api::ApiBody>::api_body_info() }
    };
    with_docs(info, &f.attr_props.docs)
}

/// Wrap an expression producing an `ApiBodyInfo` so that its description is
/// replaced with `docs`, unless `docs` is empty.
fn with_docs(info: TokenStream2, docs: &str) -> TokenStream2 {
    if docs.is_empty() {
        return info;
    }
    quote! {{
        let mut info = #info;
        info.description = #docs.to_owned();
        info
    }}
}

/// Swap our `api_body` field attrs for the serde attrs that they correspond to.
fn replace_with_serde_attrs(field: &mut syn::Field, props: &attrs::Props) {
    field
        .attrs
        .retain(|attr| !attr.path().is_ident(attrs::NAME));

    if props.flatten {
        field.attrs.push(syn::parse_quote! { #[serde(flatten)] });
    }

    if props.optional {
        // Omit the field when it's None, and default it to None when it's missing:
        field.attrs.push(syn::parse_quote! {
            #[serde(default, skip_serializing_if = "::seamless::api::OptionalField::is_none")]
        });
    }
}
