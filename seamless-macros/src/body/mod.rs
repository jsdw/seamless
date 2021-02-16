mod attrs;
mod fields;

use fields::{ Fields, Field };
use proc_macro::TokenStream;
use quote::{ quote, quote_spanned };
use syn::{ punctuated::Punctuated, parse::Parser };
use proc_macro2::{ TokenStream as TokenStream2, Span };

static CRATE_NAME_STR: &str = "seamless";
static VARIANT_DESCRIPTION: &str = "Variant tag";

#[derive(Debug)]
pub struct Attrs {
    pub deserialize: bool,
    pub serialize: bool
}

pub fn parse_top_attrs(attrs: TokenStream) -> Attrs {
    let attrs = Punctuated::<syn::Ident,syn::Token![,]>::parse_terminated
        .parse(attrs)
        .expect("Invalid Api attributes provided");

    let (serialize, deserialize) = if attrs.len() == 0 {
        (true, true)
    } else {
        let mut se = false;
        let mut de = false;
        for ident in attrs {
            if ident == "Serialize" { se = true }
            else if ident == "Deserialize" { de = true}
        }
        (se, de)
    };

    Attrs { serialize, deserialize }
}

pub fn parse_enum(e: syn::ItemEnum, attrs: Attrs) -> syn::Result<TokenStream2> {
    let crate_name: syn::Ident = syn::Ident::new(CRATE_NAME_STR, Span::call_site());
    let ident = e.ident.clone();

    let top_level_attr_props = attrs::parse(&e.attrs)?;
    let serde_tag = top_level_attr_props.tag.unwrap_or("kind".to_owned());
    let top_level_docs = top_level_attr_props.docs;

    // Errors we can return during iteration:
    let tuple_variants_not_allowed = ||
        syn::Error::new_spanned(&ident, "Enum tuple variants are not allowed");
    let unit_and_nonunit_cant_be_mixed = ||
        syn::Error::new_spanned(&ident, "Unit enum fields can't be mixed with named fields");

    // Iterate variants and generate the inner TypeScript impl for each:
    let mut ts_impl_variants = vec![];
    let mut seen_unit_fields = false;
    let mut seen_nonunit_fields = false;
    for variant in e.variants.iter() {
        let variant_ident = &variant.ident;
        let variant_ident_string = variant_ident.to_string();
        let attr_props = attrs::parse(&variant.attrs)?;
        let variant_docs = attr_props.docs;

        // What fields does our enum have in it?
        let token_stream = match Fields::from_syn(variant.fields.clone())? {
            // Unnamed multiple fields aren't allowed because how do we tag
            // them with an inner prop eg "kind": "bar".
            Fields::Unnamed(..) => {
                return Err(tuple_variants_not_allowed())
            },
            // Unit fields (no values) can't live alongside other types; enums with _only_
            // unit fields will be flattened, and enums with no unit fields will be tagged
            // like { "kind": "Bar", ...otherfields }.
            Fields::Unit => {
                // Disallow unit + names variants living side by side
                seen_unit_fields = true;
                if seen_nonunit_fields { return Err(unit_and_nonunit_cant_be_mixed()) }

                quote!{{
                    ::#crate_name::api::ApiBodyInfo {
                        description: #variant_docs.to_owned(),
                        ty: ::#crate_name::api::ApiBodyType::StringLiteral{ literal: #variant_ident_string.to_owned() }
                    }
                }}
            },
            // Single fields are treated like the inner version, but we need to remember
            // to apply our tag to them too. Only inner types that are structs are allowed.
            Fields::Single(f) => {
                // Disallow unit + names variants living side by side
                seen_nonunit_fields = true;
                if seen_unit_fields { return Err(unit_and_nonunit_cant_be_mixed()) }

                let ty = &f.field.ty;
                quote!{{
                    let mut s = <#ty as ::#crate_name::api::ApiBodyStruct>::api_body_struct_info();
                    s.struc.insert(#serde_tag.to_owned(), ::#crate_name::api::ApiBodyInfo {
                        description: #VARIANT_DESCRIPTION.to_owned(),
                        ty: ::#crate_name::api::ApiBodyType::StringLiteral{ literal: #variant_ident_string.to_owned() }
                    });
                    let mut t = ::#crate_name::api::ApiBodyInfo {
                        description: #variant_docs.to_owned(),
                        ty: ::#crate_name::api::ApiBodyType::Object{ keys: s.struc }
                    };
                    // If no variant docs, use the inner struct docs instead:
                    if t.description.len() == 0 { t.description = s.description }
                    t
                }}
            },
            // Named fields are merged with the variant tag:
            Fields::Named(fields) => {
                // Disallow unit + names variants living side by side
                seen_nonunit_fields = true;
                if seen_unit_fields { return Err(unit_and_nonunit_cant_be_mixed()) }

                // Generate impl for each field:
                let entries = fields.iter().map(|f| {
                    let name = f.field.ident.as_ref().unwrap().to_string();
                    let f = quote_field(f);
                    quote!{ m.insert(#name.to_owned(), #f); }
                }).collect::<Vec<_>>();

                // Generate a match arm for this variant:
                quote!{{
                    let mut m = std::collections::HashMap::new();
                    m.insert(#serde_tag.to_owned(), ::#crate_name::api::ApiBodyInfo {
                        description: #VARIANT_DESCRIPTION.to_owned(),
                        ty: ::#crate_name::api::ApiBodyType::StringLiteral{ literal: #variant_ident_string.to_owned() }
                    });
                    #(#entries)*
                    ::#crate_name::api::ApiBodyInfo {
                        description: #variant_docs.to_owned(),
                        ty: ::#crate_name::api::ApiBodyType::Object{ keys: m }
                    }
                }}
            }
        };
        ts_impl_variants.push(token_stream);
    }

    // Do we want to generate the serialize and deserialize impl?
    let serialize_toks = if attrs.serialize {
        quote!{ #[derive(::#crate_name::serde::Serialize)] }
    } else {
        TokenStream2::new()
    };
    let deserialize_toks = if attrs.deserialize {
        quote!{ #[derive(::#crate_name::serde::Deserialize)] }
    } else {
        TokenStream2::new()
    };

    // Do we want to tag our enum? We tag when all fields are named,
    // and don't tag when all fields are unit. We shouldn't have a mix by here.
    let serde_tag_attr = if seen_nonunit_fields {
        quote!{ #[serde(tag = #serde_tag)] }
    } else {
        TokenStream2::new()
    };

    // "api_body" tag attr, if used, needs stripping before we output the enum:
    let mut sanitized_e = e;
    sanitized_e.attrs.retain(|attr| !attr.path.is_ident(attrs::NAME));

    Ok(quote!{
        #serialize_toks
        #deserialize_toks
        #serde_tag_attr
        #sanitized_e

        impl ::#crate_name::api::ApiBody for #ident {
            fn api_body_info() -> ::#crate_name::api::ApiBodyInfo {
                ::#crate_name::api::ApiBodyInfo {
                    description: #top_level_docs.to_owned(),
                    ty: ::#crate_name::api::ApiBodyType::OneOf{
                        values:vec![ #(#ts_impl_variants),* ]
                    }
                }
            }
        }
    })
}

