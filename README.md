# FastCtx

**English** | [简体中文](./README.zh-CN.md)

FastCtx is the Rust file-tool library maintained by [fraternity-z](https://github.com/Ariember/fastctx) for CodeyX. This distribution adapts [yc-duan/fastctx](https://github.com/yc-duan/fastctx) for Codex native tool execution.

| Tool | Purpose |
| --- | --- |
| `inspect_local_file` | Read text, images, and raw bytes with bounded output. |
| `grep` | Search file contents with filters and pagination. |
| `glob` | Find files with glob patterns, filters, and pagination. |
| `replace` | Preview or apply batch replacements with file-write safeguards. |

## CodeyX integration

CodeyX owns the `codey-fastctx` sidecar, its process lifecycle, request framing, and cancellation. It publishes these four tools in the `codey_fastctx` namespace through the Codex native tool interface.

The sidecar uses `fastctx::server::FastCtxServer::new()`, `tool_definitions()`, and the `rmcp::ServerHandler` call interface. The `rmcp` types remain part of this Rust API; CodeyX invokes the handlers directly within its native worker.

CodeyX pins this library to a Git revision in its workspace `Cargo.toml`. To adopt a library change, update that revision and CodeyX's `Cargo.lock`, then run the sidecar's native tool tests. Editing a separate FastCtx checkout does not change the pinned dependency.

File operations run with the worker's operating-system permissions. CodeyX controls tool availability and the permitted execution context; `replace` can write files.

## Development

```console
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo build --locked --release --lib
```

Issues and contributions belong in [this repository](https://github.com/Ariember/fastctx/issues). See [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

FastCtx uses the [Apache License 2.0](./LICENSE-APACHE). Preserve the [NOTICE](./NOTICE) and applicable [third-party notices](./THIRD_PARTY_LICENSES_RUST.md) when redistributing it.

This product includes FastCtx
(https://github.com/yc-duan/fastctx), Copyright (c) 2026 yc-duan,
used under the Apache License 2.0.

FastCtx is redistributed and/or modified here by the maintainer of
this distribution. Any such change is that maintainer's own work
and their sole responsibility. It is not endorsed by, not
supported by, and not attributable to the author of FastCtx, who
accepts no liability of any kind arising from this distribution or
from anything built on top of it.
