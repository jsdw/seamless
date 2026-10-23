# Publishing a new version of these crates

Follow these steps to publish a new version

## Test

Make sure tests pass and examples happily run:

```
cargo test
cargo test --examples
```

## Check the docs

Make sure that the documentation is gravy

```
cargo doc --open
```

Copy any doc changes from `seamless/lib.rs` to the edited versions of `seamless/README.md` and `README.md`.

## Bump versions

Bump the version in the root `Cargo.toml` (both `workspace.package.version` and the `seamless_macros` and `seamless_info` workspace dependency versions).

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

Run this in the workspace to publish everything

```
cargo publish
```