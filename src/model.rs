//! Text and image results returned by native file tools.
//! Modified for Ariember/fastctx: retain the native file content model.

/// One content block returned by a tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolContent {
    /// UTF-8 text for the model.
    Text(String),
    /// Base64 image data and MIME type.
    Image {
        /// Standard Base64 data without a data-URL prefix.
        data: String,
        /// IANA MIME type corresponding to the magic bytes.
        mime_type: String,
    },
}

/// Tool response with explicit error state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResponse {
    /// Text or image blocks in model-consumption order.
    pub content: Vec<ToolContent>,
    /// When true, the native result carries `isError: true`.
    pub is_error: bool,
}

impl ToolResponse {
    /// Returns one successful text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text(text.into())],
            is_error: false,
        }
    }

    /// Returns one text block marked as an error.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text(message.into())],
            is_error: true,
        }
    }
}
