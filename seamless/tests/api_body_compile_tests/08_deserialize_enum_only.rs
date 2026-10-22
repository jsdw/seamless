use seamless::ApiBody;

#[ApiBody(Deserialize)]
enum Foo {
    Bar { n: usize }
}

fn main() {
    // This should fail to compile because deserialize not implemented
    let f = serde_json::to_string(&Foo::Bar{ n: 10 });
}