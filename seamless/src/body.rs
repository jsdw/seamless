//! A collection of types and such describing the JSON body that is handed back or provided
//! in requests to the API.
use std::collections::HashMap;

pub use serde::{ Serialize, Deserialize };

pub use seamless_macros::*;

/// A representation of some type, including its description and shape.
/// This is given back for anything which implements the [`trait@ApiBody`] trait,
/// and is automatically generated if one uses the [`macro@ApiBody`] macro on some type.
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct ApiBodyType {
    /// A human friendly description of the type. When using the
    /// [`ApiBody`](seamless_macros::ApiBody) macro, this will be automatically
    /// populated based on the doc comments on the type.
    pub description: String,
    /// The shape of the type. This should correspond to the JSON returned when
    /// serializing the type. If you use the [`ApiBody`](seamless_macros::ApiBody)
    /// macro, this is guaranteed to be the case.
    #[serde(rename = "shape")]
    pub ty: Type
}

// Primarily for internal use; structs can
// be converted directly to this, so we know at the
// type level that they can be represented in this way,
// and can skip a little enum matching
#[doc(hidden)]
pub struct ApiBodyStructType {
    pub description: String,
    pub struc: HashMap<String, ApiBodyType>
}

/// An enum representing the shape of the JSON that is provided or output from the API.
/// There is a straightforward mapping from this to TypeScript types.
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
#[serde(tag = "type")]
pub enum Type {
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
    /// This is used when the shape cannot be statically
    /// determined for one reason or another.
    Any,
    /// An array of values of one type, where each value has the type `value`, eg
    /// `string[]` or `number[]`.
    ArrayOf {
        /// The type of all of the values in the array.
        value: Box<ApiBodyType>
    },
    /// A fixed length array of values that can be of mixed types, eg
    /// `[string, number, Foo]`.
    TupleOf {
        /// A list of each of the types in this fixed length array.
        values: Vec<ApiBodyType>
    },
    /// An object where the keys are strings and the values are all of the same type, eg
    /// `{ [key: string]: Foo }`.
    ObjectOf {
        /// The type of all of the values in the object/map.
        value: Box<ApiBodyType>
    },
    /// An object whose keys and value types are known at compile time, eg
    /// `{ foo: string, bar: boolean, wibble: Foo }`.
    Object {
        /// The property name and type of each entry in the object.
        keys: HashMap<String, ApiBodyType>
    },
    /// The type is one of several variants, eg
    /// `string | number | Foo`.
    OneOf {
        /// Each of the possible types that this can be.
        values: Vec<ApiBodyType>
    },
    /// The type is a string literal with a specific value, eg
    /// `"stringvalue"`.
    StringLiteral {
        /// The exact string literal that we expect.
        literal: String
    },
    /// The type is optional, and need not be provided. It corresponds to either
    /// `{ key?: Foo }` in objects, or `Foo | undefined` in other contexts.
    Optional {
        /// The type that is optional.
        value: Box<ApiBodyType>
    }
}

/// Any type that implements this trait can be described in terms of [`ApiBodyType`], and
/// can potentially also be serialized or deserizlied from JSON.
///
/// This type should not be manually implemented in most cases; instead the [`ApiBody`](seamless_macros::ApiBody)
/// macro should be relied on to ensure that the description and shape of the type are consistent with how
/// it will be serialized.
///
/// In some cases however, it is necessary to manually implement this for a type (for example, an external type).
pub trait ApiBody {
    /// This returns information about the shape of the type and description of parts of it.
    fn api_body_type() -> ApiBodyType;

    /// Serialize the type to JSON.
    fn to_json_vec(&self) -> Vec<u8>
    where Self: ::serde::Serialize {
        serde_json::to_vec(self)
            .expect("Failed to serialize to JSON due to an invalid manual implementation (1)")
    }

