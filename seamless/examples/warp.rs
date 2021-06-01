//! This is an example of how to integrate a `seamless` Api with `warp`.
//!
//! Run this with `cargo run --example warp` and then try:
//!
//! curl localhost:8000/api/echo -H 'content-type: application/json' -d '"hello"'
//! curl localhost:8000/api/reverse -H 'content-type: application/json' -d '[1,2,3,4,5]'
//!
//! To see the API in action.
use warp::Filter;
use warp::filters::BoxedFilter;
use bytes::{ Buf, Bytes }; 
use std::io::Read;
use std::sync::Arc;
use seamless::{
    api::{ Api, RouteError},
    handler::{ body::FromJson, response::ToJson },
    stream
};

#[tokio::main]
async fn main() {

    let mut api = Api::new();

    api.add("/api/echo")
        .description("Echoes back a JSON string")
        .handler(|body: FromJson<String>| ToJson(body.0));
    api.add("/api/reverse")
        .description("Reverse an array of numbers")
        .handler(|body: FromJson<Vec<usize>>| ToJson(body.0.into_iter().rev().collect::<Vec<usize>>()));

    let seamless_filter = to_warp_filter(api);

    let warp_api = warp::path("api").and(seamless_filter);

    warp::serve(warp_api)
        .bind("127.0.0.1:8000".parse::<std::net::SocketAddr>().unwrap())
        .await;

}

// We can write a wap filter that returns an `http::Request` given an incoming request:
pub fn extract_request() -> impl Filter<Extract=(http::Request<stream::Bytes>,), Error=warp::Rejection> + Copy {
    warp::method()
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(|method: http::Method, path: warp::path::FullPath, headers: http::HeaderMap, body: Bytes| {
            // Get our bytes into a vector. Unfortunately this isn't streaming,
            // because `warp::body::stream()` returns a non-Sendable unnamed impl
            // at the moment which is somewhat unergonomic to convert into a Sendable
            // Stream of bytes.
            let mut bytes: Vec<u8> = vec![];
            body.reader().read_to_end(&mut bytes).unwrap();

            // Build and return a request:
            let mut req = http::Request::builder()
                .method(method)
                .uri(path.as_str())
                .body(stream::Bytes::from_vec(bytes))
                .expect("request builder");
            { *req.headers_mut() = headers; }
            req
        })
}

// If we get back an error from a `seamless` api route, we wrap it in
// something that can be used as a custom warp Rejection. We'll want to
// handle this properly as we would any other warp rejection.
#[derive(Debug)]
struct SeamlessApiError(seamless::ApiError);
impl warp::reject::Reject for SeamlessApiError {}

// Now, we can use `extract_request` above to convert a `seamless::Api` into a
// warp filter like so:
pub fn to_warp_filter(api: seamless::Api) -> BoxedFilter<(impl warp::Reply,)> {
    let api = Arc::new(api);
    extract_request()
        .and_then(move |req: http::Request<stream::Bytes>| {
            let api = api.clone();
            async move {
                // In reality we should also check for the correct Content-Type and
                // such. Perhaps we'd do that here, or perhaps we'd chain this with
                // other warp filters.
                api.handle(req).await.map_err(|e| {
                    match e {
                        RouteError::NotFound(_) => warp::reject::not_found(),
                        RouteError::Err(e) => warp::reject::custom(SeamlessApiError(e))
                    }
                })
            }
        })
        .boxed()
}
