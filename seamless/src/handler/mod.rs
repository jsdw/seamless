//! This module provides traits and structs that relate to the handler functions
//! that we can pass to API routes.
mod param;
mod handler;

/// This contains the [`RequestBody`] trait, which you can implement on a type
/// in order to allow it to be used at a parameter in a handler function which
/// can extract data from the request body. A couple of convenience types
/// ([`body::Json`] and [`body::Binary`]) are exposed which implement this trait.
pub mod body;
pub use body::{ RequestBody };
pub use param::{ RequestParam };

// This is only ever exposed internally (used inside the api router),
// so let's not expose it to the world:
pub (crate) use handler::{ Handler };
pub use handler::{ IntoHandler };
