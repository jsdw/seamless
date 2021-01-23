use seamless::ApiBody;
use serde_json::json;

#[ApiBody(Serialize)]
enum Foo {
    Bar { n: usize }
}

fn main() {
    // This should fail to compile because deserialize not implemented
    let f: Foo = ApiBody::from_json_value(json!({
        "kind": "Bar",
        "n": 10
    })).unwrap();
}