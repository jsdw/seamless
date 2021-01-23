use seamless::ApiBody;

#[ApiBody(Deserialize)]
enum Foo {
    Bar { n: usize }
}

fn main() {
    // This should fail to compile because deserialize not implemented
    let f = Foo::Bar{ n: 10 }.to_json_value();
}