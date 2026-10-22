# Seamless

[![Crates.io](https://img.shields.io/crates/v/seamless.svg)](https://crates.io/crates/seamless)
[![Docs.rs](https://img.shields.io/docsrs/seamless)](https://docs.rs/seamless)
[![CI](https://github.com/jsdw/seamless/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/jsdw/seamless/actions/workflows/ci.yml)

[See the examples](./seamless/examples/)

Seamless provides a way to declare a simple RPC style HTTP API, and from this declaration can hand back enough
information that we can generate a type safe HTTP client for interacting with it.

Seamless supports async handlers and streaming HTTP requests, but is entirely HTTP framework and async framework
agnostic: it can be wired in to your frameworks of choice. For instance, you could use seamless to handle the
API routes in an Axum app and use normal Axum routes to serve other content alongside this API.

## How does it work?

Each API route that you wish to create exists at some path, and is given a handler function which can expect
a specific shape of request body, and will return a specific shape of response, or else an error if things go wrong.

The expected body and response types must implement the `ApiBody` trait, and any errors that are
used must convert into an `ApiError`. Both can be derived via the provided macros, which under the
hood will:
- Implement the `serde::Serialize` and `serde::Deserialize` traits so that we can receive and respond with
  values of the given types.
- Implement `ApiBody` or `Into<ApiError>` so that we know the shape of the incoming and
  outgoing types, and how any given error should be presented to the client and with what status code.

See the `seamless/examples` folder in the crate repository for working examples demonstrating how to use
seamless with both Hyper and Axum, as well as a couple of more basic examples.

## A Basic Example

Below is a basic self contained example of using this library.

```rust
use seamless::{
    http::{ Request },
    api::{ Api, ApiBody, ApiError },
    handler::{ body::FromJson, request::Bytes, response::ToJson }
};

// This is an input to one of our API routes. We annotate it with the ApiBody
// macro so that it can be worked with by seamless as a request or response body.
#[ApiBody]
struct DivisionInput {
    a: usize,
    b: usize
}

// We do the same for the type we expect to hand back in a successful response
#[ApiBody]
#[derive(PartialEq)]
struct DivisionOutput {
    a: usize,
    b: usize,
    result: usize
}

// If we'd like to return a specific error from one of our API routes, we annotate it
// with ApiError, as well as expecting it to be displayable. This allows seamless to
// set the correct HTTP status code given the error returned, and return them in a
// consistent way.
#[derive(ApiError, Debug, thiserror::Error, PartialEq)]
enum MathsError {
    #[error("Division by zero")]
    #[api_error(external, code=400)]
    DivideByZero
}

let mut api = Api::new();

// Now, let's define some routes for our API. Handler functions can be async or sync,
// and can return any valid handler::HandlerResponse. Here we use `ToJson` to return
// JSON responses, and in one case return a Result which is either the error we defined
// above, or the response type we defined above.
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
// content-type so that our FromJson input tries to decode it as valid JSON.
let req = Request::post("/maths.divide")
    .header("content-type", "application/json")
    .body(Bytes::from_vec(serde_json::to_vec(&DivisionInput { a: 20, b: 10 }).unwrap()))
    .unwrap();
assert_eq!(
    api.handle(req).await.unwrap().into_body().into_vec().await.unwrap(),
    serde_json::to_vec(&DivisionOutput{ a: 20, b: 10, result: 2 }).unwrap()
);
```

## State

Most real life use cases will require some sort of state to be accessible inside a handler.

This library follows an approach a little similar to `Axum`. Any type that implements the
`handler::HandlerParam` trait can be passed into handler functions. Using this trait, you can
inspect the request to do things like obtain user information from a session ID, or you can pull
state out of the `Request` object that was placed there prior to it being handed to this library.

Here's an example:

```rust
use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ HandlerParam, body::FromJson, request::Bytes, response::ToJson },
};

// Some example state we want to inject into our handler.
#[derive(Clone)]
struct State { value: usize };

// Teach the library how to get hold of our "State" value when asked for it.
impl HandlerParam for State {
    type Error = ApiError;
    async fn handler_param(req: &http::Request<()>) -> Result<Self,Self::Error> {
        let state: State = req.extensions().get::<State>()
            .expect("State must be injected into the request")
            .clone();
        Ok(state)
    }
}

let mut api = Api::new();

// We can now ask for `State` as a parameter to the handler, as well as the HTTP body,
// which we extract as JSON using `FromJson`. We must always ask for the body last.
api.add("/echo")
    .description("Echoes back a JSON string, plus our state value")
    .handler(|state: State, body: FromJson<String>| ToJson(format!("{}, {}", body.0, state.value)));

// Now, to demo it, we build an HTTP request to send to this route.
let mut req = http::Request::post("/echo")
    .header("content-type", "application/json")
    .body(Bytes::from_vec(serde_json::to_vec("hello").unwrap()))
    .unwrap();

// When passing a request into our API, remember to inject `State` too so that
// it's available for our `HandlerParam` trait to extract when asked for.
req.extensions_mut().insert(State { value: 123 });

// We can now handle a request that is expecting our `State` to be available.
// The response comes back as a JSON string.
let res = api.handle(req).await.unwrap().into_body().into_vec().await.unwrap();
assert_eq!(res, "\"hello, 123\"".as_bytes());
```

**Note**: params implementing the `handler::HandlerParam` trait must come before the optional final param
that implements `handler::HandlerBody`. Params are resolved in order, with the first failure short circuiting
the rest.

## Extracting the request body

To extract the body from a request, the last parameter passed to a handler must implement `handler::HandlerBody`.
At most one such parameter can be provided to a handler (providing more than 1 will lead to a compile error). If no
parameter implementing `handler::HandlerBody` is provided to a handler, it's assumed that the request method will
be GET, and any body will be ignored.

Two built-in types that implement `handler::HandlerBody` exist for convenience:

- `handler::body::FromJson<T>` will assume that the request body is valid JSON that decodes to the type `T`
  (or fail with a 400 if not).
- `handler::body::FromBinary` will give you back the request body exactly as it was provided.

## Responding

Responses from handlers can be plain old synchronous objects or `std::future::Future`s. The value returned
in either case must implement `handler::HandlerResponse`. This trait determines how to create the response to
hand back to the user.

For convenience, a couple of types are provided which implement this trait:

- `handler::response::ToJson<T>` will encode the response as JSON.
- `handler::response::ToBinary` will respond with arbitrary bytes, which can be streamed back (from a
  `futures::Stream` or `futures::AsyncRead`) rather than loaded into memory all at once. Note that errors
  which occur partway through streaming a response can't be turned into an `api::ApiError`, because the
  response status will already have been sent.

Responses handed back from `Api::handle()` have a `handler::response::Body`. This is either a set of bytes that
are entirely in memory, or a stream of chunks of bytes, and can be handed over to whichever HTTP framework you're
using without needing to buffer streaming responses (see the examples folder for how this is done in Axum and Hyper).

`handler::HandlerResponse` is also implemented for `Option`s and `Result`s, returning a 404 in the event that the
`Option` is `None`, and returning the error from the `Result` (this must itself implement `Into<ApiError>`) in the
event that the `Result` is `Err`.

## Info

At some point, you'll probably want to get information about the shape of the API so that you can go
and generate a typed API client (this is, after all, the main selling point of this library). To do this,
use the `Api::info()` function.

Probably the best way to see what shapes this info can take is by looking at `api/info.rs`.

Here's an example:

```rust
use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ body::FromJson, response::ToJson },
};
use serde_json::json;

#[derive(ApiError, Debug, thiserror::Error)]
enum MathsError {
    #[error("Division by zero")]
    #[api_error(external, code=400)]
    DivideByZero
}

/// Input consisting of two numbers
#[ApiBody]
struct BinaryInput {
    /// Input 'a'
    a: usize,
    /// Input 'b'
    b: usize
}

/// Output containing the original input and result
#[ApiBody]
#[derive(PartialEq)]
struct BinaryOutput {
    a: usize,
    b: usize,
    /// The result
    result: usize
}

async fn divide(input: BinaryInput) -> Result<ToJson<BinaryOutput>,MathsError> {
    todo!()
}

// A small API with one route:
let mut api = Api::new();
api.add("maths/divide")
    .description("Divide two numbers by each other")
    .handler(|FromJson(body)| divide(body));

// Get info about this API:
let info = api.info();

// Here's what this will look like when serialized to JSON:
let info_json = json!([
    {
        "name": "maths/divide",
        "description": "Divide two numbers by each other",
        "method": "POST",
        "request_type": {
            "description": "Input consisting of two numbers",
            "shape": {
                "type": "Object",
                "keys": {
                    "a": {
                        "description": "Input 'a'",
                        "shape": { "type": "Number" }
                    },
                    "b": {
                        "description": "Input 'b'",
                        "shape": { "type": "Number" }
                    }
                }
            }
        },
        "response_type": {
            "description": "Output containing the original input and result",
            "shape": {
                "type": "Object",
                "keys": {
                    "a": {
                        "description": "",
                        "shape": { "type": "Number" }
                    },
                    "b": {
                        "description": "",
                        "shape": { "type": "Number" }
                    },
                    "result": {
                        "description": "The result",
                        "shape": { "type": "Number" }
                    }
                }
            }
        }
    }
]);
```

The "shape" object can have one of the following "type" literals: `String`, `Number`, `Boolean`, `Null`,
`Any`, `Unknown`, `Binary`, `ArrayOf`, `TupleOf`, `ObjectOf`, `Object`, `OneOf`, `StringLiteral`, `Optional`. Some
of these will come with an additional property. See `ApiBodyType` to get a better feel for exactly what the possible
responses can be.

## Integrating with other libraries

Instead of passing requests in manually, you'll probably want to attach an API you define here to a library like
`Axum` (or perhaps just plain old `Hyper`) so that you can benefit from the full power and flexibility
of a well rounded HTTP library alongside your well typed `seamless` API, and actually make the API available
externally.

See `examples/axum.rs` and `examples/hyper.rs` for examples of how you might integrate this library with those.
Essentially it boils down to being able to construct an `http::Request` from whatever input the library gives you
access to, and being able to handle the `http::Response` or error that's handed back from Seamless.

## Limitations

Seamless is designed to make it easy to create simple RPC style JSON APIs that can be "seamlessly" typed from client
to server without using external tools like OpenAPI.

- Seamless has not been optimised for building RESTful style APIs. However, the `handler::HandlerBody` and
  `handler::HandlerParam` traits in particular provide a bunch of extensibility.
- Some of the flexibility that `Serde` provides for manipulating how types are serialized and deserialized is not
  available. This library takes the approach of 'wrapping' serde using the `ApiBody` macro to deliberately restrict
  how you can transform types, ensuring that any transformations allowed are properly supported and lead to the correct
  type information being generated.
