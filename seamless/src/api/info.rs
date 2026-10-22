use serde::Serialize;
use std::collections::HashMap;

/// A representation of some type, including its description and shape.
/// This is given back for anything which implements the [`trait@crate::ApiBody`] trait,
/// and is automatically generated if one uses the [`macro@crate::ApiBody`] macro on some type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiBodyInfo {
    /// A human friendly description of the type. When using the
    /// [`ApiBody`](seamless_macros::ApiBody) macro, this will be automatically
    /// populated based on the doc comments on the type.
    pub description: String,
    /// The shape of the type. This should correspond to the JSON returned when
    /// serializing the type. If you use the [`ApiBody`](seamless_macros::ApiBody)
    /// macro, this is guaranteed to be the case.
    #[serde(rename = "shape")]
    pub ty: ApiBodyType,
}

// Primarily for internal use; structs can
// be converted directly to this, so we know at the
// type level that they can be represented in this way,
// and can skip a little enum matching
#[doc(hidden)]
pub struct ApiBodyStructInfo {
    pub description: String,
    pub fields: HashMap<String, ApiBodyInfo>,
}

/// An enum representing the shape of the JSON that is provided or output from the API.
/// There is a straightforward mapping from this to TypeScript types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
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
        keys: HashMap<String, ApiBodyInfo>,
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
    /// attribute. Adding `#[api_body(optional)]` on a field when using the `ApiBody` macro
    /// emits this, as well as the corresponding serde attribute.
    Optional {
        /// The type that is optional.
        value: Box<ApiBodyInfo>,
    },
}

/// Any type that implements this trait can be described in terms of [`ApiBodyInfo`], and
/// can potentially also be serialized or deserialized from JSON.
///
/// This type should not be manually implemented in most cases; instead the [`ApiBody`](seamless_macros::ApiBody)
/// macro should be relied on to ensure that the description and shape of the type are consistent with how
/// it will be serialized.
///
/// In some cases however, it is necessary to manually implement this for a type (for example, an external type).
pub trait ApiBody {
    /// This returns information about the shape of the type and description of parts of it.
    fn api_body_info() -> ApiBodyInfo;
}

/// The ApiBody macro implements this trait for all struct types, so that we can properly
/// flatten enums or newtypes containing a single struct type.
///
/// This is not meant to be used by hand.
#[doc(hidden)]
pub trait ApiBodyStruct {
    fn api_body_struct_info() -> ApiBodyStructInfo;
}

impl<T: ApiBodyStruct> ApiBodyStruct for Box<T> {
    fn api_body_struct_info() -> ApiBodyStructInfo {
        T::api_body_struct_info()
    }
}

// *** Below are the various built-in implementations of ApiBodyInfo ***

// Boxing:
impl<T: ApiBody> ApiBody for Box<T> {
    fn api_body_info() -> ApiBodyInfo {
        T::api_body_info()
    }
}

// Basic collections:
impl<T: ApiBody> ApiBody for Vec<T> {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: String::new(),
            ty: ApiBodyType::ArrayOf {
                value: Box::new(T::api_body_info()),
            },
        }
    }
}
impl<T: ApiBody> ApiBody for HashMap<String, T> {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: String::new(),
            ty: ApiBodyType::ObjectOf {
                value: Box::new(T::api_body_info()),
            },
        }
    }
}

// Serde serializes `None` as `null`, so the Option type is `T | null`.
impl<T: ApiBody> ApiBody for Option<T> {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: String::new(),
            ty: ApiBodyType::OneOf {
                values: vec![T::api_body_info(), <()>::api_body_info()],
            },
        }
    }
}

/// The [`ApiBody`](seamless_macros::ApiBody) macro uses this for fields marked with
/// `#[api_body(optional)]`, to get at the inner type of an Option, and confirm that
/// the field is indeed an Option.
///
/// This is not meant to be used by hand.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`#[api_body(optional)]` can only be used on fields of type `Option<T>`",
    label = "`{Self}` is not an `Option`"
)]
pub trait OptionalField {
    type Inner: ApiBody;
    fn is_none(&self) -> bool;
}
impl<T: ApiBody> OptionalField for Option<T> {
    type Inner = T;
    fn is_none(&self) -> bool {
        Option::is_none(self)
    }
}

// Primitives:
macro_rules! impl_api_body {
    ( $( $($name:ty),+ => $ty:expr );+ ) => (
        $($(
            impl ApiBody for $name {
                fn api_body_info() -> ApiBodyInfo {
                    ApiBodyInfo {
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
    std::sync::atomic::AtomicUsize,
    serde_json::Number => ApiBodyType::Number;
    bool,
    std::sync::atomic::AtomicBool => ApiBodyType::Boolean;
    String, &str => ApiBodyType::String;
    () => ApiBodyType::Null;
    serde_json::Value => ApiBodyType::Any
}

// Tuples:
macro_rules! impl_api_body_tuples {
    ( $( $( $name:ident )+ ),+ ) => (
        $(
            impl <$($name: ApiBody),+> ApiBody for ( $($name,)+ ) {
                fn api_body_info() -> ApiBodyInfo {
                    ApiBodyInfo {
                        description: String::new(),
                        ty: ApiBodyType::TupleOf {
                            values: vec![$($name::api_body_info(),)+]
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

impl<T: ApiBody> ApiBody for serde_json::Map<String, T> {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: String::new(),
            ty: ApiBodyType::ObjectOf {
                value: Box::new(T::api_body_info()),
            },
        }
    }
}

#[cfg(feature = "uuid")]
impl ApiBody for uuid::Uuid {
    fn api_body_info() -> ApiBodyInfo {
        ApiBodyInfo {
            description: "A 128 bit UUID".to_owned(),
            ty: ApiBodyType::String,
        }
    }
}
