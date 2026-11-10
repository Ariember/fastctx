# Contributing to FastCtx

This guide is maintained by fraternity-z for the CodeyX native file-tool library derived from yc-duan/fastctx.

Report issues and submit focused changes at <https://github.com/Ariember/fastctx/issues>. Include the platform, library revision, and steps to reproduce bugs.

## Development

Keep changes focused on CodeyX's native file tools and preserve their input schemas, output formats, cancellation behavior, and file-write safeguards.

```console
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo build --locked --release --lib
```

For API changes, update the CodeyX sidecar and run its native tool tests against the changed library. CodeyX's pinned Git dependency must be updated before the changes reach a normal CodeyX build.

## License of contributions

FastCtx is licensed under the Apache License 2.0.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in FastCtx by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions. Preserve the applicable license, notices, and modification notices in changed upstream files.
