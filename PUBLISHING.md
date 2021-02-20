# Publishing a new version of these crates

Follow these steps to publish a new version

## Test

Make sure tests pass and examples happily run:

```
cargo test --all-targets
```

## Check the docs

Make sure that the documentation is gravy

```
cargo doc --open
```

Copy any doc changes from `seamless/lib.rs` to `seamless/README.md` and `README.md`.

## Bump versions

Bump the version in `seamless/Cargo.toml` and `seamless-macros/Cargo.toml`.

## Update CHANGELOG.md

Describe what changes there are in this new version.

## Commit and tag

```
git add --all
git commit -m "Bump version to vX.X.X"
git tag vX.X.X
git push origin master --tags
```

## Publish

Publish the macro crate first since the main crate depends on it existing.

```
(cd seamless-macros && cargo publish)
(cd seamless && cargo publish)
```