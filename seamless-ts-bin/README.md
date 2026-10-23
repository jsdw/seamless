# seamless_ts_bin

A `seamless-ts` command line tool which generates TypeScript types describing a
[seamless](https://crates.io/crates/seamless) API, using the `seamless_ts` crate.

It takes the JSON produced by serializing the result of `seamless::Api::info()`, either from
stdin or from a file given with `--file`, and writes the generated TypeScript to stdout. Any
errors are written to stderr.

```sh
cargo install seamless_ts_bin

# Read the API info from stdin:
curl https://example.com/api/meta.info | seamless-ts > api.ts

# Or read it from a file:
seamless-ts --file api-info.json > api.ts
```
