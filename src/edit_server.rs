//! File mutation routes in the single `fastctx` server.
//! Modified for Ariember/fastctx: edit through the native file service.

use crate::budget::GLOBAL_TOKEN_BUDGET_ENV;
use crate::edit::{CreateFileRequest, DeleteRequest, MoveRequest, ReplaceRequest};
use crate::server::FastCtxServer;
use crate::server_support::run_blocking;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use std::sync::Arc;

#[tool_router(router = edit_tool_router, vis = "pub(crate)")]
impl FastCtxServer {
    #[tool(
        name = "delete",
        description = "Delete an explicit list of absolute filesystem paths. Defaults to dry_run=true;\nset dry_run=false explicitly to execute. Supports files and empty directories;\nnon-empty directories fail without recursive deletion. Symbolic links are\nremoved themselves, preserving their targets, including dangling links. No glob\nexpansion or ignore filtering is applied. Results follow request order; the\nlast line states Complete or Partial and retains counts of completed operations.",
        annotations(
            title = "Delete filesystem entries",
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete(&self, Parameters(request): Parameters<DeleteRequest>) -> CallToolResult {
        let editor = self.replace.clone();
        run_blocking(
            Arc::clone(&self.session),
            Arc::clone(&self.replace_permits),
            GLOBAL_TOKEN_BUDGET_ENV,
            move || editor.delete(request),
        )
        .await
    }

    #[tool(
        name = "move",
        description = "Move or rename one file, directory, or symbolic link within the same\nfilesystem. Both paths must be absolute. The destination must not exist,\nincluding symbolic links, and its parent directory must already exist.\nSymbolic links are moved themselves. The last line states Complete on success.",
        annotations(
            title = "Move filesystem entry",
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn move_path(&self, Parameters(request): Parameters<MoveRequest>) -> CallToolResult {
        let editor = self.replace.clone();
        run_blocking(
            Arc::clone(&self.session),
            Arc::clone(&self.replace_permits),
            GLOBAL_TOKEN_BUDGET_ENV,
            move || editor.move_path(request),
        )
        .await
    }

    #[tool(
        name = "create_file",
        description = "Create a UTF-8 text file at an absolute path, writing content verbatim\nwithout adding a BOM or newline. The path must not exist, including symbolic\nlinks, and its parent directory must already exist. Use replace to modify\nexisting text. The last line states Complete on success.",
        annotations(
            title = "Create text file",
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_file(
        &self,
        Parameters(request): Parameters<CreateFileRequest>,
    ) -> CallToolResult {
        let editor = self.replace.clone();
        run_blocking(
            Arc::clone(&self.session),
            Arc::clone(&self.replace_permits),
            GLOBAL_TOKEN_BUDGET_ENV,
            move || editor.create_file(request),
        )
        .await
    }

    #[tool(
        name = "replace",
        description = "Batch find-and-replace across a file or directory (Rust regex, same engine\nas grep; no lookaround). A reference to an undefined capture group is\nrejected before any write. To delete whole lines, include \\n in the\npattern. Matching is leftmost-first and non-overlapping; unlike grep,\n`^`/`$` anchor the whole file by default — use (?m) for per-line anchors.\nRespects .gitignore; skips .git and binaries; files whose encoding cannot\nbe determined are skipped and listed. Each file is written atomically with\na concurrent-modification check, preserving its original encoding, BOM, and\nline endings. The last line states Complete or Partial.",
        annotations(
            title = "Batch replace file contents",
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn replace(&self, Parameters(request): Parameters<ReplaceRequest>) -> CallToolResult {
        let replace = self.replace.clone();
        run_blocking(
            Arc::clone(&self.session),
            Arc::clone(&self.replace_permits),
            GLOBAL_TOKEN_BUDGET_ENV,
            move || replace.replace(request.clone()),
        )
        .await
    }
}
