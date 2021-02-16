//! A router implementation that can handle requests in a type safe way, while
//! also allowing information about the routes, route descriptions and expected
//! input and output types to be automatically generated from it.

mod api;
mod info;
mod error;

pub use api::{ Api, RouteBuilder, RouteError, RouteInfo };
pub use info::{ ApiBody, ApiBodyInfo, ApiBodyType };
pub use error::{ ApiError };

// For convenience, since these are used a fair bit:
pub use http::{ Request, Response };

// These are used in seamless_macros but are not expected to
// be made use of elsewhere and so are hidden from the docs:
#[doc(hidden)]
pub use info::{ ApiBodyStruct, ApiBodyStructInfo };