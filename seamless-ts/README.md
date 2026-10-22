# seamless_ts

Generate a type safe TypeScript client for a [seamless](https://crates.io/crates/seamless) API.

This crate is `no_std` (it needs `alloc`). It exposes a single function, `generate_client`, which
takes the JSON produced by serializing the result of `seamless::Api::info()` and writes
TypeScript source code to any `core::fmt::Write` (eg a `String`) containing:

- A type for the response of each route (eg `UsersGetById`), and a type for its request body
  (eg `UsersGetByIdParams`) if it isn't a `GET` route.
- A `createApiClient` function which, given a function that knows how to make requests to the API,
  returns a client object with a typed method for each route, grouped by the dot or slash separated
  parts of the route name (eg `client.users.getById(params)` for the route `users.get_by_id`).

```rust,ignore
let info_json = serde_json::to_string(&api.info()).unwrap();
let mut typescript = String::new();
seamless_ts::generate_client(&info_json, &mut typescript).unwrap();
```

To generate a client from the command line, see the `seamless-ts` binary in the `seamless_ts_bin` crate.
