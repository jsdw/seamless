#![warn(missing_docs)]
//! An opinionated library to easily plug RPC style JSON APIs into your existing HTTP framework.
//!
//! Here's what using it might look like:
//!
//! ```
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! use seamless::{ Api, ApiBody, ApiError, Json };
//! use http::{ Request, Response };
//!
//! /* Step 1: Define some types that can be provided or handed back */
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
//! /// We can use `seamless::ApiError` to easily allow an existing
//! /// enum or struct to be converted into an `ApiError`. We use `thiserror`
//! /// here to generate Display impls, which are what the internal and external
//! /// messages will show unless otherwise given.
//! #[derive(ApiError, Debug, thiserror::Error, PartialEq)]
//! enum DivisionError {
//!     #[error("Division by zero")]
//!     #[api_error(external, code=400)]
//!     DivideByZero
//! }
//!
//! /* Step 2: Define route handlers */
//!
//! let mut api = Api::new();
//!
//! api.add("maths.divide")
//!    .description("Divide two numbers by each other")
//!    .handler(|body: Json<DivisionInput>| async move {
//!        let a = body.json.a;
//!        let b = body.json.b;
//!        a.checked_div(b)
//!            .ok_or(DivisionError::DivideByZero)
//!            .map(|result| DivisionOutput { a, b, result })
//!    });
//!
//! /*
//!  * Step 3: Handle incoming http requests. As long as you can get an http::Request
//!  * out of your framework of choice, and handle an http::Response, you can plug this
//!  * api in.
//!  */
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
//!     api.handle(req).await.unwrap_err().unwrap_api_error(),
//!     ApiError {
//!         code: 400,
//!         internal_message: "Division by zero".to_owned(),
//!         external_message: "Division by zero".to_owned(),
//!         value: None
//!     }
//! );
//! # });
//! ```

pub mod router;
pub mod error;
pub mod body;

pub use seamless_macros::*;

pub use async_trait::async_trait;

pub use http::method::Method;

pub use router::{
    Api,
    Context,
    RouteError,
    RouteInfo,
    Json,
    Binary,
};

pub use body::{
    ApiBody,
    ApiBodyType,
    Type,
};

pub use error::{
    ApiError,
};