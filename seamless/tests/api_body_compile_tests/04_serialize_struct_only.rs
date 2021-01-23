use seamless::ApiBody;
use serde_json::json;

#[ApiBody(Serialize)]
struct Foo {
    hello: usize
}

fn main() {
    // This should fail to compile because deserialize not implemented
    let f: Foo = ApiBody::from_json_value(json!({
        "hello": 10
    })).unwrap();
}