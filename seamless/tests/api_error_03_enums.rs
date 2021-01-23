use seamless::{ ApiError, IntoApiError };

#[derive(ApiError)]
#[api_error(internal)]
enum Foo {
    A,
    #[api_error(external = "Hidden", code = 404)]
    B { message: String },
    #[api_error(external)]
    C,
    #[api_error(inner)]
    Delegated(Bar)
}
impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Foo::A => "a".to_owned(),
            Foo::B { message } => message.clone(),
            Foo::C => "c".to_owned(),
            Foo::Delegated (..) => "DELEGATED".to_owned()
        })
    }
}

#[derive(ApiError)]
#[api_error(external)]
struct Bar;

impl std::fmt::Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bar")
    }
}

#[test]
fn test_enum_a() {
    let a = Foo::A.into_api_error();
    assert_eq!(a.code, 500);
    assert_eq!(a.internal_message, "a".to_owned());
    assert_eq!(a.external_message, "Internal server error".to_owned());
}

#[test]
fn test_enum_b() {
    let a = Foo::B { message: "Custom".to_owned() }.into_api_error();
    assert_eq!(a.code, 404);
    assert_eq!(a.internal_message, "Custom".to_owned());
    assert_eq!(a.external_message, "Hidden".to_owned());
}

#[test]
fn test_enum_c() {
    let a = Foo::C.into_api_error();
    assert_eq!(a.code, 500);
    assert_eq!(a.internal_message, "c".to_owned());
    assert_eq!(a.external_message, "c".to_owned());
}

#[test]
fn test_enum_delegated() {
    let a = Foo::Delegated(Bar).into_api_error();
    assert_eq!(a.code, 500);
    assert_eq!(a.internal_message, "bar".to_owned());
    assert_eq!(a.external_message, "bar".to_owned());
}