# FastCtx

**English** | [简体中文](./README.zh-CN.md)

FastCtx is the Rust file-tool library maintained by [fraternity-z](https://github.com/Ariember/fastctx) for CodeyX. This distribution adapts [yc-duan/fastctx](https://github.com/yc-duan/fastctx) for Codex native tool execution.

| Tool | Purpose |
| --- | --- |
| `inspect_local_file` | Read text, images, and raw bytes with bounded output. |
| `grep` | Search file contents with filters and pagination. |
| `glob` | Find files with glob patterns, filters, and pagination. |
| `replace` | Preview or apply batch replacements with file-write safeguards. |
| `delete` | Preview or delete explicit files, empty directories, and symbolic links. |
| `move` | Move or rename one entry within a filesystem, refusing existing destinations. |
| `create_file` | Create a UTF-8 text file, refusing existing paths. |

## Filesystem mutations

All mutation paths must be absolute. `delete`, `move`, and `create_file` share the replacement service's in-process and cross-process locks. They operate on filesystem entries without decoding existing content.

- `delete({"paths": ["/absolute/path/file.txt", "/absolute/path/empty-directory"]})` previews by default. Set `"dry_run": false` explicitly to execute. Files and empty directories are supported; non-empty directories fail. Symbolic links, including dangling links and directory links, are removed themselves, preserving their targets. Each entry is processed in request order; a failed entry does not undo completed deletions or prevent later entries from being processed. The final `Complete` or `Partial` line reports totals; oversized reports identify truncation. Use smaller batches for full per-entry detail. Paths are explicit: there is no glob expansion or ignore filtering.
- `move({"source": "/absolute/path/old", "destination": "/absolute/path/new"})` moves a file, directory, or symbolic link. The destination's parent must exist; an existing destination, including a dangling link, is an error. Moves use a same-filesystem rename without a copy fallback.
- `create_file({"path": "/absolute/path/new.txt", "content": "hello\n"})` writes UTF-8 content verbatim, without adding a BOM or newline. The parent must exist and the destination must be absent, including symbolic links. Existing text is edited with `replace`.

## CodeyX integration

CodeyX owns the `codey-fastctx` sidecar, its process lifecycle, request framing, and cancellation. It publishes the library's tools in the `codey_fastctx` namespace through the Codex native tool interface.

The sidecar uses `fastctx::server::FastCtxServer::new()`, `tool_definitions()`, and the `rmcp::ServerHandler` call interface. The `rmcp` types remain part of this Rust API; CodeyX invokes the handlers directly within its native worker.

CodeyX pins this library to a Git revision in its workspace `Cargo.toml`. To adopt a library change, update that revision and CodeyX's `Cargo.lock`, then run the sidecar's native tool tests. Editing a separate FastCtx checkout does not change the pinned dependency.

File operations run with the worker's operating-system permissions. CodeyX controls tool availability and the permitted execution context; mutation tools can create, write, move, and delete filesystem entries.

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
