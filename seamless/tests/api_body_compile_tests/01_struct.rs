#[seamless::ApiBody]
struct Foo {
    /// Hello there!
    hello: usize,
    /// Barry!
    bar: Bar
}

#[seamless::ApiBody]
/// Barrrrrrr
struct Bar {
    /// A prop about larking
    lark: bool,
    another: String
}

fn main () {

}