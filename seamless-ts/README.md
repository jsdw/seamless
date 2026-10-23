# seamless_ts

Generate TypeScript types describing a [seamless](https://crates.io/crates/seamless) API.

This crate is `no_std` (it needs `alloc`). It exposes a single function, `generate_client`, which
takes the JSON produced by serializing the result of `seamless::Api::info()` and writes
TypeScript to any `core::fmt::Write` (eg a `String`). The TypeScript exports a single `Api` type,
which maps each route name to its HTTP method, request body type (`params`, unless it's a `GET`
route) and response type (`result`), and can be used to build a type safe client for the API:

```ts
export type Api = {
    'users.get_by_id': {
        method: 'POST'
        params: { id: string }
        result: { id: string, name: string }
    }
}
```

```rust,ignore
let info_json = serde_json::to_string(&api.info()).unwrap();
let mut typescript = String::new();
seamless_ts::generate_client(&info_json, &mut typescript).unwrap();
```

To generate these types from the command line, see the `seamless-ts` binary in the `seamless_ts_bin` crate.
