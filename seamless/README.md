# Seamless

[API Docs](https://docs.rs/seamless/latest/seamless)

This library aims to provide an easy-to-use and extensible approach to declaring API routes, and will automatically
keep track of various information surrounding requests and responses so that detailed route information (including type
information) can be generated based on the current state of the API.

This library is fully async, but totally independent from any particular async implementation (you don't even need to 
toggle any feature flags).

Using Seamless, we can write our API in pure Rust, and from that generate a TypeScript based API client (or just the types, 
or just documentation) based on the current state of the API. This allows us to achieve type safe communication between the
API and TypeScript-like clients without relying on external specifications like OpenAPI.

The typical steps for using this library are:

- Annotate your input and output types for these routes with the [`macro@ApiBody`] macro.
- Derive [`macro@ApiError`] (or manually implement `Into<ApiError>`) on any errors you wish to emit.
- Declare each of your API routes using this library. API handlers can just ask for whatever they need as a
  function parameter, including arbitrary state or information based on the incoming request (you decide).
- Once the API routes are declared, use [`Api::info()`] to obtain enough information about the API to
  generate fully type safe client code (the information is optimised towards generating TypeScript types/code).
- Integrate this API with something like `warp` or `rocket` so that your `seamless` API routes can live alongside
  everything else that you'd like to serve (see the examples for how this can be done).

Have a look at the examples in the `examples` directory to get a proper feel for how this library can be used, or
keep reading!

# A Basic Example

Below is a basic self contained example of using this library.

```rust
use seamless::{
    http::{ Request },
    api::{ Api, ApiBody, ApiError },
    handler::{ body::FromJson, request::Bytes, response::ToJson }
};

// The API relies on types that have been annotated with `ApiBody` (request and response
// types) or `ApiError` (for any errors we might give back). These annotations do some
// reflection to allow us to get information about the shape of the type and doc comments
// added to it, as well as ensuring that they can be Serialized/Deserialized.

#[ApiBody]
struct DivisionInput {
    a: usize,
    b: usize
}

#[ApiBody]
#[derive(PartialEq)]
struct DivisionOutput {
    a: usize,
    b: usize,
    result: usize
}

// Any errors that we return must implement `Into<ApiError>`, Display and Debug. We can derive
// `ApiError` to automate  this for us. Here we use `thiserror` to derive the Display impl
// for us. See the documentation on the `ApiError` macro for more info.
#[derive(ApiError, Debug, thiserror::Error, PartialEq)]
enum MathsError {
    #[error("Division by zero")]
    #[api_error(external, code=400)]
    DivideByZero
}

let mut api = Api::new();

// We add routes to our new API like so. The handler functions would often be defined
// separately and called from this handler. Handler functions can be async or sync, and can
// return any valid handler::HandlerResponse.
api.add("/echo")
    .description("Echoes back a JSON string")
    .handler(|body: FromJson<String>| ToJson(body.0));
api.add("/reverse")
    .description("Reverse an array of numbers")
    .handler(|body: FromJson<Vec<usize>>|
        ToJson(body.0.into_iter().rev().collect::<Vec<usize>>())
    );
api.add("/maths.divide")
   .description("Divide two numbers by each other")
   .handler(|body: FromJson<DivisionInput>| async move {
       let a = body.0.a;
       let b = body.0.b;
       a.checked_div(b)
           .ok_or(MathsError::DivideByZero)
           .map(|result| ToJson(DivisionOutput { a, b, result }))
   });

// Once we've added routes to the `api`, we use it by sending `http::Request`s to it.
// Since we're expecting JSON to be provided, we need to remember to set the correct
// content-type:

let req = Request::post("/maths.divide")
    .header("content-type", "application/json")
    .body(Bytes::from_vec(serde_json::to_vec(&DivisionInput { a: 20, b: 10 }).unwrap()))
    .unwrap();
assert_eq!(
    api.handle(req).await.unwrap().into_body(),
    serde_json::to_vec(&DivisionOutput{ a: 20, b: 10, result: 2 }).unwrap()
);
```

Check out the [API Docs](https://docs.rs/seamless/latest/seamless) for lots more information, or take a look at the examples.
