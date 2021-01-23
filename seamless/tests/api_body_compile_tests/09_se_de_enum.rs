use seamless::ApiBody;
use serde_json::json;

#[ApiBody(Serialize,Deserialize)]
enum Foo {
    Bar { n: usize }
}

#[ApiBody]
enum Foo2 {
    Bar { n: usize }
}

fn main() {
    // both compile fine:
    let _f: Foo = ApiBody::from_json_value(json!({
        "kind": "Bar",
        "n": 10
    })).unwrap();
    let _f = Foo::Bar{ n: 10 }.to_json_value();

    // both compile fine:
    let _f: Foo2 = ApiBody::from_json_value(json!({
        "kind": "Bar",
        "n": 10
    })).unwrap();
    let _f = Foo2::Bar{ n: 10 }.to_json_value();
}