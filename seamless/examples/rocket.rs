//! This is an example of how to integrate a `seamless` Api with `rocket` (0.5.0).
//!
//! Run this with `cargo run --example rocket` and then try:
//!
//! curl localhost:8000/api/echo -H 'content-type: application/json' -d '"hello"'
//! curl localhost:8000/api/reverse -H 'content-type: application/json' -d '[1,2,3,4,5]'
//!
//! To see the API in action (assuming port 8000).
use rocket::{Request, Data, Route, http::{ Method, Status }};
use rocket::handler::{ Handler, Outcome };
use rocket::data::ToByteUnit;
use http::header::HeaderName;
use std::io::Cursor;
use std::sync::Arc;
use tokio_util::compat::TokioAsyncReadCompatExt; 
use seamless::{
    api::{ Api, RouteError },
    handler::{ body::FromJson, request::Bytes, response::ToJson },
};

#[rocket::launch]
fn rocket() -> rocket::Rocket {

    let mut api = Api::new();

    api.add("/api/echo")
        .description("Echoes back a JSON string")
        .handler(|body: FromJson<String>| ToJson(body.0));
    api.add("/api/reverse")
        .description("Reverse an array of numbers")
        .handler(|body: FromJson<Vec<usize>>| ToJson(body.0.into_iter().rev().collect::<Vec<usize>>()));

    rocket::ignite().mount("/", SeamlessApi(Arc::new(api)))
}

// Wrap our `seamless::Api` in a thing that Rocket can work with:
#[derive(Clone)]
struct SeamlessApi(Arc<Api>);

#[rocket::async_trait]
impl Handler for SeamlessApi {
    async fn handle<'r, 's: 'r>(&'s self, req: &'r Request<'_>, data: Data) -> Outcome<'r> {

        // Stream the body into `seamless`. We use the `compat` method from tokio-utils to 
        // convert from `tokio::AsyncRead` to the `futures::AsyncRead` that seamless
        // works with. We could alternately obtain a vector of bytes here, but by streaming
        // it into seamless, we can do things like configuring per-request size limits, 
        // immediately terminating the streaming if reached.
        let body_reader = data.open(10.megabytes()).compat();
        let streamed_body = Bytes::from_reader(body_reader);

        // Build an http::Request:
        let mut http_req = http::Request::builder()
            .method(req.method().as_str())
            .uri(req.uri().path())
            .body(streamed_body)
            .unwrap();

        // Copy headers over:
        let new_headers = http_req.headers_mut();
        for header in req.headers().iter() {
            let header_name = HeaderName::from_lowercase(header.name().to_string().to_lowercase().as_bytes());
            if let Ok(header_name) = header_name {
                new_headers.insert(header_name, header.value().parse().unwrap());
            }
        }

        // Give this to `seamless` and then tell Rocket how to
        // handle the result:
        match self.0.handle(http_req).await {
            Ok(res) => {
                let response_body = res.into_body();
                let rocket_response = rocket::Response::build()
                    .header(rocket::http::ContentType::JSON)
                    .sized_body(response_body.len(), Cursor::new(response_body))
                    .finalize();
                Outcome::Success(rocket_response)
            },
            Err(RouteError::NotFound(_req)) => {
                Outcome::failure(Status::NotFound)
            },
            Err(RouteError::Err(e)) => {
                eprintln!("Whoops: {:?}", e);
                Outcome::failure(Status::InternalServerError)
            }
        }
    }
}

impl Into<Vec<Route>> for SeamlessApi {
    fn into(self) -> Vec<Route> {
        // Show rocket what routes exist in our API
        // by inspecting the api info:
        self.0.info().into_iter().map(|r| {
            let method = match r.method.as_str() {
                "GET" => Method::Get,
                _ => Method::Post
            };
            Route::new(method, format!("/{}", r.name), self.clone())
        }).collect()
    }
}
