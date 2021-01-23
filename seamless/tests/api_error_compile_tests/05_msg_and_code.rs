// PASS

#[derive(seamless::ApiError)]
#[api_error(external = "Foo", code = 123)]
struct Foo {
    error: String
}

// Normally we'd use thiserror or something:
impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

fn main() { }