pub fn parse_struct(s: syn::ItemStruct, attrs: Attrs) -> syn::Result<TokenStream2> {
    let crate_name: syn::Ident = syn::Ident::new(CRATE_NAME_STR, Span::call_site());
    let ident = s.ident.clone();

    let top_level_attr_props = attrs::parse(&s.attrs)?;
    let top_level_docs = top_level_attr_props.docs;

    // Iterate struct and generate the TypeScript impl:
    let ts_impl = match Fields::from_syn(s.fields.clone())? {
        // serde deserialises to inner val
        Fields::Single(f) => {
            let field_toks = quote_field(&f);
            quote!{
                impl ::#crate_name::api::ApiBody for #ident {
                    fn api_body_info() -> ::#crate_name::api::ApiBodyInfo {
                        let mut t = #field_toks;
                        let d = #top_level_docs;
                        if d.len() > 0 { t.description = d.to_owned() }
                        t
                    }
                }
            }
        },
        // serde deserialises to [val1, val2..]
        Fields::Unnamed(fields) => {
            let types = fields.iter()
                .map(quote_field)
                .collect::<Vec<_>>();
            quote!{
                impl ::#crate_name::api::ApiBody for #ident {
                    fn api_body_info() -> ::#crate_name::api::ApiBodyInfo {
                        ::#crate_name::api::ApiBodyInfo {
                            description: #top_level_docs.to_owned(),
                            ty: ::#crate_name::api::ApiBodyType::TupleOf {
                                values: vec![ #( #types ),* ]
                            }
                        }
                    }
                }
            }
        },
        // serde deserializes to { key: ty,... }. We also impl a
        // special ApiStruct trait, which we can try using
        // in the enum variant to ensure that we have named structs.
        Fields::Named(fields) => {
            let entries = fields.iter().map(|f| {
                let flatten = f.attr_props.flatten;
                if flatten {
                    // Get the field struct info and append all sub things to the map:
                    let ty = &f.field.ty;
                    quote!{{
                        let s = <#ty as ::#crate_name::api::ApiBodyStruct>::api_body_struct_info();
                        for (key, val) in s.struc.into_iter() { m.insert(key, val); }
                    }}
                } else {
                    // Just append the api_body info for the field to the map:
                    let name = f.field.ident.as_ref().unwrap().to_string();
                    let f = quote_field(&f);
                    quote!{ m.insert(#name.to_owned(), #f); }
                }
            }).collect::<Vec<_>>();
            quote!{
                impl ::#crate_name::api::ApiBodyStruct for #ident {
                    fn api_body_struct_info() -> ::#crate_name::api::ApiBodyStructInfo {
                        let mut m = std::collections::HashMap::new();
                        #(#entries)*
                        ::#crate_name::api::ApiBodyStructInfo {
                            description: #top_level_docs.to_owned(),
                            struc: m
                        }
                    }
                }
                impl ::#crate_name::api::ApiBody for #ident {
                    fn api_body_info() -> ::#crate_name::api::ApiBodyInfo {
                        let s = <#ident as ::#crate_name::api::ApiBodyStruct>::api_body_struct_info();
                        ::#crate_name::api::ApiBodyInfo {
                            description: s.description,
                            ty: ::#crate_name::api::ApiBodyType::Object { keys: s.struc }
                        }
                    }
                }
            }
        },
        // Not allowed
        Fields::Unit => {
            quote_spanned!{s.ident.span() =>
                compile_error!("TypeScript: unit structs are not supported")
            }
        }
    };

    // Do we want to generate the serialize and deserialize impl?
    let serialize_toks = if attrs.serialize {
        quote!{ #[derive(::#crate_name::serde::Serialize)] }
    } else {
        TokenStream2::new()
    };
    let deserialize_toks = if attrs.deserialize {
        quote!{ #[derive(::#crate_name::serde::Deserialize)] }
    } else {
        TokenStream2::new()
    };

    // "api_body" tag attr, if used, needs stripping before we output the enum:
    let mut sanitized_s = s;
    for field in sanitized_s.fields.iter_mut() {
        let attr_props = attrs::parse(&field.attrs)?;
        // Keep all attributes that aren't ours:
        field.attrs.retain(|attr| !attr.path.is_ident(attrs::NAME));
        // Append back on a serde(flatten) attr if the field was marked with api_body(flatten):
        if attr_props.flatten {
            let new_attr: syn::Attribute = syn::parse_quote!{ #[serde(flatten)] };
            field.attrs.push(new_attr);
        }
    }

    Ok(quote!{
        #serialize_toks
        #deserialize_toks
        #sanitized_s

        #ts_impl
    })
}

fn quote_field(f: &Field) -> TokenStream2 {
    let crate_name: syn::Ident = syn::Ident::new(CRATE_NAME_STR, Span::call_site());
    let ty = &f.field.ty;
    let docs = &f.attr_props.docs;
    quote!{{
        let mut t = <#ty as ::#crate_name::api::ApiBody>::api_body_info();
        let d = #docs;
        if d.len() > 0 { t.description = d.to_owned(); }
        t
    }}
}