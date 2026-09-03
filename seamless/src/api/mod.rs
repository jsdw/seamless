//! A router implementation that can handle requests in a type safe way, while
//! also allowing information about the routes, route descriptions and expected
//! input and output types to be automatically generated from it.

#[allow(clippy::module_inception)]
mod api;
mod error;
mod info;

pub use api::{Api, RouteBuilder, RouteError, RouteInfo};
pub use error::ApiError;
pub use info::{ApiBody, ApiBodyInfo, ApiBodyType};

// Export these on top of the types, so that you don't need to
// import `seamless::api::ApiBody` AND `seamless::ApiBody` for
// instance:
pub use seamless_macros::{ApiBody, ApiError};

// These are used in seamless_macros but are not expected to
// be made use of elsewhere and so are hidden from the docs:
#[doc(hidden)]
pub use info::{ApiBodyStruct, ApiBodyStructInfo};
