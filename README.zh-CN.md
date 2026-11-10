# FastCtx

[English](./README.md) | **简体中文**

FastCtx 是由 [fraternity-z](https://github.com/Ariember/fastctx) 为 CodeyX 维护的 Rust 文件工具库。本版本基于 [yc-duan/fastctx](https://github.com/yc-duan/fastctx) 修改，服务于 Codex 原生工具调用。

| 工具 | 用途 |
| --- | --- |
| `inspect_local_file` | 读取文本、图片和原始字节，控制输出大小。 |
| `grep` | 搜索文件内容，支持过滤与分页。 |
| `glob` | 按 glob 模式查找文件，支持过滤与分页。 |
| `replace` | 预览或执行批量替换，保留文件写入保护。 |
| `delete` | 预览或删除显式指定的文件、空目录和符号链接。 |
| `move` | 在同一文件系统内移动或重命名一个路径项，目标存在时报错。 |
| `create_file` | 创建 UTF-8 文本文件，路径存在时报错。 |

## 文件增删与移动

所有修改路径必须是绝对路径。`delete`、`move` 和 `create_file` 与替换服务共用进程内锁和跨进程锁，直接处理路径项，不解码已有文件内容。

- `delete({"paths": ["/absolute/path/file.txt", "/absolute/path/empty-directory"]})` 默认预览，显式设置 `"dry_run": false` 才执行。支持文件和空目录，非空目录报错。符号链接只删除链接本身，保留目标，支持失效链接和目录链接。按请求顺序逐项处理；某项失败不会撤销已完成的删除，也不会阻止后续项目继续处理。末行用 `Complete` 或 `Partial` 报告总数；报告过长时提示列表截断，需要完整逐项明细时应缩小批次。只接受显式路径，不展开 glob，也不应用忽略规则。
- `move({"source": "/absolute/path/old", "destination": "/absolute/path/new"})` 移动文件、目录或符号链接。目标父目录必须已存在；目标已存在时报错，包括失效链接。通过同一文件系统内的重命名完成移动，不回退到复制。
- `create_file({"path": "/absolute/path/new.txt", "content": "hello\n"})` 原样写入 UTF-8 内容，不额外添加 BOM 或换行。父目录必须已存在，目标路径必须不存在，包括符号链接。已有文本通过 `replace` 修改。

## CodeyX 接入

CodeyX 负责 `codey-fastctx` 工作进程的生命周期、请求传输和取消，并通过 Codex 原生工具接口将本库的工具注册到 `codey_fastctx` 命名空间。

工作进程使用 `fastctx::server::FastCtxServer::new()`、`tool_definitions()` 和 `rmcp::ServerHandler` 调用接口。`rmcp` 类型属于 Rust API 的一部分；CodeyX 在原生工作进程内直接调用处理函数。

CodeyX 在工作区 `Cargo.toml` 中固定本库的 Git revision。采用本库变更时，需要更新该 revision 和 CodeyX 的 `Cargo.lock`，并运行工作进程的原生工具测试。修改另一份 FastCtx 检出目录不会改变已经固定的依赖。

文件操作使用工作进程的系统权限。CodeyX 负责工具可用性和执行上下文的权限边界；修改工具可以创建、写入、移动和删除路径项。

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
