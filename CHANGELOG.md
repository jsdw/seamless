# 0.10.0

- Add `Unknown` and `Binary` types. The former corresponds to `unknown` in TypeScript, and the latter probably corresponds to `Blob` (depending on your environment) but is a sort-fo special case that can be useful to support.

# 0.9.0

- Update to 2021 edition.
- impl `ApiBody` for `serde_json::Map<String,T>` and `serde_json::Number`.
- ensure Seamless points to its re-export of `serde` in the macro so that `serde` not a required dep in projects using Seamless.

# v0.8.0

- Accept request bodies which are `futures::AsyncRead + Send + Unpin` rather than `Vec<u8>` to allow streaming data into seamless endpoints. Code passing in `Vec<u8>` can be trivially updated to instead take `seamless::handler::request::Bytes::from_vec(body)`, although optimally you'd make better use of the new streaming capabilities if interested in doing so.
- Introduce supporting code for the above, as well as a `Capped` struct which can wrap things like `FromJson` to impose request body byte limits (using const generics) on specific endpoints.

# v0.7.2

- Fix bug which made it hard to return a valid value from a handler function that didn't accept a body.

# v0.7.1

- Minor doc fix.

# v0.7.0

- `handler::body::Json` and `handler::body::Binary` renamed to `handler::body::FromJson` and `handler::body::FromBinary`.
- `handler::response::HandlerResponse` altered so that it is in a better state to be implemented by others, and takes on more of the responsibility of deciding what the response should look like (no hardcoded JSON assumption any more).
- Handlers can now return anything implementing `handler::response::HandlerResponse` (sync or async).

# v0.6.0

- Require 'content-type: application/json' when requestion a `Json<_>` body in a handler.
- Add more documentation around integrating, and more doc improvements.

# v0.5.1

- Document getting API info. No breaking changes.

# v0.5.0

- `RequestParam` and `RequestBody` renamed to `HandlerParam` and `HandlerBody`, and their methods renamed following a similar convention.
- Handler functions can now be either async or non-async, and return `Result`s or `Option`s.
- Doc and example improvements around state.

# v0.4.0

- Rename `RequestParam`'s `get_param` to `request_param`.
- A couple of minor tweaks and trait impls.
- Add more documentation.

# v0.3.0

- Rename and move things around so that they hopefully make more sense (see examples and such for specifics).
- Remove the dependency on `ApiError` from various traits.
- Tidy up export hierarchy.

# v0.2.0

- Allow the expected HTTP method for a route to be configured by implementors of the `Body` trait. (previously it was always POST if a `Body` was provided, else GET).
- Remove `IntoApiError` and implement `From<T>` as appropriate instead.
- Remove `Method` enum and use `http::method::Method` in its place.

# v0.1.0

Improve on the crate documentation.

# v0.0.1

Initial release to get the ball rolling.