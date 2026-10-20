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
    let _f: Foo = serde_json::from_value(json!({
        "hello": 10
    })).unwrap();
    let _f = serde_json::to_string(&Foo { hello: 10 });

    // both fine:
    let _f: Foo2 = serde_json::from_value(json!({
        "hello": 10
    })).unwrap();
    let _f = serde_json::to_string(&Foo2 { hello: 10 });
}