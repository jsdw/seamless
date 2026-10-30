# seamless_ts

Generate TypeScript types describing a [seamless](https://crates.io/crates/seamless) API.

This crate is `no_std` (it needs `alloc`). It exposes a `generate` function, which takes the
result of `seamless::Api::info()` and writes TypeScript to any `core::fmt::Write` (eg a `String`), and a `generate_from_str` function, which does the same given the JSON produced by
serializing that info. The TypeScript exports an `Api` type, which
maps each route name to its HTTP method, request body type (`params`, which is `undefined` for `GET`
routes) and response type (`result`), and a readonly `methods` object mapping each route name to its
HTTP method for use at runtime. Together, these can be used to build a type safe client for the API:

```ts
export type Api = {
    'users.get_by_id': {
        method: 'POST'
        params: { id: string }
        result: { id: string, name: string }
    }
}

export const methods = {
    'users.get_by_id': 'POST',
} as const
```

```rust,ignore
let mut typescript = String::new();
seamless_ts::generate(api.info(), &mut typescript).unwrap();
```

To generate these types from the command line, see the `seamless-ts` binary in the `seamless_ts_bin` crate.
