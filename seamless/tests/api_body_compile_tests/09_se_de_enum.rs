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
    let _f: Foo = serde_json::from_value(json!({
        "kind": "Bar",
        "n": 10
    })).unwrap();
    let _f = serde_json::to_string(&Foo::Bar{ n: 10 });

    // both compile fine:
    let _f: Foo2 = serde_json::from_value(json!({
        "kind": "Bar",
        "n": 10
    })).unwrap();
    let _f = serde_json::to_string(&Foo2::Bar{ n: 10 });
}