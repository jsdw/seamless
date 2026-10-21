use pretty_assertions::assert_eq;
use seamless::api::{ApiBodyInfo, ApiBodyType};
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
        another_prop: bool,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s("Foo comment"),
            ty: ApiBodyType::Object {
                keys: map! {
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
        to_json_value(Foo {
            prop: 2,
            another_prop: true
        }),
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
            lark1: String,
        },
        /// Other is different
        Other(Other),
        AnotherOther(Other),
        /// Bar is empty
        Bar {},
    }

    #[ApiBody]
    #[allow(dead_code)]
    /// Other comes from here
    struct Other {
        other_prop: bool,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::OneOf {
                values: vec![
                    ApiBodyInfo {
                        description: s("Lark is larky"),
                        ty: ApiBodyType::Object {
                            keys: map! {
                                s("kind") => ApiBodyInfo {
                                    description: s("Variant tag"),
                                    ty: ApiBodyType::StringLiteral { literal: s("Lark") }
                                },
                                s("lark1") => ApiBodyInfo {
                                    description: s("Lark1"),
                                    ty: ApiBodyType::String
                                }
                            }
                        }
                    },
                    ApiBodyInfo {
                        description: s("Other is different"),
                        ty: ApiBodyType::Object {
                            keys: map! {
                                s("kind") => ApiBodyInfo {
                                    description: s("Variant tag"),
                                    ty: ApiBodyType::StringLiteral { literal: s("Other") }
                                },
                                s("other_prop") => ApiBodyInfo {
                                    description: s(""),
                                    ty: ApiBodyType::Boolean
                                }
                            }
                        }
                    },
                    ApiBodyInfo {
                        description: s("Other comes from here"),
                        ty: ApiBodyType::Object {
                            keys: map! {
                                s("kind") => ApiBodyInfo {
                                    description: s("Variant tag"),
                                    ty: ApiBodyType::StringLiteral { literal: s("AnotherOther") }
                                },
                                s("other_prop") => ApiBodyInfo {
                                    description: s(""),
                                    ty: ApiBodyType::Boolean
                                }
                            }
                        }
                    },
                    ApiBodyInfo {
                        description: s("Bar is empty"),
                        ty: ApiBodyType::Object {
                            keys: map! {
                                s("kind") => ApiBodyInfo {
                                    description: s("Variant tag"),
                                    ty: ApiBodyType::StringLiteral { literal: s("Bar") }
                                }
                            }
                        }
                    },
                ]
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        to_json_value(Foo::Lark { lark1: s("hi") }),
        serde_json::json!({ "kind": "Lark", "lark1": "hi" })
    );
    assert_eq!(
        to_json_value(Foo::Other(Other { other_prop: true })),
        serde_json::json!({ "kind": "Other", "other_prop": true })
    );
    assert_eq!(
        to_json_value(Foo::AnotherOther(Other { other_prop: true })),
        serde_json::json!({ "kind": "AnotherOther", "other_prop": true })
    );
    assert_eq!(
        to_json_value(Foo::Bar {}),
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
        C,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s("Foo help"),
            ty: ApiBodyType::OneOf {
                values: vec![
                    ApiBodyInfo {
                        description: s("A help"),
                        ty: ApiBodyType::StringLiteral { literal: s("A") }
                    },
                    ApiBodyInfo {
                        description: s("B help"),
                        ty: ApiBodyType::StringLiteral { literal: s("B") }
                    },
                    ApiBodyInfo {
                        description: s(""),
                        ty: ApiBodyType::StringLiteral { literal: s("C") }
                    },
                ]
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(to_json_value(Foo::A), serde_json::json!("A"));
    assert_eq!(to_json_value(Foo::B), serde_json::json!("B"));
    assert_eq!(to_json_value(Foo::C), serde_json::json!("C"));
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
        hi: usize,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s("Foo3 docs"),
            ty: ApiBodyType::Object {
                keys: map! {
                    s("hi") => ApiBodyInfo {
                        description: s("Hi!"),
                        ty: ApiBodyType::Number
                    }
                }
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        to_json_value(Foo(Foo2(Foo3 { hi: 2 }))),
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
        hi: usize,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s("Foo2 docs"),
            ty: ApiBodyType::Object {
                keys: map! {
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
        hi: usize,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s("Foo docs"),
            ty: ApiBodyType::Object {
                keys: map! {
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
    #[derive(Debug, PartialEq)]
    struct Foo {
        /// Hello docs
        hello: usize,
        #[api_body(flatten)]
        another: Bar,
    }

    #[ApiBody]
    #[derive(Debug, PartialEq)]
    struct Bar {
        /// There docs
        there: bool,
        /// World docs
        world: String,
    }

    let f = Foo::api_body_info();

    assert_eq!(
        f,
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::Object {
                keys: map! {
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
        to_json_value(Foo {
            hello: 10,
            another: Bar {
                there: true,
                world: s("w")
            }
        }),
        serde_json::json!({ "hello": 10, "there": true, "world": "w" })
    );
    // ... and check that flattening works the other way around, too:
    assert_eq!(
        serde_json::from_value::<Foo>(
            serde_json::json!({ "hello": 10, "there": true, "world": "w" })
        ).unwrap(),
        Foo {
            hello: 10,
            another: Bar {
                there: true,
                world: s("w")
            }
        },
    );
}


#[test]
fn raw_identifiers_use_serde_names() {
    #[ApiBody]
    #[allow(dead_code, non_camel_case_types)]
    enum Foo {
        r#match {
            r#in: bool,
        },
    }

    #[ApiBody]
    #[allow(dead_code)]
    struct Bar {
        r#type: String,
    }

    assert_eq!(
        Foo::api_body_info(),
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::OneOf {
                values: vec![ApiBodyInfo {
                    description: s(""),
                    ty: ApiBodyType::Object {
                        keys: map! {
                            s("kind") => ApiBodyInfo {
                                description: s("Variant tag"),
                                ty: ApiBodyType::StringLiteral { literal: s("match") }
                            },
                            s("in") => ApiBodyInfo {
                                description: s(""),
                                ty: ApiBodyType::Boolean
                            }
                        }
                    }
                }]
            }
        }
    );

    assert_eq!(
        Bar::api_body_info(),
        ApiBodyInfo {
            description: s(""),
            ty: ApiBodyType::Object {
                keys: map! {
                    s("type") => ApiBodyInfo {
                        description: s(""),
                        ty: ApiBodyType::String
                    }
                }
            }
        }
    );

    // Sanity check that serde outputs a format which aligns with expectation:
    assert_eq!(
        to_json_value(Foo::r#match { r#in: true }),
        serde_json::json!({ "kind": "match", "in": true })
    );
    assert_eq!(
        to_json_value(Bar { r#type: s("t") }),
        serde_json::json!({ "type": "t" })
    );
}

#[test]
fn option_is_nullable_by_default() {
    #[ApiBody]
    #[allow(dead_code)]
    struct Foo {
        /// Maybe a number
        a: Option<usize>,
        b: Vec<Option<bool>>,
    }

    #[ApiBody]
    #[allow(dead_code)]
    struct Bar(Option<usize>);

    assert_eq!(
        Foo::api_body_info(),
        info(ApiBodyType::Object {
            keys: map! {
                s("a") => ApiBodyInfo {
                    description: s("Maybe a number"),
                    ..nullable(ApiBodyType::Number)
                },
                s("b") => info(ApiBodyType::ArrayOf {
                    value: Box::new(nullable(ApiBodyType::Boolean))
                })
            }
        })
    );
    assert_eq!(Bar::api_body_info(), nullable(ApiBodyType::Number));
    assert_eq!(
        <Option<String>>::api_body_info(),
        nullable(ApiBodyType::String)
    );

    // `None` is serialized as `null`, matching the `T | null` description:
    assert_eq!(
        to_json_value(Foo {
            a: None,
            b: vec![Some(true), None]
        }),
        serde_json::json!({ "a": null, "b": [true, null] })
    );
    assert_eq!(to_json_value(Bar(None)), serde_json::json!(null));
}

#[test]
fn optional_struct_fields() {
    type Maybe<T> = Option<T>;

    #[ApiBody]
    #[allow(dead_code)]
    struct Foo {
        /// Maybe a number
        #[api_body(optional)]
        a: Option<usize>,
        #[api_body(optional)]
        b: std::option::Option<String>,
        #[api_body(optional)]
        c: Option<Option<bool>>,
        d: Maybe<bool>,
        #[api_body(optional)]
        e: Maybe<bool>,
    }

    assert_eq!(
        Foo::api_body_info(),
        info(ApiBodyType::Object {
            keys: map! {
                s("a") => ApiBodyInfo {
                    description: s("Maybe a number"),
                    ..optional(info(ApiBodyType::Number))
                },
                s("b") => optional(info(ApiBodyType::String)),
                s("c") => optional(nullable(ApiBodyType::Boolean)),
                s("d") => nullable(ApiBodyType::Boolean),
                s("e") => optional(info(ApiBodyType::Boolean))
            }
        })
    );

    // Marked fields are omitted when `None`, others are `null`:
    assert_eq!(
        to_json_value(Foo {
            a: None,
            b: None,
            c: None,
            d: None,
            e: None
        }),
        serde_json::json!({ "d": null })
    );
    assert_eq!(
        to_json_value(Foo {
            a: Some(1),
            b: Some(s("hi")),
            c: Some(None),
            d: Some(true),
            e: Some(false)
        }),
        serde_json::json!({ "a": 1, "b": "hi", "c": null, "d": true, "e": false })
    );

    // Missing keys and `null`s both deserialize to `None`:
    let foo: Foo = serde_json::from_value(serde_json::json!({ "b": null, "d": null })).unwrap();
    assert_eq!(
        (foo.a, foo.b, foo.c, foo.d, foo.e),
        (None, None, None, None, None)
    );
}

#[test]
fn optional_enum_fields() {
    #[ApiBody]
    #[allow(dead_code)]
    enum Foo {
        A {
            #[api_body(optional)]
            a: Option<usize>,
            b: Option<usize>,
        },
    }

    assert_eq!(
        Foo::api_body_info(),
        info(ApiBodyType::OneOf {
            values: vec![info(ApiBodyType::Object {
                keys: map! {
                    s("kind") => ApiBodyInfo {
                        description: s("Variant tag"),
                        ty: ApiBodyType::StringLiteral { literal: s("A") }
                    },
                    s("a") => optional(info(ApiBodyType::Number)),
                    s("b") => nullable(ApiBodyType::Number)
                }
            })]
        })
    );

    assert_eq!(
        to_json_value(Foo::A { a: None, b: None }),
        serde_json::json!({ "kind": "A", "b": null })
    );
    assert_eq!(
        to_json_value(Foo::A {
            a: Some(1),
            b: Some(2)
        }),
        serde_json::json!({ "kind": "A", "a": 1, "b": 2 })
    );
    let Foo::A { a, b } =
        serde_json::from_value(serde_json::json!({ "kind": "A", "b": null })).unwrap();
    assert_eq!((a, b), (None, None));
}

#[test]
fn optional_deserialize_only() {
    #[ApiBody(Deserialize)]
    #[allow(dead_code)]
    struct Foo {
        #[api_body(optional)]
        a: Option<usize>,
        b: bool,
    }

    assert_eq!(
        Foo::api_body_info(),
        info(ApiBodyType::Object {
            keys: map! {
                s("a") => optional(info(ApiBodyType::Number)),
                s("b") => info(ApiBodyType::Boolean)
            }
        })
    );

    // The key can be omitted or null:
    let foo: Foo = serde_json::from_value(serde_json::json!({ "b": true })).unwrap();
    assert_eq!(foo.a, None);
    let foo: Foo = serde_json::from_value(serde_json::json!({ "a": null, "b": true })).unwrap();
    assert_eq!(foo.a, None);
    let foo: Foo = serde_json::from_value(serde_json::json!({ "a": 1, "b": true })).unwrap();
    assert_eq!(foo.a, Some(1));
}

fn info(ty: ApiBodyType) -> ApiBodyInfo {
    ApiBodyInfo {
        description: s(""),
        ty,
    }
}

fn nullable(ty: ApiBodyType) -> ApiBodyInfo {
    info(ApiBodyType::OneOf {
        values: vec![info(ty), info(ApiBodyType::Null)],
    })
}

fn optional(ty: ApiBodyInfo) -> ApiBodyInfo {
    info(ApiBodyType::Optional {
        value: Box::new(ty),
    })
}

fn to_json_value(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value)
        .expect("Failed to serialize to JSON due to an invalid manual implementation (2)")
}