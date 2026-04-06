//! File-path to LSP-URI conversions.

use std::path::Path;

use tower_lsp_server::ls_types::Uri;

/// Converts an absolute filesystem path into a percent-encoded file URI.
pub fn file_uri_from_path(path: &Path) -> Option<Uri> {
    Uri::from_file_path(path)
}
