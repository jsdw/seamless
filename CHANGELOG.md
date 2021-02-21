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