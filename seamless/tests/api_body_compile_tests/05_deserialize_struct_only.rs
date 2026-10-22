use seamless::ApiBody;

#[ApiBody(Deserialize)]
struct Foo {
    hello: usize
}

fn main() {
    // This should fail to compile because serialize not implemented
    let _f = serde_json::to_string(&Foo { hello: 10 }).unwrap();
}