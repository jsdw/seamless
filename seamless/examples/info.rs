//! This is a cut down/tweaked version of `basic.rs` to show that, given some API
//! routes, we can ask for information which is sufficient to construct type definitions
//! and such in something like TypeScript. The information knows about the shape of the
//! request and response types, as well as any doc comments added to the corresponding Rust
//! structs/enums.

use seamless::{
    api::{ Api, ApiBody, ApiError },
    handler::{ body::FromJson, response::ToJson },
};
use serde_json::json;

#[tokio::main]
async fn main() {

    // Instantiate our API:
    //
    let mut api = Api::new();

    // Add some routes:
    //
    api.add("maths/divide")
        .description("Divide two numbers by each other")
        .handler(|body: FromJson<_>| divide(body.0));

    // We can get hold of information about the routes we've added:
    //
    let info = api.info();

    // We can see that info contains any doc stricts added to types and fields,
    // as well as information about the shape of them:
    let expected = json!([
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
    assert_eq!(serde_json::to_value(info).unwrap(), expected);

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

async fn divide(input: BinaryInput) -> Result<ToJson<BinaryOutput>,MathsError> {
    let a = input.a;
    let b = input.b;
    a.checked_div(b)
        .ok_or(MathsError::DivideByZero)
        .map(|result| ToJson(BinaryOutput { a, b, result }))
}

// Make sure the example is valid when runnign cargo test
#[test]
fn test_main() {
    main()
}