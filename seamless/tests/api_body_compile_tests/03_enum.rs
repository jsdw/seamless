
#[seamless::ApiBody]
/// Barrrrrrr
enum Bar {
    /// Larkkk
    Lark { foo: String },
    /// barryyyy
    /// Is
    /// God!
    Barry { wibble: usize }
}

#[seamless::ApiBody]
#[api_body(tag = "internal_tag")]
enum Wibble {
    /// Larkkk
    Lark { foo: String },
    /// barryyyy
    /// Is
    /// God!
    Barry { wibble: usize },
    /// An inner struct is OK too:
    Inner(Inner)
}

#[seamless::ApiBody]
struct Inner {
    a: String,
    b: usize
}

fn main () {

}