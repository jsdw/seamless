use seamless::ApiBody;

#[ApiBody(Deserialize)]
struct Foo {
    hello: usize
}

fn main() {
    // This should fail to compile because serialize not implemented
    let f = Foo { hello: 10 }.to_json_value();
}