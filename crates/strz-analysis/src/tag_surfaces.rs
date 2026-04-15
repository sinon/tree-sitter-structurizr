//! Shared descriptions of syntax sites that carry explicit tag values.
//!
//! Analysis uses these descriptors when collecting workspace tag vocabularies,
//! and the LSP reuses the same table when deciding whether the cursor is inside
//! a tag-valid completion context. Keeping the mapping here reduces the risk of
//! grammar coverage drifting between extraction and editor features.

/// Shared description of one syntax surface that can carry explicit tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagSurface {
    /// A `tag ...` body statement that contributes one tag value.
    TagStatement,
    /// A `tags ...` body statement that contributes one or more tag values.
    TagsStatement,
    /// A node whose tag value lives in one named field.
    NamedField {
        /// The field name containing the tag value.
        field_name: &'static str,
        /// Whether the surface uses one comma-delimited tag string.
        comma_separated: bool,
    },
    /// A node whose tag value lives in one positional field occurrence.
    IndexedField {
        /// The repeated field name containing positional metadata.
        field_name: &'static str,
        /// The zero-based occurrence index containing the tag value.
        index: usize,
        /// An optional child kind to ignore before selecting the indexed field.
        excluded_kind: Option<&'static str>,
        /// Whether the surface uses one comma-delimited tag string.
        comma_separated: bool,
    },
}

/// Returns the shared tag-surface description for one syntax node kind.
#[must_use]
pub fn tag_surface_for_node_kind(node_kind: &str) -> Option<TagSurface> {
    match node_kind {
        "tag_statement" => Some(TagSurface::TagStatement),
        "tags_statement" => Some(TagSurface::TagsStatement),
        "person"
        | "software_system"
        | "container"
        | "component"
        | "software_system_instance"
        | "container_instance"
        | "instance_of"
        | "filtered_view" => Some(TagSurface::NamedField {
            field_name: "tags",
            comma_separated: true,
        }),
        "element_style" | "relationship_style" => Some(TagSurface::NamedField {
            field_name: "tag",
            comma_separated: false,
        }),
        "relationship" | "custom_element" => Some(TagSurface::IndexedField {
            field_name: "attribute",
            index: 2,
            excluded_kind: None,
            comma_separated: true,
        }),
        "deployment_node" | "infrastructure_node" => Some(TagSurface::IndexedField {
            field_name: "attribute",
            index: 2,
            excluded_kind: Some("number"),
            comma_separated: true,
        }),
        "archetype_instance" => Some(TagSurface::IndexedField {
            field_name: "metadata",
            index: 2,
            excluded_kind: None,
            comma_separated: true,
        }),
        _ => None,
    }
}
