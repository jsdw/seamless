use seamless::{
    api::{Api, RouteError},
    handler::{request::Bytes, response::ToJson},
    http::Request,
};

fn get(path: &str) -> Request<Bytes> {
    Request::get(path).body(Bytes::from_vec(vec![])).unwrap()
}

fn api_with_base(base: &str) -> Api {
    let mut api = Api::new_with_base_path(base);
    api.add("foo").handler(|| ToJson("foo"));
    api.add("v2/foo").handler(|| ToJson("v2/foo"));
    api
}

async fn found(api: &Api, path: &str) -> bool {
    match api.handle(get(path)).await {
        Ok(_) => true,
        Err(RouteError::NotFound(_)) => false,
        Err(RouteError::Err(e)) => panic!("unexpected handler error for {path}: {e:?}"),
    }
}

#[tokio::test]
async fn base_path_matches_whole_segments_only() {
    for base in ["/api", "api", "/api/", "api/"] {
        let api = api_with_base(base);
        assert!(found(&api, "/api/foo").await, "base {base:?}: /api/foo");
        assert!(
            found(&api, "/api/v2/foo").await,
            "base {base:?}: /api/v2/foo"
        );
        assert!(found(&api, "/api//foo").await, "base {base:?}: /api//foo");
        assert!(
            !found(&api, "/apiv2/foo").await,
            "base {base:?}: /apiv2/foo"
        );
        assert!(!found(&api, "/apifoo").await, "base {base:?}: /apifoo");
        assert!(!found(&api, "/foo").await, "base {base:?}: /foo");
    }
}

#[tokio::test]
async fn multi_segment_base_path() {
    let api = api_with_base("/a/b");
    assert!(found(&api, "/a/b/foo").await);
    assert!(!found(&api, "/a/bc/foo").await);
    assert!(!found(&api, "/a/foo").await);
}

#[tokio::test]
async fn empty_base_path_matches_everything() {
    for base in ["", "/"] {
        let api = api_with_base(base);
        assert!(found(&api, "/foo").await, "base {base:?}");
        assert!(found(&api, "/v2/foo").await, "base {base:?}");
    }
}
