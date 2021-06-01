//! This basic example shows that we can instantiate an API, add a few routes (which
//! are async and play nicely with regular async functions that know nothing about the API),
//! and then pass in `http::Request`s to use the API.

use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ body::{ FromJson }, response::ToJson },
    http::{ Request },
    stream::Bytes
};
use serde_json::{ Value, json };

#[tokio::main]
async fn main() {

    // Instantiate our API:
    //
    let mut api = Api::new();

    // Add some routes:
    //
    api.add("basic/echo")
        .description("Echoes back a JSON string")
        .handler(|FromJson(body)| ToJson::<String>(body));
    api.add("basic/reverse")
        .description("Reverse an array of numbers")
        .handler(|body: FromJson<Vec<usize>>| ToJson(body.0.into_iter().rev().collect::<Vec<usize>>()));

    // This route uses custom types (see below):
    api.add("meta/status")
        .description("Get the current API status")
        .handler(|| status().map(ToJson));

    // This route has an `async` handler (see below):
    api.add("maths/divide")
        .description("Divide two numbers by each other")
        .handler(|FromJson(body)| divide(body));


    // Now, we can handle incoming requests. Let's test a couple:
    //

    // Division..
    let req = Request::post("/maths/divide")
        .header("content-type", "application/json")
        .body(Bytes::from_vec(serde_json::to_vec(&BinaryInput { a: 20, b: 10 }).unwrap()))
        .unwrap();
    let actual: Value = serde_json::from_slice(&api.handle(req).await.unwrap().into_body()).unwrap();
    let expected = serde_json::to_value(json!({ "a": 20, "b": 10, "result": 2 })).unwrap();
    assert_eq!(actual, expected);

    // Division, hitting our error..
    let req = Request::post("/maths/divide")
        .header("content-type", "application/json")
        .body(Bytes::from_vec(serde_json::to_vec(&BinaryInput { a: 10, b: 0 }).unwrap()))
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

    // API status:
    let req = Request::get("/meta/status")
        .header("content-type", "application/json")
        .body(Bytes::from_vec(Vec::new()))
        .unwrap();
    let actual: Value =  serde_json::from_slice(&api.handle(req).await.unwrap().into_body()).unwrap();
    let expected = serde_json::to_value(json!({ "status": "Ok" })).unwrap();
    assert_eq!(actual, expected);

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
    a: usize,
    b: usize
}

/// Output containing the original input and result
#[ApiBody]
#[derive(PartialEq)]
struct BinaryOutput {
    a: usize,
    b: usize,
    result: usize
}

// We can have async handlers that return Results..
async fn divide(input: BinaryInput) -> Result<ToJson<BinaryOutput>,MathsError> {
    let a = input.a;
    let b = input.b;
    a.checked_div(b)
        .ok_or(MathsError::DivideByZero)
        .map(|result| ToJson(BinaryOutput { a, b, result }))
}

/// The API status
#[ApiBody]
struct Status {
    status: StatusValue
}

#[ApiBody]
enum StatusValue {
    Ok,
    NotOk
}

// ..or async/sync handlers that return Options..
fn status() -> Option<Status> {
    Some(Status {
        status: StatusValue::Ok
    })
}

// Make sure the example is valid when runnign cargo test
#[test]
fn test_main() {
    main()
}