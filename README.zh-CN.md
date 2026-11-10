# FastCtx

[English](./README.md) | **简体中文**

FastCtx 是由 [fraternity-z](https://github.com/Ariember/fastctx) 为 CodeyX 维护的 Rust 文件工具库。本版本基于 [yc-duan/fastctx](https://github.com/yc-duan/fastctx) 修改，服务于 Codex 原生工具调用。

| 工具 | 用途 |
| --- | --- |
| `inspect_local_file` | 读取文本、图片和原始字节，控制输出大小。 |
| `grep` | 搜索文件内容，支持过滤与分页。 |
| `glob` | 按 glob 模式查找文件，支持过滤与分页。 |
| `replace` | 预览或执行批量替换，保留文件写入保护。 |

## CodeyX 接入

CodeyX 负责 `codey-fastctx` 工作进程的生命周期、请求传输和取消，并通过 Codex 原生工具接口将这四个工具注册到 `codey_fastctx` 命名空间。

工作进程使用 `fastctx::server::FastCtxServer::new()`、`tool_definitions()` 和 `rmcp::ServerHandler` 调用接口。`rmcp` 类型属于 Rust API 的一部分；CodeyX 在原生工作进程内直接调用处理函数。

CodeyX 在工作区 `Cargo.toml` 中固定本库的 Git revision。采用本库变更时，需要更新该 revision 和 CodeyX 的 `Cargo.lock`，并运行工作进程的原生工具测试。修改另一份 FastCtx 检出目录不会改变已经固定的依赖。

文件操作使用工作进程的系统权限。CodeyX 负责工具可用性和执行上下文的权限边界；`replace` 可以写入文件。

## 开发

```console
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo build --locked --release --lib
```

问题反馈和贡献请提交到[本仓库](https://github.com/Ariember/fastctx/issues)，参见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 许可

FastCtx 采用 [Apache License 2.0](./LICENSE-APACHE)。再分发时须保留 [NOTICE](./NOTICE) 和适用的[第三方声明](./THIRD_PARTY_LICENSES_RUST.md)。

This product includes FastCtx
(https://github.com/yc-duan/fastctx), Copyright (c) 2026 yc-duan,
used under the Apache License 2.0.

FastCtx is redistributed and/or modified here by the maintainer of
this distribution. Any such change is that maintainer's own work
and their sole responsibility. It is not endorsed by, not
supported by, and not attributable to the author of FastCtx, who
accepts no liability of any kind arising from this distribution or
from anything built on top of it.
