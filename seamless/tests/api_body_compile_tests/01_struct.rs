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

#[seamless::ApiBody]
/// The serde JSON types are supported.
struct SerdeJsonTypes {
    m: serde_json::Map<String, serde_json::Value>,
    n: serde_json::Number
}

fn main () {

}