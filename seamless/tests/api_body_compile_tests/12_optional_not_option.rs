#[seamless::ApiBody]
struct Foo {
    #[api_body(optional)]
    bar: usize,
}

fn main() {}
