//! Typed registration and shared state for the CodeyX native file tools.
//! Modified for Ariember/fastctx: retain the CodeyX native file tool library.

use crate::budget::{GLOB_TOKEN_BUDGET_ENV, GREP_TOKEN_BUDGET_ENV, READ_TOKEN_BUDGET_ENV};
use crate::edit::ReplaceService;
use crate::file_executor::GrepGlobExecutor;
use crate::glob_tool::{GlobRequest, glob_files_cancellable};
use crate::grep_tool::{GrepRequest, grep_files_cancellable};
use crate::read_tool::{ReadRequest, read_file};
use crate::server_support::{CancellableBlockingRequest, run_blocking, run_blocking_cancellable};
use crate::session::SessionContext;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, tool, tool_handler, tool_router};
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FILE_OPERATIONS: usize = 8;
const MAX_REPLACE_OPERATIONS: usize = 8;

/// Stateful file tools used by the CodeyX native worker.
#[derive(Clone, Debug)]
pub struct FastCtxServer {
    tool_router: ToolRouter<Self>,
    pub(crate) replace: ReplaceService,
    pub(crate) file_permits: Arc<Semaphore>,
    pub(crate) grep_glob_executor: Arc<GrepGlobExecutor>,
    pub(crate) replace_permits: Arc<Semaphore>,
    pub(crate) session: Arc<SessionContext>,
}

impl FastCtxServer {
    /// Creates the four native file tools with the worker's cwd and environment.
    pub fn new() -> Self {
        Self::with_executor(GrepGlobExecutor::shared())
    }

    fn with_executor(grep_glob_executor: Arc<GrepGlobExecutor>) -> Self {
        let mut tool_router = Self::file_tool_router();
        tool_router.merge(Self::edit_tool_router());
        tool_router
            .map
            .get_mut("inspect_local_file")
            .expect("the compiled file router must contain inspect_local_file")
            .attr
            .description = Some(crate::model_guidance::inspect_tool_description().into());
        for route in tool_router.map.values_mut() {
            route.attr.input_schema = Arc::new(crate::tool_schema::normalize_published_schema(
                &route.attr.input_schema,
            ));
        }
        Self {
            tool_router,
            replace: ReplaceService::new(),
            file_permits: Arc::new(Semaphore::new(MAX_FILE_OPERATIONS)),
            grep_glob_executor,
            replace_permits: Arc::new(Semaphore::new(MAX_REPLACE_OPERATIONS)),
            session: SessionContext::library_default(),
        }
    }

    /// Returns the native tools' descriptions and portable input schemas.
    pub fn tool_definitions(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }
}

impl Default for FastCtxServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = file_tool_router, vis = "pub(crate)")]
impl FastCtxServer {
    #[tool(
        name = "inspect_local_file",
        description = "Inspect local files.",
        annotations(
            title = "Inspect local file",
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn inspect_local_file(
        &self,
        Parameters(request): Parameters<ReadRequest>,
    ) -> CallToolResult {
        run_blocking(
            Arc::clone(&self.session),
            Arc::clone(&self.file_permits),
            READ_TOKEN_BUDGET_ENV,
            move || read_file(request.clone()),
        )
        .await
    }

    #[tool(
        name = "grep",
        description = "Fast regex content search (ripgrep engine; Rust regex, no lookaround). Output\nmodes: \"files_with_matches\" (default, paths only), \"content\", \"count\" (total\nmatches, not matching lines), \"summary\" (global totals). Respects .gitignore;\nsearches hidden files; skips .git and binaries. Files are decoded to UTF-8\nbefore searching; files whose encoding can't be determined, that change, or\nthat cannot be searched are skipped and listed for directory targets; the\nequivalent single-file failure returns an error. Matching is line-by-line:\n`^` and `$` anchor line boundaries and are CRLF-aware. A path component of the\nform ~fastctx~b...~ (reversible bytes/UTF-8) or ~fastctx~w...~ (Windows UTF-16)\nis a filename escape; copy that whole component verbatim in later calls and\ndo not decode or rewrite it. The last line of every successful result states\nComplete or Partial — continue only with the exact offset a Partial note\nprovides; errors are self-contained.",
        annotations(
            title = "Search file contents",
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn grep(
        &self,
        Parameters(request): Parameters<GrepRequest>,
        context: RequestContext<RoleServer>,
    ) -> CallToolResult {
        run_blocking_cancellable(
            CancellableBlockingRequest::new(
                Arc::clone(&self.session),
                context.id,
                context.ct,
                Arc::clone(&self.file_permits),
                Arc::clone(&self.grep_glob_executor),
                GREP_TOKEN_BUDGET_ENV,
            ),
            move |operation, executor| grep_files_cancellable(operation, executor, request.clone()),
        )
        .await
    }

    #[tool(
        name = "glob",
        description = "Find files by glob pattern, e.g. \"**/*.rs\" or \"src/**/*.ts\". Matches files\nonly, never directories. Returns absolute paths sorted by path (or newest first\nwith sort=\"modified\"), 100 per page by default. filter_mode defaults to\n\"ignore\" (plain .ignore files only); \"all\" disables that filtering. Hidden\nfiles and .git are always visible unless a negative pattern excludes them.\noutput_mode defaults to \"paths\"; \"details\" returns compact JSON lines with\npath, byte size, and UTC modification time. Omit `path` entirely for the session\nworking directory — never pass \"null\" or \"undefined\". A path component of the\nform ~fastctx~b...~ (reversible bytes/UTF-8) or ~fastctx~w...~ (Windows UTF-16)\nis a filename escape; copy that whole component verbatim in later calls and do\nnot decode or rewrite it. The last line of every successful result states\nComplete or Partial — continue only with the exact offset a Partial note\nprovides; errors are self-contained.",
        annotations(
            title = "Match file paths",
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn glob(
        &self,
        Parameters(request): Parameters<GlobRequest>,
        context: RequestContext<RoleServer>,
    ) -> CallToolResult {
        run_blocking_cancellable(
            CancellableBlockingRequest::new(
                Arc::clone(&self.session),
                context.id,
                context.ct,
                Arc::clone(&self.file_permits),
                Arc::clone(&self.grep_glob_executor),
                GLOB_TOKEN_BUDGET_ENV,
            ),
            move |operation, executor| glob_files_cancellable(operation, executor, request.clone()),
        )
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FastCtxServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FastCtxServer;
    use crate::file_executor::GrepGlobExecutor;
    use crate::search_parallelism::MAX_SEARCH_PARALLELISM;
    use std::sync::Arc;

    #[test]
    fn configured_executor_is_the_server_search_source_for_serial_mid_and_maximum_p() {
        let middle = (MAX_SEARCH_PARALLELISM / 2).max(1);
        for parallelism in [1, middle, MAX_SEARCH_PARALLELISM] {
            let executor = Arc::new(GrepGlobExecutor::with_test_parallelism(parallelism));
            let server = FastCtxServer::with_executor(Arc::clone(&executor));
            assert!(Arc::ptr_eq(&server.grep_glob_executor, &executor));
            assert_eq!(server.grep_glob_executor.parallelism(), parallelism);
            assert_eq!(server.grep_glob_executor.extra_capacity(), parallelism - 1);
        }
    }
}
