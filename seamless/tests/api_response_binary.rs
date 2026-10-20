use futures::StreamExt;
use seamless::{
    api::{Api, ApiBodyType, ApiError},
    handler::{
        request::Bytes,
        response::{Body, ToBinary, ToJson},
    },
    http::Request,
};

fn get(path: &str) -> Request<Bytes> {
    Request::get(path).body(Bytes::from_vec(vec![])).unwrap()
}

#[tokio::test]
async fn to_json_bodies_are_full() {
    let mut api = Api::new();
    api.add("json").handler(|| ToJson(vec![1, 2, 3]));

    let res = api.handle(get("/json")).await.unwrap();
    assert_eq!(res.headers()["content-type"], "application/json");
    assert_eq!(&*res.into_body().into_bytes().unwrap(), b"[1,2,3]");
}

#[tokio::test]
async fn to_binary_full_body() {
    let mut api = Api::new();
    api.add("bytes").handler(|| ToBinary::new(vec![1u8, 2, 3]));

    let res = api.handle(get("/bytes")).await.unwrap();
    assert_eq!(res.headers()["content-type"], "application/octet-stream");
    // The length is known up front, so it's set automatically:
    assert_eq!(res.headers()["content-length"], "3");
    assert_eq!(&*res.into_body().into_bytes().unwrap(), &[1, 2, 3]);
}

#[tokio::test]
async fn to_binary_streaming_body() {
    let mut api = Api::new();
    api.add("count").handler(|| async {
        let lines = futures::stream::iter(1..=3).map(|n| Ok::<_, std::io::Error>(format!("{n}\n")));
        ToBinary::stream(lines)
            .content_type("text/plain")
            .content_length(6)
    });

    let res = api.handle(get("/count")).await.unwrap();
    assert_eq!(res.headers()["content-type"], "text/plain");
    assert_eq!(res.headers()["content-length"], "6");

    // The body should come back as a stream, chunk by chunk:
    let body = res
        .into_body()
        .into_bytes()
        .expect_err("should be a streaming body");
    let chunks: Vec<_> = body.into_stream().map(|c| c.unwrap()).collect().await;
    assert_eq!(chunks, vec!["1\n", "2\n", "3\n"]);
}

#[tokio::test]
async fn to_binary_from_reader() {
    let mut api = Api::new();
    api.add("reader")
        .handler(|| ToBinary::reader(futures::io::Cursor::new(b"hello".to_vec())));

    let res = api.handle(get("/reader")).await.unwrap();
    assert_eq!(res.into_body().into_vec().await.unwrap(), b"hello");
}

#[tokio::test]
async fn to_binary_from_body() {
    let mut api = Api::new();
    api.add("body").handler(|| ToBinary::new(Body::from("hi")));

    let res = api.handle(get("/body")).await.unwrap();
    assert_eq!(res.into_body().into_vec().await.unwrap(), b"hi");
}

#[tokio::test]
async fn to_binary_errors_before_streaming_are_api_errors() {
    let mut api = Api::new();
    api.add("maybe")
        .handler(|| -> Result<ToBinary, ApiError> { Err(ApiError::path_not_found()) });
    api.add("none").handler(|| -> Option<ToBinary> { None });

    let err = api.handle(get("/maybe")).await.unwrap_err().unwrap_err();
    assert_eq!(err.code, 404);
    let err = api.handle(get("/none")).await.unwrap_err().unwrap_err();
    assert_eq!(err.code, 404);
}

#[tokio::test]
async fn to_binary_invalid_content_type_is_500() {
    let mut api = Api::new();
    api.add("bad")
        .handler(|| ToBinary::new("x").content_type("bad\nvalue"));

    let err = api.handle(get("/bad")).await.unwrap_err().unwrap_err();
    assert_eq!(err.code, 500);
}

#[test]
fn to_binary_info_is_binary() {
    let mut api = Api::new();
    api.add("bytes").handler(|| ToBinary::new(vec![1u8]));
    api.add("maybe_bytes")
        .handler(|| Some(ToBinary::new(vec![1u8])));

    for route in api.info() {
        assert_eq!(
            route.response_type.ty,
            ApiBodyType::Binary,
            "route {}",
            route.name
        );
    }
}
