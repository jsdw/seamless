#[seamless::ApiBody]
struct Foo {
    /// Hello there!
    hello: usize,
    /// Barry!
    #[serde(rename = "foo")]
    bar: String
}

fn main () {

}