type Maybe<T> = Option<T>;

#[seamless::ApiBody]
struct Foo {
    #[api_body(optional)]
    bar: Maybe<usize>,
}

fn main() {}
