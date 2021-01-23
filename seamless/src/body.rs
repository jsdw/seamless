use std::collections::HashMap;

pub use serde::{ Serialize, Deserialize };

pub use seamless_macros::*;

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct ApiBodyType {
    /// A description of the type (could be empty):
    pub description: String,
    /// The shape of the type (to be useful, this
    /// should correspond to the JSON returned):
    #[serde(rename = "shape")]
    pub ty: Type
}

/// Primarily for internal use; structs can
/// be converted directly to this, so we know at the
/// type level that they can be represented in this way,
/// and can skip a little enum matching
pub struct ApiBodyStructType {
    pub description: String,
    pub struc: HashMap<String, ApiBodyType>
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
#[serde(tag = "type")]
pub enum Type {
    String,
    Number,
    Boolean,
    Null,
    // any
    Any,
    // type[]
    ArrayOf { value: Box<ApiBodyType> },
    // [type1, type2, type3]
    TupleOf { values: Vec<ApiBodyType> },
    // { [key: string]: type }
    ObjectOf { value: Box<ApiBodyType> },
    // { key1: type1, key2: type2, ... }
    Object { keys: HashMap<String, ApiBodyType> },
    // type1 | type2 | type3
    OneOf { values: Vec<ApiBodyType> },
    // "stringvalue"
    StringLiteral { literal: String },
    // key?: type, or type | undefined (depending on context)
    Optional { value: Box<ApiBodyType> }
}

/// Denotes that a thing can be represented in ApiBody, and
/// hands back that representation when asked:
pub trait ApiBody {
    /// Type info:
    fn api_body_type() -> ApiBodyType;

    /// Serialize to JSON:
    fn to_json_vec(&self) -> Vec<u8>
    where Self: ::serde::Serialize {
        serde_json::to_vec(self)
            .expect("impl of ApiBody should guarantee valid JSON conversion (1)")
    }

    /// Deserialize from JSON Value:
    fn to_json_value(&self) -> serde_json::Value
    where Self: ::serde::Serialize {
        serde_json::to_value(self)
            .expect("impl of ApiBody should guarantee valid JSON conversion (2)")
    }

    /// Deserialize from JSON:
    fn from_json_slice(bytes: &[u8]) -> serde_json::Result<Self>
    where Self: ::serde::de::DeserializeOwned {
        serde_json::from_slice(bytes)
    }

    /// Deserialize from JSON Value:
    fn from_json_value(value: serde_json::Value) -> serde_json::Result<Self>
    where Self: ::serde::de::DeserializeOwned {
        serde_json::from_value(value)
    }
}

pub trait ApiBodyStruct {
    fn api_body_struct_type() -> ApiBodyStructType;
}

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
    ( $( $($name:ident),+ => $ty:expr ),+ ) => (
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
    i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64 => Type::Number,
    bool => Type::Boolean,
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