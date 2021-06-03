//! This module provides traits and structs that relate to the handler functions
//! that we can pass to API routes.
mod param;
mod handler;
mod to_async;

/// This contains the [`HandlerBody`] trait, which you can implement on a type
/// in order to allow it to be used at a parameter in a handler function which
/// can extract data from the request body. A couple of convenience types
/// ([`body::FromJson`] and [`body::FromBinary`]) are exposed which implement this trait.
pub mod body;

/// This contains the [`HandlerResponse`] trait, which determines how to form an HTTP
/// response given some type. Implement this for types that you wish to be able to
/// return from handler functions.
pub mod response;

/// This contains helpers around the body that you'll need to provide as part
/// of an [`http::Request`], mainly geared around allowing requests to be streamed
/// in if desired.
pub mod request;

pub use body::{ HandlerBody };
pub use param::{ HandlerParam };
pub use response::{ HandlerResponse };

// This is only ever exposed internally (used inside the api router),
// so let's not expose it to the world:
pub (crate) use handler::{ Handler };
pub use handler::{ IntoHandler };
