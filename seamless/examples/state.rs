//! This builds on `basic.rs` a little to show how you can use the `HandlerParam` trait to
//! inject state into handlers.

use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ HandlerParam, body::{ Json } },
};

// Something we want to inject into our handler.
// In reality this might contain a database connection
// pool or configuration.
#[derive(Clone)]
struct State;

// Teach the library how to get hold of State when asked for it.
// This can inspect the request headers and path as well so that we
// can do things like load in a user based on a session cookie and complain
// with an ApiError if the user is not found.
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

#[tokio::main]
async fn main() {

    // Instantiate our API:
    //
    let mut api = Api::new();

    // Add some routes. Note that we can now ask for `State` as a parameter
    // to the handler.
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

}

/// We can use `seamless::ApiError` to easily allow an existing
/// enum or struct to be converted into an `ApiError`. Errors need to
/// implement the `Display` trait; we use `thiserror` to help with that
/// in this example.
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
    let a = input.a;
    let b = input.b;
    a.checked_div(b)
        .ok_or(MathsError::DivideByZero)
        .map(|result| BinaryOutput { a, b, result })
}
