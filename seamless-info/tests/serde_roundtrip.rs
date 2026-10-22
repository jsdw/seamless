#![cfg(all(feature = "serialize", feature = "deserialize"))]

use seamless_info::{ApiBodyInfo, ApiBodyType, RouteInfo};

fn info(description: &str, ty: ApiBodyType) -> ApiBodyInfo {
    ApiBodyInfo {
        description: description.to_owned(),
        ty,
    }
}

#[test]
fn route_info_roundtrips_through_json() {
    let route = RouteInfo {
        name: "maths/divide".to_owned(),
        method: "POST".to_owned(),
        description: "Divide two numbers".to_owned(),
        request_type: info(
            "Input",
            ApiBodyType::Object {
                keys: [
                    ("a".to_owned(), info("", ApiBodyType::Number)),
                    (
                        "b".to_owned(),
                        info(
                            "",
                            ApiBodyType::Optional {
                                value: Box::new(info("", ApiBodyType::Number)),
                            },
                        ),
                    ),
                ]
                .into_iter()
                .collect(),
            },
        ),
        response_type: info(
            "Output",
            ApiBodyType::OneOf {
                values: vec![
                    info(
                        "",
                        ApiBodyType::StringLiteral {
                            literal: "Nope".to_owned(),
                        },
                    ),
                    info(
                        "",
                        ApiBodyType::TupleOf {
                            values: vec![
                                info("", ApiBodyType::String),
                                info("", ApiBodyType::Null),
                            ],
                        },
                    ),
                    info(
                        "",
                        ApiBodyType::ArrayOf {
                            value: Box::new(info("", ApiBodyType::Binary)),
                        },
                    ),
                    info(
                        "",
                        ApiBodyType::ObjectOf {
                            value: Box::new(info("", ApiBodyType::Unknown)),
                        },
                    ),
                    info("", ApiBodyType::Any),
                    info("", ApiBodyType::Boolean),
                ],
            },
        ),
    };

    let json = serde_json::to_value(vec![route.clone()]).unwrap();
    let back: Vec<RouteInfo> = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(back, vec![route]);

    // Spot check the wire format:
    assert_eq!(json[0]["request_type"]["shape"]["type"], "Object");
    assert_eq!(
        json[0]["request_type"]["shape"]["keys"]["b"]["shape"]["type"],
        "Optional"
    );
}
