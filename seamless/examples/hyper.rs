//! This is an example of how to integrate a `seamless` Api with `hyper`.
//!
//! Run this with `cargo run --example hyper` and then try:
//!
//! curl localhost:8000/api/echo -H 'content-type: application/json' -d '"hello"'
//! curl localhost:8000/api/reverse -H 'content-type: application/json' -d '[1,2,3,4,5]'
//! curl localhost:8000/api/count
//! curl localhost:8000/api/info
//!
//! To see the API in action. Any request whose path doesn't begin with `/api`
//! will get a 404 response.
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http::{Request, Response, StatusCode};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Frame, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use seamless::{
    api::{Api, RouteError},
    handler::{
        body::FromJson,
        request::Bytes as RequestBytes,
        response::{Body as ResponseBody, ToBinary, ToJson},
    },
};
use std::{convert::Infallible, sync::Arc};
use tokio::net::TcpListener;

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
    seamless_api
        .add("/api/count")
        .description("Stream back the numbers 1 to 100,000, one per line")
        .handler(|| {
            // This response is streamed back to the client one chunk at a time
            // rather than being built up in memory first.
            let lines = futures::stream::iter(1..=100_000)
                .map(|n| Ok::<_, std::io::Error>(format!("{n}\n")));
            ToBinary::stream(lines).content_type("text/plain")
        });

    // The API can hand back information about itself if we want:
    let info = seamless_api.info();
    seamless_api
        .add("/api/info")
        .description("Information about the other routes")
        .handler(move || ToJson(serde_json::to_value(&info).unwrap()));

    // Share the API across every connection we accept.
    let seamless_api = Arc::new(seamless_api);

    // Run our Hyper server, accepting connections in a loop and
    // serving each one on its own task.
    println!("Listening on 127.0.0.1:8000");
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Error accepting connection: {e}");
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let api = seamless_api.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| router(api.clone(), req));
            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("Error serving connection: {e}");
            }
        });
    }
}

/// The response body we hand to hyper. This lets us respond with either a
/// complete body or a streaming one.
type HyperBody = UnsyncBoxBody<Bytes, std::io::Error>;

/// Forward anything whose path begins with `/api` to seamless, and
/// return a 404 for everything else.
async fn router(
    api: Arc<Api>,
    request: Request<Incoming>,
) -> Result<Response<HyperBody>, Infallible> {
    if request.uri().path().starts_with("/api") {
        Ok(seamless_to_hyper(&api, request).await)
    } else {
        Ok(not_found())
    }
}

fn not_found() -> Response<HyperBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(full(Bytes::new()))
        .unwrap()
}

/// A complete (non-streaming) body.
fn full(bytes: Bytes) -> HyperBody {
    Full::new(bytes)
        .map_err(|never| match never {})
        .boxed_unsync()
}

/// Convert a seamless response body into a hyper one. Complete bodies are handed
/// over as-is, and streaming bodies are streamed through without being buffered.
fn to_hyper_body(body: ResponseBody) -> HyperBody {
    match body.into_bytes() {
        Ok(bytes) => full(bytes),
        Err(body) => StreamBody::new(body.into_stream().map_ok(Frame::data)).boxed_unsync(),
    }
}

async fn seamless_to_hyper(api: &Api, request: Request<Incoming>) -> Response<HyperBody> {
    let (parts, body) = request.into_parts();

    // Convert the body stream into a seamless-compatible format,
    // preserving the streaming.
    let body_stream = body.into_data_stream().map(|bytes| match bytes {
        Ok(bytes) => Ok(bytes.to_vec()),
        Err(e) => Err(std::io::Error::other(e)),
    });

    let body = RequestBytes::from_stream(body_stream);
    let request = Request::from_parts(parts, body);

    // Have our seamless API handle the request.
    api.handle(request).await.map_or_else(
        |e| match e {
            RouteError::NotFound(_) => not_found(),
            RouteError::Err(e) => Response::builder()
                .status(e.code)
                .body(full(Bytes::from(e.external_message)))
                .unwrap(),
        },
        |res| {
            let (parts, body) = res.into_parts();
            Response::from_parts(parts, to_hyper_body(body))
        },
    )
}
