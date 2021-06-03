//! This builds on `basic.rs` a little to show how you can use the `HandlerParam` trait to
//! inject state into handlers.

use seamless::{
    api::{ Api, ApiError },
    handler::{ HandlerParam, body::FromJson, request::Bytes, response::ToJson },
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
    api.add("echo")
        .description("Echo back the string provided")
        .handler(|_state: State, body: FromJson<String>| ToJson(body.0));


    // When passing a request into our API, remember to inject `State` so that
    // it's available for our `HandlerParam` trait to extract:
    let mut req = http::Request::post("/echo")
        .header("content-type", "application/json")
        .body(Bytes::from_vec(serde_json::to_vec("hello").unwrap()))
        .unwrap();

    req.extensions_mut().insert(State);

    // We can now handle the request without issues:
    assert!(api.handle(req).await.is_ok());

}

// Make sure the example is valid when runnign cargo test
#[test]
fn test_main() {
    main()
}