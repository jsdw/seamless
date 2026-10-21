use seamless::{
    api::Api,
    handler::{body::FromJson, response::ToJson},
};

// Routes that share a path must come out of `info()` in a stable order
// (by name, then method), not in `HashMap` iteration order. `HashMap` is
// randomly seeded, so build the API many times to catch any instability.
#[test]
fn info_order_is_stable_for_same_path() {
    for _ in 0..50 {
        let mut api = Api::new();
        api.add("b").handler(|| ToJson("b"));
        api.add("a").handler(|body: FromJson<String>| ToJson(body.0));
        api.add("a").handler(|| ToJson("a"));
        api.add("c").handler(|body: FromJson<String>| ToJson(body.0));
        api.add("c").handler(|| ToJson("c"));

        let order: Vec<(String, String)> = api
            .info()
            .into_iter()
            .map(|r| (r.name, r.method))
            .collect();

        let expected: Vec<(String, String)> = [
            ("a", "GET"),
            ("a", "POST"),
            ("b", "GET"),
            ("c", "GET"),
            ("c", "POST"),
        ]
        .iter()
        .map(|(n, m)| (n.to_string(), m.to_string()))
        .collect();

        assert_eq!(order, expected);
    }
}
