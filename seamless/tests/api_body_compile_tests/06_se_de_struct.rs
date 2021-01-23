use seamless::ApiBody;
use serde_json::json;

#[ApiBody(Serialize,Deserialize)]
struct Foo {
    hello: usize
}

#[ApiBody]
struct Foo2 {
    hello: usize
}

fn main() {
    // both fine:
    let _f: Foo = ApiBody::from_json_value(json!({
        "hello": 10
    })).unwrap();
    let _f = Foo { hello: 10 }.to_json_value();

    // both fine:
    let _f: Foo2 = ApiBody::from_json_value(json!({
        "hello": 10
    })).unwrap();
    let _f = Foo2 { hello: 10 }.to_json_value();
}