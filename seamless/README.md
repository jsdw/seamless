# Seamless

[API Docs](https://docs.rs/seamless/latest/seamless)

An opinionated library to easily plug RPC style JSON APIs into your existing HTTP framework to enable
type safe communication with TypeScript (or similar) clients.

The main USP of this library is that it takes advantage of trait and macro magic to automatically infer
the shape of the API (paths, descriptions, and the types of request and response for each route) from
just the Rust code you've written, negating the need for external definitions like OpenAPI.

# Pros & Cons

Seamless is a library primarily designed to facilitate communication between a Rust backend
and a TypeScript (or similar) client via JSON. By using this library you get:
- The ablity to use any async framework of your choice without feature flags and such.
- A self describing API that can automatically provide back enough information to generate
  a fully typed client in a language like TypeScript. This leans on a [`macro@ApiBody`] macro
  which is placed on structs/enums you'd like to receive or return from the API, along with trait
  magic.
- Consistent error handling: You can return whatever domain specific errors you like from handlers,
  so long as they implement `Into<ApiError>`. The provided [`macro@ApiError`] macro makes this simple.
- The ability to pull in state or guard requests using the [`handler::HandlerParam`] trait. With this
  trait, handlers can ask for whatever parameters they need, and know that they won't run if those
  parameters cannot be obtained (for example, an invalid user session was provided).

This library also has limitations, some of them being:
- Streaming of request and response bodies is not supported. Currently the library doesn't expose
  means to stream data in and our of handlers for the sake of simplicity (instead, everything comes in
  and leaves as a `Vec<u8>`). This is simple to use, but large data transfers should happen
  outside of this library.
- Type information from the [`Api::info()`] method is tuned towards generating TypeScript client
  code, and cannot provide enough detail to generate, for example, a well typed Rust client.
- No support for more complex URL matching (eg to extract query params). I don't intend to support this
  use case. Keeping parameters in the body allows us to type them properly; this would be much more
  difficult to do with query params. Think of this library as more RPC, less REST.

# Example

Below is a basic self contained example of using this library. Please have a look in the `examples`
folder for more detailed examples.

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

/// Provide two numbers to get back the division of them.
#[ApiBody]
struct DivisionInput {
    a: usize,
    b: usize
}

/// The division of two numbers `a` and `b`.
#[ApiBody]
#[derive(PartialEq)]
struct DivisionOutput {
    a: usize,
    b: usize,
    /// The division of the first and second number
    result: usize
}

// We can use `seamless::ApiError` to easily allow an existing
// enum or struct to be returnable from the API if things go wrong.
// `ApiError`s must implement `Debug` and `Display`. We use `thiserror`
// here to easily implement Display.
#[derive(ApiError, Debug, thiserror::Error, PartialEq)]
enum MathsError {
    #[error("Division by zero")]
    #[api_error(external, code=400)]
    DivideByZero
}

// We instantiate an API and add routes to it like so. The handler function would
// often be an external `async fn foo()` defined elsewhere (see the examples), but
// for the sake of this example we define it inline.
let mut api = Api::new();

api.add("maths.divide")
   .description("Divide two numbers by each other")
   .handler(|body: Json<DivisionInput>| async move {
       let a = body.json.a;
       let b = body.json.b;
       a.checked_div(b)
           .ok_or(MathsError::DivideByZero)
           .map(|result| DivisionOutput { a, b, result })
   });

// Once we've added routes to the `api`, we use it by sending `http::Request`s to it.
// Below, we give the API a quick test and assert that we get back what we expect when
// we do this:

let req = Request::post("/maths.divide")
   .body(serde_json::to_vec(&DivisionInput { a: 20, b: 10 }).unwrap())
   .unwrap();
assert_eq!(
    api.handle(req).await.unwrap().into_body(),
    serde_json::to_vec(&DivisionOutput{ a: 20, b: 10, result: 2 }).unwrap()
);

let req = Request::post("/maths.divide")
   .body(serde_json::to_vec(&DivisionInput { a: 10, b: 0 }).unwrap())
   .unwrap();
assert_eq!(
    api.handle(req).await.unwrap_err().unwrap_err(),
    ApiError {
        code: 400,
        internal_message: "Division by zero".to_owned(),
        external_message: "Division by zero".to_owned(),
        value: None
    }
);
# });
```

# State

Most real life use cases will require some sort of state to be accessible inside a handler.


This library follows an approach a little similar to `Rocket`. Any type that implements the
[`handler::HandlerParam`] trait can be passed into handler functions. To pass state in, you can
inject it into the `http::Request` prior to handing it to this library, and then extract it out
of the request again in the [`handler::HandlerParam`] implementation.

**Note**: params implementing the `RequestParam` trait must come before the one that implements
`RequestBody` (if any) in the handler function argument list.

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
api.add("maths/divide")
    .description("Divide two numbers by each other")
    .handler(|_state: State, body: Json<_>| divide(body.json));

// When passing a request into our API, remember to inject `State` so that
// it's available for our `HandlerParam` trait to extract:
let mut req = http::Request::post("/maths/divide")
    .body(serde_json::to_vec(&BinaryInput { a: 20, b: 10 }).unwrap())
    .unwrap();

req.extensions_mut().insert(State);

// We can now handle the request without issues:
assert!(api.handle(req).await.is_ok());
# })
```
