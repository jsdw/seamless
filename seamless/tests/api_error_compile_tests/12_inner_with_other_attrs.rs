#[derive(seamless::ApiError)]
#[api_error(external)]
struct Inner;

impl std::fmt::Display for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inner")
    }
}

#[derive(seamless::ApiError)]
enum Foo {
    #[api_error(inner, code = 400)]
    Bar(Inner),
}

impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "err")
    }
}

fn main() { }
