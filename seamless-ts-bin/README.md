# seamless_ts_bin

A `seamless-ts` command line tool which generates TypeScript types describing a
[seamless](https://crates.io/crates/seamless) API, using the `seamless_ts` crate.

It takes the JSON produced by serializing the result of `seamless::Api::info()`, either from
stdin, from a file given with `--file`, or from a URL given with `--url`, and writes the generated TypeScript to stdout. Any
errors are written to stderr.

```sh
cargo install seamless_ts_bin

# Fetch the API info from a URL:
seamless-ts --url https://example.com/api/meta.info > api.ts

# Headers can be sent along with the request (as many as you like):
seamless-ts --url https://example.com/api/meta.info -H 'Authorization: Bearer foo' > api.ts

# Read the API info from stdin:
cat api-info.json | seamless-ts > api.ts

# Or read it from a file:
seamless-ts --file api-info.json > api.ts
```
