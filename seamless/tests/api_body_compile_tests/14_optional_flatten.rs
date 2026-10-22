#[seamless::ApiBody]
struct Foo {
    #[api_body(flatten, optional)]
    bar: Option<Bar>,
}

#[seamless::ApiBody]
struct Bar {
    lark: bool,
}

fn main() {}
