//! CPU ceiling shared by the file-search executor.
//! Modified for Ariember/fastctx: use automatic worker parallelism.

/// Hard ceiling for one request-local lane plus shared extra lanes.
pub(crate) const MAX_SEARCH_PARALLELISM: usize = 16;

/// Detects the upper bound used by the search executor.
pub(crate) fn detected_available() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, MAX_SEARCH_PARALLELISM)
}
