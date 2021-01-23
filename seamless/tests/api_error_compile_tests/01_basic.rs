// FAIL: attrs that we need weren't provided.

#[derive(seamless::ApiError)]
struct Foo {
    error: String
}

fn main() { }
