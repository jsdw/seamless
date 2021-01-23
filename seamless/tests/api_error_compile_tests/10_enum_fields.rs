#[derive(seamless::ApiError)]
#[api_error(internal)]
enum Foo {
    Bar(usize),
    Wibble{ size: String, other: usize }
}

// Normally we'd use thiserror or something:
impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "err")
    }
}

fn main() { }
