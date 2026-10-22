#[seamless::ApiBody]
enum Foo {
    A {
        #[api_body(flatten)]
        bar: Bar,
    },
}

#[seamless::ApiBody]
struct Bar {
    lark: bool,
}

fn main() {}
