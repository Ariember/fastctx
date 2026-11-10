//! Single-source prose for routing local-file work into the model-visible tool surface.
//! Modified for Ariember/fastctx: describe the supported native file tools.

/// Positive routing used by native tool descriptions.
pub(crate) const LOCAL_FILE_ROUTE_GUIDANCE: &str = concat!(
    "Use FastCtx file tools directly for local-file operations, including when a\n",
    "local reference is URI-shaped; pass the equivalent plain absolute filesystem path."
);

/// Shared path-shape contract used by every file tool's path field.
pub(crate) const LOCAL_PATH_INPUT_GUIDANCE: &str = concat!(
    "Plain absolute local filesystem path. When the source reference is URI-shaped, ",
    "use its equivalent local absolute path."
);

/// Opening sentence of the file-inspection tool's description.
///
/// It leads with what the tool is over a local filesystem path, because that is the
/// sentence a routing decision is made on: `read_mcp_resource` competes for the same
/// intent and the discriminating fact is the input shape, not the verb (2026-08-08).
const INSPECT_TOOL_SUMMARY: &str = concat!(
    "Inspect the contents of local filesystem paths: one file (text or image),\n",
    "a batch of text files, or any file's raw bytes."
);

const INSPECT_TOOL_DETAILS: &str = concat!(
    "Text returns 1-based `N<tab>content` lines, as much of the file as\n",
    "the output budget holds. For several text files in one call, pass\n",
    "files=[{\"path\": ...}, ...] instead of file_path. Repeat a path for distinct\n",
    "offset/limit ranges, and freely mix ranges from multiple files. A top-level limit\n",
    "is the default for entries that omit one; an entry limit overrides it. The batch uses\n",
    "one token budget,\n",
    "per-entry problems reported inline, and a Partial note returns\n",
    "the exact files array for the next call. Images (PNG/JPG/GIF/WebP/BMP) are\n",
    "shown to you visually. view=\"hex\" dumps any file's raw bytes. Images and\n",
    "hex view are single-file only. Text\n",
    "output is always UTF-8; when auto-detection is not confident it returns an\n",
    "error listing candidate encodings instead of guessed text, so pass encoding\n",
    "only then. Text and hex responses end with a Complete or Partial status\n",
    "— continue only with the exact parameters a Partial note provides."
);

pub(crate) fn local_path_description(context: &str) -> String {
    format!("{LOCAL_PATH_INPUT_GUIDANCE} {context}")
}

pub(crate) fn inspect_tool_description() -> String {
    format!(
        "{} {} {INSPECT_TOOL_DETAILS}",
        INSPECT_TOOL_SUMMARY.replace('\n', " "),
        LOCAL_FILE_ROUTE_GUIDANCE.replace('\n', " ")
    )
}
