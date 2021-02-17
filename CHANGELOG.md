# v0.4.0

- Rename `RequestParam`'s `get_param` to `request_param`.
- A couple of minor tweaks and trait impls.
- Add more documentation.

# v0.3.0

- Rename and move things around so that they hopefully make more sense (see examples and such for specifics).
- Remove the dependency on `ApiError` from various traits.
- Tidy up export hierarchy.

# v0.2.0

- Allow the expected HTTP method for a route to be configured by implementors of the `Body` trait.
  (previously it was always POST if a `Body` was provided, else GET).
- Remove `IntoApiError` and implement `From<T>` as appropriate instead.
- Remove `Method` enum and use `http::method::Method` in its place.

# v0.1.0

Improve on the crate documentation.

# v0.0.1

Initial release to get the ball rolling.