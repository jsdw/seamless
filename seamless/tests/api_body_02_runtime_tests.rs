use pretty_assertions::{ assert_eq };
use seamless::body::{ ApiBody, Type, ApiBodyType };

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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s("Foo comment"),
            ty: Type::Object {
                keys: map!{
                    s("prop") => ApiBodyType {
                        description: s("Prop comment"),
                        ty: Type::Number
                    },
                    s("another_prop") => ApiBodyType {
                        description: s("Another prop comment"),
                        ty: Type::Boolean

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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s(""),
            ty: Type::OneOf { values:
                vec![
                    ApiBodyType {
                        description: s("Lark is larky"),
                        ty: Type::Object { keys: map!{
                            s("kind") => ApiBodyType {
                                description: s("Variant tag"),
                                ty: Type::StringLiteral { literal: s("Lark") }
                            },
                            s("lark1") => ApiBodyType {
                                description: s("Lark1"),
                                ty: Type::String
                            }
                        }}
                    },
                    ApiBodyType {
                        description: s("Other is different"),
                        ty: Type::Object { keys: map!{
                            s("kind") => ApiBodyType {
                                description: s("Variant tag"),
                                ty: Type::StringLiteral { literal: s("Other") }
                            },
                            s("other_prop") => ApiBodyType {
                                description: s(""),
                                ty: Type::Boolean
                            }
                        }}
                    },
                    ApiBodyType {
                        description: s("Other comes from here"),
                        ty: Type::Object { keys: map!{
                            s("kind") => ApiBodyType {
                                description: s("Variant tag"),
                                ty: Type::StringLiteral { literal: s("AnotherOther") }
                            },
                            s("other_prop") => ApiBodyType {
                                description: s(""),
                                ty: Type::Boolean
                            }
                        }}
                    },
                    ApiBodyType {
                        description: s("Bar is empty"),
                        ty: Type::Object { keys: map!{
                            s("kind") => ApiBodyType {
                                description: s("Variant tag"),
                                ty: Type::StringLiteral { literal: s("Bar") }
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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s("Foo help"),
            ty: Type::OneOf { values:
                vec![
                    ApiBodyType {
                        description: s("A help"),
                        ty: Type::StringLiteral{ literal: s("A") }
                    },
                    ApiBodyType {
                        description: s("B help"),
                        ty: Type::StringLiteral{ literal: s("B") }
                    },
                    ApiBodyType {
                        description: s(""),
                        ty: Type::StringLiteral{ literal: s("C") }
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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s("Foo3 docs"),
            ty: Type::Object { keys: map!{
                s("hi") => ApiBodyType {
                    description: s("Hi!"),
                    ty: Type::Number
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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s("Foo2 docs"),
            ty: Type::Object { keys: map!{
                s("hi") => ApiBodyType {
                    description: s("Hi!"),
                    ty: Type::Number
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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s("Foo docs"),
            ty: Type::Object {
                keys: map!{
                    s("hi") => ApiBodyType {
                        description: s("Hi!"),
                        ty: Type::Number
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

    let f = Foo::api_body_type();

    assert_eq!(f,
        ApiBodyType {
            description: s(""),
            ty: Type::Object {
                keys: map!{
                    s("hello") => ApiBodyType {
                        description: s("Hello docs"),
                        ty: Type::Number
                    },
                    s("there") => ApiBodyType {
                        description: s("There docs"),
                        ty: Type::Boolean
                    },
                    s("world") => ApiBodyType {
                        description: s("World docs"),
                        ty: Type::String
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