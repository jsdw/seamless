#[seamless::ApiBody]
struct Foo(#[api_body(optional)] Option<usize>, bool);

fn main() {}
