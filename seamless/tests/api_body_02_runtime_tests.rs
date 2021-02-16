use pretty_assertions::{ assert_eq };
use seamless::api::{ ApiBodyType, ApiBodyInfo };
use seamless::ApiBody;

macro_rules! map {
    ( $($key:expr => $val:expr),* ) => ({
        let mut m = std::collections::HashMap::new();
        $( m.insert($key, $val); )*
        m
    })
}

fn s(s: &str) -> String {
    s.to_owned()
}

#[test]
fn has_struct_shape() {

    #[ApiBody]
    #[allow(dead_code)]
    /// Foo comment
    struct Foo {
        /// Prop comment
        prop: usize,
        /// Another prop comment
        another_prop: bool
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s("Foo comment"),
            ty: ApiBodyType::Object {
                keys: map!{
                    s("prop") => ApiBodyInfo {
                        description: s("Prop comment"),
                        ty: ApiBodyType::Number
                    },
                    s("another_prop") => ApiBodyInfo {
                        description: s("Another prop comment"),
                        ty: ApiBodyType::Boolean

                    }
                }
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        Foo { prop: 2, another_prop: true }.to_json_value(),
        serde_json::json!({ "prop": 2, "another_prop": true })
    )
}

#[test]
fn has_enum_shape() {

    #[ApiBody]
    #[allow(dead_code)]
    enum Foo {
        /// Lark is larky
        Lark {
            /// Lark1
            lark1: String
        },
        /// Other is different
        Other(Other),
        AnotherOther(Other),
        /// Bar is empty
        Bar {}
    }

    #[ApiBody]
    #[allow(dead_code)]
    /// Other comes from here
    struct Other {
        other_prop: bool
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::OneOf { values:
                vec![
                    ApiBodyInfo {
                        description: s("Lark is larky"),
                        ty: ApiBodyType::Object { keys: map!{
                            s("kind") => ApiBodyInfo {
                                description: s("Variant tag"),
                                ty: ApiBodyType::StringLiteral { literal: s("Lark") }
                            },
                            s("lark1") => ApiBodyInfo {
                                description: s("Lark1"),
                                ty: ApiBodyType::String
                            }
                        }}
                    },
                    ApiBodyInfo {
                        description: s("Other is different"),
                        ty: ApiBodyType::Object { keys: map!{
                            s("kind") => ApiBodyInfo {
                                description: s("Variant tag"),
                                ty: ApiBodyType::StringLiteral { literal: s("Other") }
                            },
                            s("other_prop") => ApiBodyInfo {
                                description: s(""),
                                ty: ApiBodyType::Boolean
                            }
                        }}
                    },
                    ApiBodyInfo {
                        description: s("Other comes from here"),
                        ty: ApiBodyType::Object { keys: map!{
                            s("kind") => ApiBodyInfo {
                                description: s("Variant tag"),
                                ty: ApiBodyType::StringLiteral { literal: s("AnotherOther") }
                            },
                            s("other_prop") => ApiBodyInfo {
                                description: s(""),
                                ty: ApiBodyType::Boolean
                            }
                        }}
                    },
                    ApiBodyInfo {
                        description: s("Bar is empty"),
                        ty: ApiBodyType::Object { keys: map!{
                            s("kind") => ApiBodyInfo {
                                description: s("Variant tag"),
                                ty: ApiBodyType::StringLiteral { literal: s("Bar") }
                            }
                        }}
                    },
                ]
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        Foo::Lark { lark1: s("hi") }.to_json_value(),
        serde_json::json!({ "kind": "Lark", "lark1": "hi" })
    );
    assert_eq!(
        Foo::Other(Other { other_prop: true }).to_json_value(),
        serde_json::json!({ "kind": "Other", "other_prop": true })
    );
    assert_eq!(
        Foo::AnotherOther(Other { other_prop: true }).to_json_value(),
        serde_json::json!({ "kind": "AnotherOther", "other_prop": true })
    );
    assert_eq!(
        Foo::Bar{}.to_json_value(),
        serde_json::json!({ "kind": "Bar" })
    );

}

#[test]
fn has_enum_shape_unit() {
    #[ApiBody]
    #[allow(dead_code)]
    /// Foo help
    enum Foo {
        /// A help
        A,
        /// B help
        B,
        C
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s("Foo help"),
            ty: ApiBodyType::OneOf { values:
                vec![
                    ApiBodyInfo {
                        description: s("A help"),
                        ty: ApiBodyType::StringLiteral{ literal: s("A") }
                    },
                    ApiBodyInfo {
                        description: s("B help"),
                        ty: ApiBodyType::StringLiteral{ literal: s("B") }
                    },
                    ApiBodyInfo {
                        description: s(""),
                        ty: ApiBodyType::StringLiteral{ literal: s("C") }
                    },
                ]
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        Foo::A.to_json_value(),
        serde_json::json!("A")
    );
    assert_eq!(
        Foo::B.to_json_value(),
        serde_json::json!("B")
    );
    assert_eq!(
        Foo::C.to_json_value(),
        serde_json::json!("C")
    );

}

#[test]
fn delegates_to_inner() {

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo(Foo2);

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo2(Foo3);

    #[ApiBody]
    #[allow(dead_code)]
    /// Foo3 docs
    struct Foo3 {
        /// Hi!
        hi: usize
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s("Foo3 docs"),
            ty: ApiBodyType::Object { keys: map!{
                s("hi") => ApiBodyInfo {
                    description: s("Hi!"),
                    ty: ApiBodyType::Number
                }
            }}
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        Foo(Foo2(Foo3 { hi: 2 })).to_json_value(),
        serde_json::json!({ "hi": 2 })
    );
}

#[test]
fn delegates_to_inner2() {

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo(Foo2);

    #[ApiBody]
    #[allow(dead_code)]
    /// Foo2 docs
    struct Foo2(Foo3);

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo3 {
        /// Hi!
        hi: usize
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s("Foo2 docs"),
            ty: ApiBodyType::Object { keys: map!{
                s("hi") => ApiBodyInfo {
                    description: s("Hi!"),
                    ty: ApiBodyType::Number
                }
            }}
        }
    )
}

#[test]
fn delegates_to_inner3() {

    #[ApiBody]
    #[allow(dead_code)]
    /// Foo docs
    struct Foo(Foo2);

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo2(Foo3);

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo3 {
        /// Hi!
        hi: usize
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s("Foo docs"),
            ty: ApiBodyType::Object {
                keys: map!{
                    s("hi") => ApiBodyInfo {
                        description: s("Hi!"),
                        ty: ApiBodyType::Number
                    }
                }
            }
        }
    )
}

#[test]
fn flattens() {

    #[ApiBody]
    #[derive(Debug,PartialEq)]
    struct Foo {
        /// Hello docs
        hello: usize,
        #[api_body(flatten)]
        another: Bar
    }

    #[ApiBody]
    #[derive(Debug,PartialEq)]
    struct Bar {
        /// There docs
        there: bool,
        /// World docs
        world: String
    }

    let f = Foo::api_body_info();

    assert_eq!(f,
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::Object {
                keys: map!{
                    s("hello") => ApiBodyInfo {
                        description: s("Hello docs"),
                        ty: ApiBodyType::Number
                    },
                    s("there") => ApiBodyInfo {
                        description: s("There docs"),
                        ty: ApiBodyType::Boolean
                    },
                    s("world") => ApiBodyInfo {
                        description: s("World docs"),
                        ty: ApiBodyType::String
                    }
                }
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        Foo{ hello: 10, another: Bar{ there: true, world: s("w") } }.to_json_value(),
        serde_json::json!({ "hello": 10, "there": true, "world": "w" })
    );
    // ... and check that flattening works the other way around, too:
    assert_eq!(
        Foo::from_json_value(serde_json::json!({ "hello": 10, "there": true, "world": "w" })).unwrap(),
        Foo{ hello: 10, another: Bar{ there: true, world: s("w") } },
    );

}