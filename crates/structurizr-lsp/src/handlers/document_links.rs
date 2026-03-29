//! Document-link handler for directive arguments that point at files or folders.

use std::{collections::BTreeMap, str::FromStr};

use structurizr_analysis::{DocumentSnapshot, TextSpan, WorkspaceFacts};
use tower_lsp_server::ls_types::{DocumentLink, DocumentLinkParams, Uri};

use crate::{convert::positions::span_to_range, documents::DocumentState, server::Backend};

/// Handles `textDocument/documentLink` for local directive paths.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing documents are
/// reported as `Ok(None)`.
pub async fn document_link(
    backend: &Backend,
    params: DocumentLinkParams,
) -> tower_lsp_server::jsonrpc::Result<Option<Vec<DocumentLink>>> {
    let links = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&params.text_document.uri) else {
            return Ok(None);
        };

        document_links(document, snapshot, state.workspace_facts())
    };

    Ok(Some(links))
}

fn document_links(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Vec<DocumentLink> {
    // Path-like directive spans are intentionally surfaced through
    // `textDocument/documentLink`, not `definition`. That keeps semantic symbol
    // navigation and plain file/folder opening as separate UX surfaces.
    let mut links = super::directive_paths::resolved_directive_paths(snapshot, workspace_facts)
        .into_iter()
        .fold(
            BTreeMap::<TextSpan, Vec<_>>::new(),
            |mut paths_by_span, path| {
                paths_by_span.entry(path.span()).or_default().push(path);
                paths_by_span
            },
        )
        // `documentLink` cannot represent "one source span, many possible targets"
        // in a way editors handle predictably. Suppress ambiguous spans here and
        // let the definition fallback surface multiple file targets instead.
        .into_values()
        .filter_map(|mut paths| {
            if paths.len() != 1 {
                return None;
            }

            let path = paths.pop().expect("one resolved path should exist");
            link_for_target(
                document,
                path.span(),
                path.path(),
                path.kind().tooltip().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    links.sort_by_key(|link| {
        (
            link.range.start.line,
            link.range.start.character,
            link.range.end.line,
            link.range.end.character,
        )
    });
    links
}

fn link_for_target(
    document: &DocumentState,
    span: TextSpan,
    target: &std::path::Path,
    tooltip: String,
) -> Option<DocumentLink> {
    let target = file_uri_from_path(target)?;
    Some(DocumentLink {
        range: span_to_range(document.line_index(), span)?,
        target: Some(target),
        tooltip: Some(tooltip),
        data: None,
    })
}

fn file_uri_from_path(path: &std::path::Path) -> Option<Uri> {
    Uri::from_str(&format!("file://{}", path.to_string_lossy())).ok()
}
