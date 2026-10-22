# seamless-info

The types describing a [seamless](https://crates.io/crates/seamless) API, as returned from `Api::info()`.

This crate is `no_std` (it needs `alloc`) and has no dependencies other than `serde`, which is only
pulled in if one of the following features is enabled:

- `serialize`: derive `serde::Serialize` on all types.
- `deserialize`: derive `serde::Deserialize` on all types.

`seamless` enables `serialize` so that it can hand back the API information. Enable `deserialize`
to read that information back in, for instance in order to generate an API client from it.
