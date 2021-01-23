// FAIL: attrs that we need weren't provided.

#[derive(seamless::ApiError)]
enum Foo {
    Bar,
    Wibble
}

fn main() { }
