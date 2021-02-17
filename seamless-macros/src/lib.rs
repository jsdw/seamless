mod error;
mod body;

use proc_macro::TokenStream;
use quote::{ quote_spanned };
use syn::{ spanned::Spanned };

/// Use this macro to generate serde `Serialize`/`Deserialize` impls in addition
/// to an `ApiBody` impl that can hand back information about the shape of the
/// type.
///
/// # Attributes
///
/// Several attributes can be provided to tweak how this works:
///
/// - `#[ApiBody`]: Generates `serde` Serialize and Deserialize impls for this type.
/// - `#[ApiBody(Serialize,Deserialize`]: The same as above.
/// - `#[ApiBody(Serialize)]`: Only generate the `Serialize` impl for this type.
/// - `#[ApiBody(Deserialize)]`: Only generate the `Deserialize` impl for this type.
/// - `#[api_body(tag = "foo")]`: Used at the top level, right under `#[ApiBody]`, and
///   works the same as `#[serde(tag = "foo")]` would.
/// - `#[api_body(flatten)]`: Used on a struct field whose value is itself a struct, and
///   works the same as `#[serde(flatten)]` would.
///
/// # Notes
///
/// Unit enums like `enum Foo { A, B, C }` will automatically (de)serialize to one-of the string
/// literals "A", "B" or "C", which slightly differs from how Serde normally apply serialization.
/// Unit and non-unit variants cannot exist in the same enum for this reason, so that it is
/// "obvious" how the thing will be (de)serialized.
///
/// # Example
///
/// ```
/// # use seamless::ApiBody;
/// # use serde_json::json;
///
/// /// This text will form part of the description of the type
/// #[ApiBody]
/// struct Foo {
///     /// This is a value
///     value: usize,
///     bar: Bar
/// }
///
/// /// A 'Bar'y thing
/// #[ApiBody]
/// enum Bar {
///     A {
///         /// Hello!
///         hello: String
///     },
///     /// B!
///     B {
///         /// Bye!
///         bye: String
///     }
/// }
///
/// // Here's an example of what the JSON output (because it's more concise) of
/// // obtaining the type info of `Foo` would look like):
/// assert_eq!(
///     serde_json::to_value(Foo::api_body_info()).unwrap(),
///     json!({
///         "description": "This text will form part of the description of the type",
///         "shape": {
///             "type": "Object",
///             "keys": {
///                 "value": {
///                     "description": "This is a value",
///                     "shape": { "type": "Number" }
///                 },
///                 "bar": {
///                     "description": "A 'Bar'y thing",
///                     "shape": {
///                         "type": "OneOf",
///                         "values": [
///                             {
///                                 "description": "",
///                                 "shape": {
///                                     "type": "Object",
///                                     "keys": {
///                                         "kind": {
///                                             "description": "Variant tag",
///                                             "shape": { "type": "StringLiteral", "literal": "A" }
///                                         },
///                                         "hello": {
///                                             "description": "Hello!",
///                                             "shape": { "type": "String" }
///                                         }
///                                     }
///                                 }
///                             },
///                             {
///                                 "description": "B!",
///                                 "shape": {
///                                     "type": "Object",
///                                     "keys": {
///                                         "kind": {
///                                             "description": "Variant tag",
///                                             "shape": { "type": "StringLiteral", "literal": "B" }
///                                         },
///                                         "bye": {
///                                             "description": "Bye!",
///                                             "shape": { "type": "String" }
///                                         }
///                                     }
///                                 }
///                             }
///                         ]
///                     }
///                 },
///             }
///         }
///     })
/// )
/// ```
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

/// Use this macro to generate an `Into<ApiError>` implementation for your custom error
/// type. Your custom error type needs to implement `Debug` and `Display` in order to
/// derive `ApiError`. `Display` in particular determines what the error message will be.
/// You can then use attribtues to set the status code, and decide on whether the
/// error message will be `internal`-only or `external`.
///
/// If the error is marked as being `internal`, the output from the `Display` impl will be
/// set as the `internal_message` on the `ApiError` struct, and by default the `external_message`
/// field will be set to `"Internal server error"`. You can set the external message to something
/// different by using the `external = "some message"` attribute.
///
/// If the error is marked as being `external`, the output from the `Display` impl will be
/// set as the `external_message` _and_ `internal_message` on the `ApiError`.
///
/// You can also set a status code, otherwise the error will return with a status code set to 500.
///
/// # Attributes
///
/// Several attributes can be provided to tweak how this works:
/// - `#[api_error(internal)]`: At the top of a struct or on an enum variant, this
///   denotes that the error message is for internal eyes only, and the external message
///   will be set to a sensible default.
/// - `#[api_error(external = "Foo")]`: At the top of a struct of enum variant, this
///   sets the `external_message` to be "Foo", so that the `Display` impl will be set on
///   the `internal_message` field only (similar to `internal`, above).
/// - `#[api_error(code = 401)]`: At the top of a struct of enum variant, this
///   sets the status code to be returned in the `ApiError` struct.
///
/// These attributes can be combined.
///
/// # Example
///
/// ```
/// # use seamless::ApiError;
/// # use std::fmt;
/// #[derive(ApiError, Debug)]
/// #[api_error(internal, code = 401, external = "Whoops!")]
/// struct MyError;
///
/// // We could use something like `thiserror` to generate our `Display` impls:
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///         write!(f, "A thing has gone wrong")
///     }
/// }
///
/// let e: ApiError = MyError.into();
/// assert_eq!(
///     e,
///     ApiError {
///         code: 401,
///         internal_message: "A thing has gone wrong".to_owned(),
///         external_message: "Whoops!".to_owned(),
///         value: None
///     }
/// );
/// ```
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