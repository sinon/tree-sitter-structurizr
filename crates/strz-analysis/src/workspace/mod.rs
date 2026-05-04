//! Workspace discovery, include-following, and file-level include diagnostics.

// Keep the workspace layer split across focused files without introducing a
// second namespace layer for every internal helper. These fragments are all part
// of the same `workspace` module; the file boundaries exist to keep each concern
// readable and reviewable.
#[cfg(test)]
include!("test_support.rs");

include!("model.rs");
include!("errors.rs");
include!("loader.rs");
include!("session.rs");
include!("includes.rs");
include!("index.rs");
include!("diagnostics/structure.rs");
include!("diagnostics/resources.rs");
include!("diagnostics/views.rs");
include!("references.rs");
include!("selectors.rs");
include!("bindings.rs");
include!("diagnostics/merge.rs");
