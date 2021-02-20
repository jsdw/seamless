# Seamless

[API Docs](https://docs.rs/seamless/latest/seamless)

The main goal of this library is to allow typesafe communication (and to a degree, documentation) generation
between TypeScript and your Rust API. Using this library, that can all be automatically derived from just the
Rust code, without any external definitions like OpenAPI being needed. The steps for using this library are:

- Annotate your input and output types for these routes with the [`macro@ApiBody`] macro.
- Derive [`macro@ApiError`] (or manually implement `Into<ApiError`) on any errors you wish to emit.
- Declare each of your API routes using this library. API handlers can just ask for whatever they need as a
  function parameter, including state or user info based on the incoming request.
- Once the API routes are declared, use [`Api::info()`] to obtain enough information about the API to
  generate fully type safe client code (the information is optimised towards generating TypeScript types/code).
- Integrate this API with something like `warp` or `rocket` so that your `seamless` API routes can live alongside
  everything else that you'd like to serve.

Have a look at the examples in the `examples` directory to get a proper feel for how this library can be used, or
keep reading!

# A Basic Example

Below is a basic self contained example of using this library.

```rust
# tokio::runtime::Runtime::new().unwrap().block_on(async {
use seamless::{
    http::{ Request },
    api::{ Api, ApiBody, ApiError },
    handler::body::{ Json }
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

// We add routes to our new API like so. The handler functions would often be defined separately and
// called from this handler. Handler functions can be async or sync, and can return either a `Result`
// or an `Option` where the success value is an `ApiBody` and the error an `Into<ApiError>`. Often,
// you'll find that types can be inferred, and so you can use `Json<_>` if you prefer in those cases.
api.add("/echo")
    .description("Echoes back a JSON string")
    .handler(|body: Json<String>| Some(body.json));
api.add("/reverse")
    .description("Reverse an array of numbers")
    .handler(|body: Json<Vec<usize>>| Some(body.json.into_iter().rev().collect::<Vec<usize>>()));
api.add("/maths.divide")
   .description("Divide two numbers by each other")
   .handler(|body: Json<DivisionInput>| async move {
       let a = body.json.a;
       let b = body.json.b;
       a.checked_div(b)
           .ok_or(MathsError::DivideByZero)
           .map(|result| DivisionOutput { a, b, result })
   });

// Once we've added routes to the `api`, we use it by sending `http::Request`s to it.
// Since we're expecting JSON to be provided, we need to remember to set the correct
// content-type:

let req = Request::post("/maths.divide")
    .header("content-type", "application/json")
    .body(serde_json::to_vec(&DivisionInput { a: 20, b: 10 }).unwrap())
    .unwrap();
assert_eq!(
    api.handle(req).await.unwrap().into_body(),
    serde_json::to_vec(&DivisionOutput{ a: 20, b: 10, result: 2 }).unwrap()
);
# });
```

# State

Most real life use cases will require some sort of state to be accessible inside a handler.

This library follows an approach a little similar to `Rocket`. Any type that implements the
[`handler::HandlerParam`] trait can be passed into handler functions. Using this trait, you can
inspect the request to do things like obtain user information from a session ID, or you can pull
state out of the `Request` object that was placed there prior to it being handed to this library.

Here's an example:

```rust
use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ HandlerParam, body::{ Json } },
};
# #[ApiBody]
# struct BinaryInput { a: usize, b: usize }
# #[ApiBody]
# #[derive(PartialEq)]
# struct BinaryOutput {}
# async fn divide(input: BinaryInput) -> Option<BinaryOutput> { Some(BinaryOutput {}) }
# tokio::runtime::Runtime::new().unwrap().block_on(async {

// Something we want to inject into our handler.
#[derive(Clone)]
struct State;

// Teach the library how to get hold of State when asked for it.
#[seamless::async_trait]
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

// Note that we can now ask for `State` as a parameter to the handler. State
// MUST come before our `Json<_>` parameter. `HandlerParam` impls are evaluated
// in the order that arguments appear in the parameter list.
api.add("/echo")
    .description("Echoes back a JSON string")
    .handler(|_state: State, body: Json<String>| Some(body.json));

let mut req = http::Request::post("/echo")
    .header("content-type", "application/json")
    .body(serde_json::to_vec("hello").unwrap())
    .unwrap();

// When passing a request into our API, remember to inject `State` too so that
// it's available for our `HandlerParam` trait to extract:
req.extensions_mut().insert(State);

// We can now handle the request without issues:
assert!(api.handle(req).await.is_ok());
# })
```

**Note**: params implementing the [`handler::HandleParam`] trait must come before the optional final param
that implements [`handler::HandlerBody`]. Params are resolved in order, with the first failure short circuiting
the rest.

# Info

At some point, you'll probably want to get information about the shape of the API so that you can go
and generate a typed API client (this is, after all, the main selling point of this library). To do this,
use the [`Api::info()`] function.

Probably the best way to see what shapes this info can take is by looking at `api/info.rs`.

Here's an example:

```rust
# tokio::runtime::Runtime::new().unwrap().block_on(async {
use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::body::{ Json },
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

async fn divide(input: BinaryInput) -> Result<BinaryOutput,MathsError> {
    todo!()
}

// A small APi with one route:
let mut api = Api::new();
api.add("maths/divide")
    .description("Divide two numbers by each other")
    .handler(|body: Json<_>| divide(body.json));

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
# assert_eq!(serde_json::to_value(info).unwrap(), info_json);
# })
```

The "shape" object can have one of the following "type" literals: `String`, `Number`, `Boolean`, `Null`, `Any`, `ArrayOf`, `TupleOf`, `ObjectOf`, `Object`, `OneOf`, `StringLiteral`, `Optional`. Some of these will come with an additional property.
See `seamless/src/api/info.rs` to get a better feel for exactly what the possible responses can be.

# Integrating with other libraries

Instead of passing requests in manually, you'll probably want to attach an API you define here to a library like
`Rocket` or `Warp` (or perhaps just plain old `Hyper`) so that you can benefit from the full power and flexibility
of a well rounded HTTP library alongside your well typed `seamless` API, and actually make the API available
externally.

See `examples/warp.rs` and `examples/rocket.rs` for examples of how you might integrate this library with those.
Essentially it boils down to being able to construct an `http::Request` from whatever input the library gives you
access to, and being able to handle the `http::Response` or error that's handed back from Seamless.

# Limitations

Seamless is designed to make it easy to create simple RPC style JSON APIs that can be "seamlessly" typed from client
to server without using external tools like OpenAPI.

- Seamless has not been optimised for building RESTful style APIs (notably, the ability to work with query params is
lacking, because they do not play nicely with the type safety that this library tries to provide).
- Some of the flexiblity that `Serde` provides for manipulating how types are serialized and deserialized is not
available. This library takes the approach of 'wrapping' serde using the [`macro@ApiBody`] macro to deliberately restrict
how you can transform types, ensuring that any transformations allowed are properly supported and lead to the correct
type information being generated.
- Streaming request and response bodies back from seamless is currently not supported. For simplicity, bodies are
expected to be `Vec<u8>`s so that the yare easy to work with. It's expected that JSON will be the main method by
which this library inputs and outputs data, and JSON doesn't stream well naturally, so this does not seem like a big
loss at present.