    /// Serialize the type to a [`serde_json::Value`].
    fn to_json_value(&self) -> serde_json::Value
    where Self: ::serde::Serialize {
        serde_json::to_value(self)
            .expect("Failed to serialize to JSON due to an invalid manual implementation (2)")
    }

    /// Deserialize from bytes containing a JSON value.
    fn from_json_slice(bytes: &[u8]) -> serde_json::Result<Self>
    where Self: ::serde::de::DeserializeOwned {
        serde_json::from_slice(bytes)
    }

    /// Deserialize from a [`serde_json::Value`].
    fn from_json_value(value: serde_json::Value) -> serde_json::Result<Self>
    where Self: ::serde::de::DeserializeOwned {
        serde_json::from_value(value)
    }
}

/// This trait is implemented for all struct types, so that we know, at compile time,
/// whether we're working with a struct type that can be flattened or not.
#[doc(hidden)]
pub trait ApiBodyStruct {
    fn api_body_struct_type() -> ApiBodyStructType;
}


// *** Below are the various built-in implementations of ApiBodyType ***


// Basic collections:
impl <T: ApiBody> ApiBody for Vec<T> {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::ArrayOf { value: Box::new(T::api_body_type()) }
        }
    }
}
impl <T: ApiBody> ApiBody for HashMap<String,T> {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::ObjectOf { value: Box::new(T::api_body_type()) }
        }
    }
}
impl <T: ApiBody> ApiBody for Option<T> {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::Optional { value: Box::new(T::api_body_type()) }
        }
    }
}

// Primitives:
macro_rules! impl_api_body {
    ( $( $($name:path),+ => $ty:expr ),+ ) => (
        $($(
            impl ApiBody for $name {
                fn api_body_type() -> ApiBodyType {
                    ApiBodyType {
                        description: String::new(),
                        ty: $ty
                    }
                }
            }
        )+)+
    )
}
impl_api_body! {
    i8, i16, i32, i64, isize,
    u8, u16, u32, u64, usize,
    f32, f64,
    std::sync::atomic::AtomicI8,
    std::sync::atomic::AtomicI16,
    std::sync::atomic::AtomicI32,
    std::sync::atomic::AtomicI64,
    std::sync::atomic::AtomicIsize,
    std::sync::atomic::AtomicU8,
    std::sync::atomic::AtomicU16,
    std::sync::atomic::AtomicU32,
    std::sync::atomic::AtomicU64,
    std::sync::atomic::AtomicUsize => Type::Number,
    bool,
    std::sync::atomic::AtomicBool => Type::Boolean,
    String => Type::String
}
impl <'a> ApiBody for &'a str {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::String
        }
    }
}

// Tuples:
impl ApiBody for () {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::Null
        }
    }
}
macro_rules! impl_api_body_tuples {
    ( $( $( $name:ident )+ ),+ ) => (
        $(
            impl <$($name: ApiBody),+> ApiBody for ( $($name,)+ ) {
                fn api_body_type() -> ApiBodyType {
                    ApiBodyType {
                        description: String::new(),
                        ty: Type::TupleOf {
                            values: vec![$($name::api_body_type(),)+]
                        }
                    }
                }
            }
        )+
    )
}
impl_api_body_tuples! {
    A,
    A B,
    A B C,
    A B C D,
    A B C D E,
    A B C D E F,
    A B C D E F G,
    A B C D E F G H,
    A B C D E F G H I,
    A B C D E F G H I J
}

impl ApiBody for serde_json::Value {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: String::new(),
            ty: Type::Any
        }
    }
}

#[cfg(feature = "uuid")]
impl ApiBody for uuid::Uuid {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: "A UUID".to_owned(),
            ty: Type::String
        }
    }
}

#[cfg(feature = "chrono")]
impl ApiBody for chrono::NaiveDateTime {
    fn api_body_type() -> ApiBodyType {
        ApiBodyType {
            description: "A Datetime".to_owned(),
            ty: Type::String
        }
    }
}