//! The types used to describe a [seamless](https://docs.rs/seamless) API. A list of
//! [`RouteInfo`]s is what `seamless::Api::info()` hands back, and contains enough
//! information to generate a type safe client for the API.
//!
//! This crate is `no_std` (though it requires `alloc`), and has no dependencies other than
//! `serde`. Enable the `serialize` and/or `deserialize` features to derive
//! [`serde::Serialize`] and/or [`serde::Deserialize`] on every type here.

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "deserialize")]
use serde::Deserialize;
#[cfg(feature = "serialize")]
use serde::Serialize;

/// Information about a single route.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "deserialize", derive(Deserialize))]
pub struct RouteInfo {
    /// The name/path that the HTTP request needs to contain
    /// in order to match this route.
    pub name: String,
    /// The HTTP method expected in order for an HTTP request to
    /// match this route, as a string.
    pub method: String,
    /// The description of the route, as set by `RouteBuilder::description()` in seamless.
    pub description: String,
    /// The shape of the data expected to be provided as part of the HTTP request
    /// for this route. This doesn't care about the wire format that the data is provided in,
    /// though the type information is somewhat related to what the possible types that can
    /// be sent and received via JSON.
    ///
    /// Types can use seamless' `ApiBody` macro, or implement its `ApiBody` trait manually,
    /// in order to describe the shape and documentation that they should hand back.
    pub request_type: ApiBodyInfo,
    /// The shape of the data that is returned from this API route.
    pub response_type: ApiBodyInfo,
}

/// A representation of some type, including its description and shape.
/// This is given back for anything which implements seamless' `ApiBody` trait,
/// and is automatically generated if one uses seamless' `ApiBody` macro on some type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "deserialize", derive(Deserialize))]
pub struct ApiBodyInfo {
    /// A human friendly description of the type. When using seamless' `ApiBody`
    /// macro, this will be automatically populated based on the doc comments on the type.
    pub description: String,
    /// The shape of the type. This should correspond to the JSON returned when
    /// serializing the type. If you use seamless' `ApiBody` macro, this is
    /// guaranteed to be the case.
    #[cfg_attr(
        any(feature = "serialize", feature = "deserialize"),
        serde(rename = "shape")
    )]
    pub ty: ApiBodyType,
}

/// An enum representing the shape of the JSON that is provided or output from the API.
/// There is a straightforward mapping from this to TypeScript types.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "deserialize", derive(Deserialize))]
#[cfg_attr(
    any(feature = "serialize", feature = "deserialize"),
    serde(tag = "type")
)]
pub enum ApiBodyType {
    /// Corresponds to the TypeScript type `string`.
    String,
    /// Corresponds to the TypeScript type `number`.
    Number,
    /// Corresponds to the TypeScript type `boolean`.
    Boolean,
    /// Corresponds to the TypeScript type `null`.
    Null,
    /// Corresponds to the TypeScript type `any`.
    ///
    /// This is used when the shape cannot be statically determined, and we want to
    /// indicate that any type can be provided. In many cases [`ApiBodyType::Unknown`]
    /// may be preferred.
    Any,
    /// Corresponds to the TypeScript type `unknown`.
    ///
    /// This is used when the shape cannot be statically determined, and we want to
    /// indicate that we're not sure what the type is, unlike [`ApiBodyType::Any`]
    /// which hints that *any* type can be provided.
    Unknown,
    /// Indicate that we expect binary data to be provided. This may correspond to
    /// `Blob` in a UI.
    Binary,
    /// An array of values of one type, where each value has the type `value`, eg
    /// `string[]` or `number[]`.
    ArrayOf {
        /// The type of all of the values in the array.
        value: Box<ApiBodyInfo>,
    },
    /// A fixed length array of values that can be of mixed types, eg
    /// `[string, number, Foo]`.
    TupleOf {
        /// A list of each of the types in this fixed length array.
        values: Vec<ApiBodyInfo>,
    },
    /// An object where the keys are strings and the values are all of the same type, eg
    /// `{ [key: string]: Foo }`.
    ObjectOf {
        /// The type of all of the values in the object/map.
        value: Box<ApiBodyInfo>,
    },
    /// An object whose keys and value types are known at compile time, eg
    /// `{ foo: string, bar: boolean, wibble: Foo }`.
    Object {
        /// The property name and type of each entry in the object.
        keys: BTreeMap<String, ApiBodyInfo>,
    },
    /// The type is one of several variants, eg
    /// `string | number | Foo`.
    OneOf {
        /// Each of the possible types that this can be.
        values: Vec<ApiBodyInfo>,
    },
    /// The type is a string literal with a specific value, eg
    /// `"stringvalue"`.
    StringLiteral {
        /// The exact string literal that we expect.
        literal: String,
    },
    /// The object key may be omitted, eg `{ key?: Foo }`. This is expected to be used
    /// on struct fields in conjunction with serde's `skip_serializing_if = Option::is_none`
    /// attribute. Adding `#[api_body(optional)]` on a field when using seamless' `ApiBody`
    /// macro emits this, as well as the corresponding serde attribute.
    Optional {
        /// The type that is optional.
        value: Box<ApiBodyInfo>,
    },
}
