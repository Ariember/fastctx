//! Shared test helpers for fixtures generated at runtime.
//! Modified for CodeyX: exercise native file-tool handlers directly.

#![allow(dead_code)]

use fastctx::glob_tool::GlobRequest;
use fastctx::grep_tool::GrepRequest;
use fastctx::server::FastCtxServer;
use fastctx::session::SessionEnvironment;
use fastctx::{ToolContent, ToolResponse};
use rmcp::model::{CallToolRequestParams, RequestId};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub fn text(response: ToolResponse) -> String {
    assert!(!response.is_error, "unexpected tool error: {response:?}");
    assert_eq!(response.content.len(), 1);
    match response.content.into_iter().next().unwrap() {
        ToolContent::Text(text) => text,
        content => panic!("expected text content, got {content:?}"),
    }
}

pub fn error_text(response: ToolResponse) -> String {
    assert!(response.is_error, "expected tool error: {response:?}");
    assert_eq!(response.content.len(), 1);
    match response.content.into_iter().next().unwrap() {
        ToolContent::Text(text) => text,
        content => panic!("expected text error, got {content:?}"),
    }
}

pub fn grep_files(request: GrepRequest) -> ToolResponse {
    fastctx::grep_tool::grep_files(request, tokio_util::sync::CancellationToken::new())
}

pub fn glob_files(request: GlobRequest) -> ToolResponse {
    fastctx::glob_tool::glob_files(request, tokio_util::sync::CancellationToken::new())
}

pub fn normalized(path: &Path) -> String {
    let absolute = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut value = absolute.to_string_lossy().replace('\\', "/");
    if let Some(rest) = value.strip_prefix("//?/UNC/") {
        value = format!("//{rest}");
    } else if let Some(rest) = value.strip_prefix("//?/") {
        value = rest.to_string();
    }
    value
}

pub fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

pub fn call_tool(root: &Path, budget: usize, name: &str, arguments: Value) -> Value {
    let environment = Arc::new(SessionEnvironment::new(
        root.to_path_buf(),
        [
            "FASTCTX_TOKEN_BUDGET",
            "FASTCTX_READ_TOKEN_BUDGET",
            "FASTCTX_GREP_TOKEN_BUDGET",
            "FASTCTX_GLOB_TOKEN_BUDGET",
        ]
        .into_iter()
        .map(|name| (name.into(), budget.to_string().into()))
        .collect(),
    ));
    let server = environment.activate(FastCtxServer::new);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (transport, _peer_transport) = tokio::io::duplex(1);
            let service = rmcp::service::serve_directly::<RoleServer, _, _, _, _>(
                server.clone(),
                transport,
                None,
            );
            let context =
                RequestContext::new(RequestId::String("contract".into()), service.peer().clone());
            let call = CallToolRequestParams::new(name.to_string())
                .with_arguments(arguments.as_object().unwrap().clone());
            let response = server.call_tool(call, context).await.unwrap();
            service.cancel().await.unwrap();
            serde_json::to_value(response).unwrap()
        })
}

pub fn tool_text(response: &Value) -> &str {
    response["content"][0]["text"].as_str().unwrap()
}
