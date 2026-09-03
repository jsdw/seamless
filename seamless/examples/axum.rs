//! This is an example of how to integrate a `seamless` Api with `warp`.
//!
//! Run this with `cargo run --example warp` and then try:
//!
//! curl localhost:8000/api/echo -H 'content-type: application/json' -d '"hello"'
//! curl localhost:8000/api/reverse -H 'content-type: application/json' -d '[1,2,3,4,5]'
//!
//! To see the API in action.
use axum::{
    body::Body,
    extract::State,
    routing::{any, get},
    Router,
};
use futures::StreamExt;
use http::{Request, Response};
use seamless::{
    api::{Api, RouteError},
    handler::{body::FromJson, request::Bytes as RequestBytes, response::ToJson},
};
use std::sync::Arc;
use std::usize;

#[tokio::main]
async fn main() {
    // Define a simple seamless API
    let mut seamless_api = Api::new();

    seamless_api
        .add("/api/echo")
        .description("Echoes back a JSON string")
        .handler(|body: FromJson<String>| ToJson(body.0));
    seamless_api
        .add("/api/reverse")
        .description("Reverse an array of numbers")
        .handler(|body: FromJson<Vec<usize>>| {
            ToJson(body.0.into_iter().rev().collect::<Vec<usize>>())
        });

    // The API can hand back information about itself if we want:
    let info = seamless_api.info();
    seamless_api
        .add("/api/info")
        .description("Information about the other routes")
        .handler(move || ToJson(serde_json::to_value(&info).unwrap()));

    // Plug our seamless routes into an Axum router via `seamless_to_axum`.
    let app = Router::new()
        .route("/hello", get(hello))
        // Need to capture all of the path but can ignore the rest and let seamless handle it.
        .route(
            "/api/*rest",
            any(seamless_to_axum).with_state(Arc::new(seamless_api)),
        );

    // Run our Axum app.
    println!("Listening on 127.0.0.1:8000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "hello"
}

async fn seamless_to_axum(state: State<Arc<Api>>, request: Request<Body>) -> Response<Body> {
    let (parts, body) = request.into_parts();

    // Convert the body stream into a seamless-compatible format,
    // preserving the streaming.
    let body_stream = body.into_data_stream().map(|bytes| match bytes {
        Ok(bytes) => Ok(bytes.to_vec()),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    });

    let body = RequestBytes::from_stream(body_stream);
    let request = Request::from_parts(parts, body);

    // Have our seamless API handle the request.
    state.0.handle(request).await.map_or_else(
        |e| match e {
            RouteError::NotFound(_) => Response::builder().status(404).body(Body::empty()).unwrap(),
            RouteError::Err(e) => Response::builder()
                .status(e.code)
                .body(Body::from(e.external_message))
                .unwrap(),
        },
        |res| {
            let (parts, body_bytes) = res.into_parts();
            Response::from_parts(parts, Body::from(body_bytes))
        },
    )
}
