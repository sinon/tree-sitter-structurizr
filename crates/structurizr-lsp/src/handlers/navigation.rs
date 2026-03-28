//! Shared same-document navigation helpers for the bounded first LSP slice.
//!
//! These helpers intentionally stay within one analyzed snapshot. Cross-file
//! resolution can layer on top later without complicating the bounded MVP.

use structurizr_analysis::{DocumentSnapshot, Reference, ReferenceTargetHint, Symbol, SymbolKind};

/// Finds the declaration or reference target at one byte offset.
///
/// Returns the referenced declaration when the offset lands on a reference, or
/// the directly bound declaration when the offset lands on a symbol site.
pub fn target_symbol_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<&Symbol> {
    reference_at_offset(snapshot, offset).map_or_else(
        || bindable_symbol_at_offset(snapshot, offset),
        |reference| resolve_reference(snapshot, reference),
    )
}

/// Collects all same-document references that resolve to one symbol.
#[must_use]
pub fn references_for_symbol<'a>(
    snapshot: &'a DocumentSnapshot,
    symbol: &Symbol,
) -> Vec<&'a Reference> {
    snapshot
        .references()
        .iter()
        .filter(|reference| symbol_matches_reference(symbol, reference))
        .collect()
}

fn bindable_symbol_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<&Symbol> {
    snapshot
        .symbols()
        .iter()
        .filter(|symbol| {
            symbol.binding_name.is_some()
                && span_contains(symbol.span.start_byte, symbol.span.end_byte, offset)
        })
        .min_by_key(|symbol| symbol.span.end_byte - symbol.span.start_byte)
}

fn reference_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<&Reference> {
    snapshot
        .references()
        .iter()
        .find(|reference| span_contains(reference.span.start_byte, reference.span.end_byte, offset))
}

fn resolve_reference<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
) -> Option<&'a Symbol> {
    // Prefer returning no result over guessing between multiple candidates. The
    // bounded MVP stays conservative until cross-file resolution is in place.
    let candidates: Vec<&Symbol> = snapshot
        .symbols()
        .iter()
        .filter(|symbol| symbol_matches_reference(symbol, reference))
        .collect();

    if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

fn symbol_matches_reference(symbol: &Symbol, reference: &Reference) -> bool {
    let Some(binding_name) = symbol.binding_name.as_deref() else {
        return false;
    };

    if binding_name != reference.raw_text {
        return false;
    }

    match reference.target_hint {
        ReferenceTargetHint::Element => symbol.kind != SymbolKind::Relationship,
        ReferenceTargetHint::Relationship => symbol.kind == SymbolKind::Relationship,
        ReferenceTargetHint::ElementOrRelationship => true,
    }
}

const fn span_contains(start_byte: usize, end_byte: usize, offset: usize) -> bool {
    if start_byte == end_byte {
        offset == start_byte
    } else {
        start_byte <= offset && offset < end_byte
    }
}
