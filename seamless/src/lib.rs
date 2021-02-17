#![warn(missing_docs)]
//! An opinionated library to easily plug RPC style JSON APIs into your existing HTTP framework.
//!
//! The main USP of this library is that it takes advantage of trait and macro magic to automatically infer
//! the shape of the API (paths, descriptions, and the type of request and response for each route) from the
//! Rust code, without requiring any external API definition to be created or maintained. This allows one to
//! generate (as one example) a TypeScript based API client to allow type safe communication from a browser.
//!
//! # Introduction
//!
//! Seamless is a library primarily designed to facilitate communication between a Rust backend
//! and a TypeScript (or similar) client via JSON. By using this library you get:
//! - The ablity to use any async framework of your choice without feature flags and such.
//! - A self describing API that can automatically provide back enough information to generate
//!   a fully typed client in a language like TypeScript. This works by using the provided
//!   [`macro@ApiBody`] macro to analyse structs and enums and ensure that type information is in sync with
//!   how `serde` will Serialize/Deserialize it, along with some trait magic applied to handlers.
//! - Consistent error handling: All errors returned from the API must be convertable into an
//!   [`type@ApiError`] type. This behaviour can be derived using the provided [`macro@ApiError`] macro
//!   to make it easy to work with domain specific errors in the backend and then describe how they
//!   should be presented to end users.
//! - The ability to guard requests using the [`handler::RequestParam`] trait to asynchronously attempt to load
//!   things from an incoming request, and only calling the request handler if all such loads succeed.
//!   This is useful for loading things like user information, to guarantee that a valid user exists
//!   before a handler function can run.
//!
//! This library also has limitations, some of them being:
//! - Facilities for creating more 'RESTful' APIs may be more sparse (you _can_ create a RESTful API with
//!   this, but the library is optimised for APIs that are more fluid and RPC like in nature).
//! - Streaming of request and response bodies is not supported. Currently the library assumes you'll be
//!   primarily working with JSON (that doesn't stream so well) or small binary blobs, and doesn't expose
//!   means to stream data in and our of handlers for the sake of simplicity (instead, everything comes in
//!   and leaves as a `Vec<u8>`).
//! - Type information from the [`Api::info()`] method is tuned towards generating TypeScript client
//!   code, and is not sufficiently detailed to, for instance, generate a suitable Rust client.
//! - API handlers all take the form `async fn(...params) -> Result<response,impl Into<ApiError>>` at
//!   present. You can wrangle anything into this shape by using [`std::convert::Infallible`] as the error
//!   type if there is none, and using `async move` closures to "asyncify" sync handlers. Sometimes you'll
//!   need to explicitly type things to give the compiler enough to work with.
//!
//! # Example
//!
//! Below is a fully self contained example of using this library. Please have a look in the `examples`
//! folder for more detailed examples.
//!
//! ```
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! use seamless::{
//!     http::{ Request },
//!     api::{ Api, ApiBody, ApiError },
//!     handler::body::{ Json }
//! };
//!
//! // The API relies on types that have been annotated with `ApiBody` (request and response
//! // types) or `ApiError` (for any errors we might give back). These annotations do some
//! // reflection to allow us to get information about the shape of the type and doc comments
//! // added to it, as well as ensuring that they can be Serialized/Deserialized.
//!
//! /// Provide two numbers to get back the division of them.
//! #[ApiBody]
//! struct DivisionInput {
//!     a: usize,
//!     b: usize
//! }
//!
//! /// The division of two numbers `a` and `b`.
//! #[ApiBody]
//! #[derive(PartialEq)]
//! struct DivisionOutput {
//!     a: usize,
//!     b: usize,
//!     /// The division of the first and second number
//!     result: usize
//! }
//!
//! // We can use `seamless::ApiError` to easily allow an existing
//! // enum or struct to be returnable from the API if things go wrong.
//! // `ApiError`s must implement `Debug` and `Display`. We use `thiserror`
//! // here to easily implement Display.
//! #[derive(ApiError, Debug, thiserror::Error, PartialEq)]
//! enum MathsError {
//!     #[error("Division by zero")]
//!     #[api_error(external, code=400)]
//!     DivideByZero
//! }
//!
//! // We instantiate an API and add routes to it like so. The handler function would
//! // often be an external `async fn foo()` defined elsewhere (see the examples), but
//! // for the sake of this example we define it inline.
//! let mut api = Api::new();
//!
//! api.add("maths.divide")
//!    .description("Divide two numbers by each other")
//!    .handler(|body: Json<DivisionInput>| async move {
//!        let a = body.json.a;
//!        let b = body.json.b;
//!        a.checked_div(b)
//!            .ok_or(MathsError::DivideByZero)
//!            .map(|result| DivisionOutput { a, b, result })
//!    });
//!
//! // Once we've added routes to the `api`, we use it by sending `http::Request`s to it.
//! // Below, we give the API a quick test and assert that we get back what we expect when
//! // we do this:
//!
//! let req = Request::post("/maths.divide")
//!    .body(serde_json::to_vec(&DivisionInput { a: 20, b: 10 }).unwrap())
//!    .unwrap();
//! assert_eq!(
//!     api.handle(req).await.unwrap().into_body(),
//!     serde_json::to_vec(&DivisionOutput{ a: 20, b: 10, result: 2 }).unwrap()
//! );
//!
//! let req = Request::post("/maths.divide")
//!    .body(serde_json::to_vec(&DivisionInput { a: 10, b: 0 }).unwrap())
//!    .unwrap();
//! assert_eq!(
//!     api.handle(req).await.unwrap_err().unwrap_err(),
//!     ApiError {
//!         code: 400,
//!         internal_message: "Division by zero".to_owned(),
//!         external_message: "Division by zero".to_owned(),
//!         value: None
//!     }
//! );
//! # });
//! ```

pub mod handler;
pub mod api;

// Only exposed for seamless_macros; doesn't need to be documented
#[doc(hidden)]
pub mod serde;

pub use seamless_macros::*;

pub use async_trait::async_trait;

/// A re-export of types from the `http` crate that are useful here.
pub mod http {
    pub use http::{ Request, Response, Method };
}

pub use api::{
    Api,
    ApiBody,
    ApiBodyInfo,
    ApiBodyType,
    ApiError
};
