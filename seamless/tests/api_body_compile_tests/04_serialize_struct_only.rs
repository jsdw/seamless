use seamless::ApiBody;
use serde_json::json;

#[ApiBody(Serialize)]
struct Foo {
    hello: usize
}

fn main() {
    // This should fail to compile because deserialize not implemented
    let f: Foo = serde_json::from_value(json!({
        "hello": 10
    })).unwrap();
}