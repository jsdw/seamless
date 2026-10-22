use seamless::{
    api::{Api, RouteError},
    handler::{body::FromJson, request::Bytes, response::ToJson},
    http::Request,
};

fn post_json(content_type: &str) -> Request<Bytes> {
    Request::post("/echo")
        .header("content-type", content_type)
        .body(Bytes::from_vec(b"[1,2,3]".to_vec()))
        .unwrap()
}

fn echo_api() -> Api {
    let mut api = Api::new();
    api.add("echo")
        .handler(|body: FromJson<Vec<u64>>| ToJson(body.0));
    api
}

#[tokio::test]
async fn json_content_type_with_charset_is_accepted() {
    let api = echo_api();
    for ct in [
        "application/json",
        "application/json; charset=utf-8",
        "Application/JSON;charset=UTF-8",
        "application/json ; charset=utf-8",
    ] {
        let res = api
            .handle(post_json(ct))
            .await
            .unwrap_or_else(|e| panic!("{ct:?} was rejected: {e:?}"));
        assert_eq!(
            &*res.into_body().into_bytes().unwrap(),
            b"[1,2,3]",
            "{ct:?}"
        );
    }
}

#[tokio::test]
async fn non_json_content_type_is_415() {
    let api = echo_api();
    for ct in [
        "text/plain",
        "application/jsonx",
        "text/plain; x=application/json",
    ] {
        match api.handle(post_json(ct)).await {
            Err(RouteError::Err(e)) => assert_eq!(e.code, 415, "{ct:?}"),
            Err(RouteError::NotFound(..)) => panic!("{ct:?}: route not found"),
            Ok(_) => panic!("{ct:?} should have been rejected"),
        }
    }
}
