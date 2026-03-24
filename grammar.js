/**
 * @file Grammar for Structurizr DSL for describing c4 models
 * @author Rob Hand <146272+sinon@users.noreply.github.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "structurizr",

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat($._definition),

    comment: _ => token(choice(
      seq("//", /.*/),
      seq("#", /[ \t].*/),
      seq(
        "/*",
        /[^*]*\*+([^/*][^*]*\*+)*/,
        "/",
      ),
    )),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_.-]*/,

    number: _ => /\d+/,

    bare_value: _ => /[^\s{}"]+/,

    string: _ => token(seq(
      '"',
      repeat(choice(
        /[^"\\\n]+/,
        /\\./,
      )),
      '"',
    )),

    _value: $ => choice(
      $.string,
      $.identifier,
    ),

    _metadata_value: $ => $.string,

    _directive_value: $ => choice(
      $.string,
      $.bare_value,
      $.identifier,
    ),

    _definition: $ => choice(
      $.workspace,
      $.model,
      $.views,
      $.include_directive,
      $.identifiers_directive,
      $.implied_relationships_directive,
    ),

    workspace: $ => choice(
      seq(
        "workspace",
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        field("name", $._value),
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        field("name", $._value),
        field("description", $._value),
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        "extends",
        field("base", $._value),
        field("body", $.workspace_block),
      ),
    ),

    workspace_block: $ => seq(
      "{",
      repeat(choice(
        $.include_directive,
        $.identifiers_directive,
        $.implied_relationships_directive,
        $.docs_directive,
        $.adrs_directive,
        $.name_statement,
        $.description_statement,
        $.model,
        $.views,
        $.configuration,
      )),
      "}",
    ),

    model: $ => seq(
      "model",
      field("body", $.model_block),
    ),

    model_block: $ => seq(
      "{",
      repeat($._model_item),
      "}",
    ),

    views: $ => seq(
      "views",
      field("body", $.views_block),
    ),

    views_block: $ => seq(
      "{",
      repeat($._view_item),
      "}",
    ),

    name_statement: $ => seq(
      "name",
      field("value", $._value),
    ),

    description_statement: $ => seq(
      "description",
      field("value", $._value),
    ),

    technology_statement: $ => seq(
      "technology",
      field("value", $._value),
    ),

    tags_statement: $ => seq(
      "tags",
      field("value", $._value),
    ),

    tag_statement: $ => seq(
      "tag",
      field("value", $._value),
    ),

    metadata_statement: $ => seq(
      "metadata",
      field("value", $._value),
    ),

    title_statement: $ => seq(
      "title",
      field("value", $._value),
    ),

    _model_item: $ => choice(
      $.archetypes,
      $.person,
      $.software_system,
      $.custom_element,
      $.archetype_instance,
      $.elements_directive,
      $.element_directive,
      $.identifiers_directive,
      $.relationship,
    ),

    _software_system_item: $ => choice(
      $.container,
      $.custom_element,
      $.archetype_instance,
      $.elements_directive,
      $.element_directive,
      $.relationship,
      $.docs_directive,
      $.adrs_directive,
      $.description_statement,
      $.tag_statement,
      $.tags_statement,
      $.metadata_statement,
    ),

    _container_item: $ => choice(
      $.component,
      $.custom_element,
      $.archetype_instance,
      $.elements_directive,
      $.element_directive,
      $.relationship,
      $.docs_directive,
      $.adrs_directive,
      $.description_statement,
      $.technology_statement,
      $.tag_statement,
      $.tags_statement,
      $.metadata_statement,
    ),

    _component_item: $ => choice(
      $.elements_directive,
      $.element_directive,
      $.relationship,
      $.description_statement,
      $.technology_statement,
      $.tag_statement,
      $.tags_statement,
      $.metadata_statement,
    ),

    _custom_block_item: $ => choice(
      $.person,
      $.software_system,
      $.container,
      $.component,
      $.custom_element,
      $.archetype_instance,
      $.elements_directive,
      $.element_directive,
      $.relationship,
      $.description_statement,
      $.technology_statement,
      $.tag_statement,
      $.tags_statement,
      $.metadata_statement,
      $.docs_directive,
      $.adrs_directive,
    ),

    person: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      choice(
        seq(
          "person",
          field("name", $._value),
        ),
        seq(
          "person",
          field("name", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          "person",
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._metadata_value),
        ),
        seq(
          "person",
          field("name", $._value),
          field("body", $.person_block),
        ),
        seq(
          "person",
          field("name", $._value),
          field("description", $._metadata_value),
          field("body", $.person_block),
        ),
        seq(
          "person",
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._metadata_value),
          field("body", $.person_block),
        ),
      ),
    ),

    person_block: $ => seq(
      "{",
      repeat(choice(
        $.description_statement,
        $.tag_statement,
        $.tags_statement,
        $.relationship,
        $.metadata_statement,
      )),
      "}",
    ),

    software_system: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      choice(
        seq(
          "softwareSystem",
          field("name", $._value),
        ),
        seq(
          "softwareSystem",
          field("name", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          "softwareSystem",
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._metadata_value),
        ),
        seq(
          "softwareSystem",
          field("name", $._value),
          field("body", $.software_system_block),
        ),
        seq(
          "softwareSystem",
          field("name", $._value),
          field("description", $._metadata_value),
          field("body", $.software_system_block),
        ),
        seq(
          "softwareSystem",
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._metadata_value),
          field("body", $.software_system_block),
        ),
      ),
    ),

    software_system_block: $ => seq(
      "{",
      repeat($._software_system_item),
      "}",
    ),

    container: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      choice(
        seq(
          "container",
          field("name", $._value),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("tags", $._metadata_value),
        ),
        seq(
          "container",
          field("name", $._value),
          field("body", $.container_block),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
          field("body", $.container_block),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("body", $.container_block),
        ),
        seq(
          "container",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("tags", $._metadata_value),
          field("body", $.container_block),
        ),
      ),
    ),

    container_block: $ => seq(
      "{",
      repeat($._container_item),
      "}",
    ),

    component: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      choice(
        seq(
          "component",
          field("name", $._value),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("tags", $._metadata_value),
        ),
        seq(
          "component",
          field("name", $._value),
          field("body", $.component_block),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
          field("body", $.component_block),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("body", $.component_block),
        ),
        seq(
          "component",
          field("name", $._value),
          field("description", $._metadata_value),
          field("technology", $._metadata_value),
          field("tags", $._metadata_value),
          field("body", $.component_block),
        ),
      ),
    ),

    component_block: $ => seq(
      "{",
      repeat($._component_item),
      "}",
    ),

    relationship: $ => choice(
      seq(
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        repeat(field("attribute", $._metadata_value)),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        repeat(field("attribute", $._metadata_value)),
        optional(field("body", $.relationship_block)),
      ),
    ),

    _relationship_endpoint: $ => choice(
      $.identifier,
      $.this_keyword,
    ),

    this_keyword: _ => "this",

    relationship_operator: $ => choice(
      "->",
      seq("--", field("archetype", $.relationship_archetype_name), "->"),
    ),

    relationship_archetype_name: _ => /[A-Za-z_][A-Za-z0-9_.]*/,

    relationship_block: $ => seq(
      "{",
      repeat(choice(
        $.tag_statement,
        $.tags_statement,
        $.description_statement,
        $.technology_statement,
      )),
      "}",
    ),

    archetypes: $ => seq(
      "archetypes",
      field("body", $.archetypes_block),
    ),

    archetypes_block: $ => seq(
      "{",
      repeat($.archetype_definition),
      "}",
    ),

    archetype_definition: $ => seq(
      field("identifier", $.identifier),
      "=",
      field("base", $.archetype_base),
      optional(field("body", $.archetype_body)),
    ),

    archetype_base: $ => choice(
      "person",
      "softwareSystem",
      "container",
      "component",
      "element",
      "group",
      "->",
      $.identifier,
    ),

    archetype_body: $ => seq(
      "{",
      repeat(choice(
        $.description_statement,
        $.technology_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
      )),
      "}",
    ),

    custom_element: $ => seq(
      optional(seq(field("identifier", $.identifier), "=")),
      "element",
      field("name", $._value),
      repeat(field("attribute", $._metadata_value)),
      optional(field("body", $.custom_element_block)),
    ),

    custom_element_block: $ => seq(
      "{",
      repeat($._custom_block_item),
      "}",
    ),

    archetype_instance: $ => seq(
      optional(seq(field("identifier", $.identifier), "=")),
      field("kind", $.identifier),
      field("name", $._value),
      repeat(field("metadata", $._metadata_value)),
      optional(field("body", $.custom_element_block)),
    ),

    element_directive: $ => seq(
      optional(seq(field("identifier", $.identifier), "=")),
      "!element",
      field("target", $._directive_value),
      field("body", $.element_directive_block),
    ),

    element_directive_block: $ => seq(
      "{",
      repeat($._custom_block_item),
      "}",
    ),

    elements_directive: $ => seq(
      "!elements",
      field("expression", $._directive_value),
      field("body", $.elements_block),
    ),

    elements_block: $ => seq(
      "{",
      repeat(choice(
        $.relationship,
        $.tag_statement,
        $.tags_statement,
        $.description_statement,
        $.technology_statement,
      )),
      "}",
    ),

    _view_item: $ => choice(
      $.system_landscape_view,
      $.system_context_view,
      $.container_view,
      $.component_view,
      $.filtered_view,
      $.dynamic_view,
      $.deployment_view,
      $.custom_view,
      $.image_view,
      $.styles,
    ),

    _view_value: $ => choice(
      $.identifier,
      $.string,
      $.wildcard,
    ),

    wildcard: _ => choice("*", "*?"),

    layout_direction: _ => choice("tb", "bt", "lr", "rl"),

    _static_view_statement: $ => choice(
      $.include_statement,
      $.exclude_statement,
      $.auto_layout_statement,
      $.default_statement,
      $.title_statement,
      $.description_statement,
    ),

    _filtered_view_statement: $ => choice(
      $.default_statement,
      $.title_statement,
      $.description_statement,
    ),

    _dynamic_view_statement: $ => choice(
      $.dynamic_relationship,
      $.auto_layout_statement,
      $.default_statement,
      $.title_statement,
      $.description_statement,
    ),

    _advanced_view_statement: $ => choice(
      $.include_statement,
      $.exclude_statement,
      $.auto_layout_statement,
      $.default_statement,
      $.title_statement,
      $.description_statement,
    ),

    include_statement: $ => seq(
      "include",
      repeat1(field("value", $._view_value)),
    ),

    exclude_statement: $ => seq(
      "exclude",
      repeat1(field("value", $._view_value)),
    ),

    auto_layout_statement: $ => choice(
      "autoLayout",
      seq("autoLayout", field("direction", $.layout_direction)),
      seq("autoLayout", field("direction", $.layout_direction), field("rank_separation", $.number)),
      seq(
        "autoLayout",
        field("direction", $.layout_direction),
        field("rank_separation", $.number),
        field("node_separation", $.number),
      ),
    ),

    default_statement: _ => "default",

    include_directive: $ => seq(
      "!include",
      field("value", $._directive_value),
    ),

    identifiers_directive: $ => seq(
      "!identifiers",
      field("value", $._directive_value),
    ),

    implied_relationships_directive: $ => seq(
      "!impliedRelationships",
      field("value", $._directive_value),
    ),

    docs_directive: $ => seq(
      "!docs",
      field("path", $._directive_value),
    ),

    adrs_directive: $ => seq(
      "!adrs",
      field("path", $._directive_value),
    ),

    system_landscape_view: $ => choice(
      seq("systemLandscape", field("body", $.system_landscape_view_block)),
      seq("systemLandscape", field("key", $._value), field("body", $.system_landscape_view_block)),
      seq(
        "systemLandscape",
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.system_landscape_view_block),
      ),
    ),

    system_landscape_view_block: $ => seq(
      "{",
      repeat($._static_view_statement),
      "}",
    ),

    system_context_view: $ => choice(
      seq(
        "systemContext",
        field("scope", $.identifier),
        field("body", $.system_context_view_block),
      ),
      seq(
        "systemContext",
        field("scope", $.identifier),
        field("key", $._value),
        field("body", $.system_context_view_block),
      ),
      seq(
        "systemContext",
        field("scope", $.identifier),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.system_context_view_block),
      ),
    ),

    system_context_view_block: $ => seq(
      "{",
      repeat($._static_view_statement),
      "}",
    ),

    container_view: $ => choice(
      seq(
        "container",
        field("scope", $.identifier),
        field("body", $.container_view_block),
      ),
      seq(
        "container",
        field("scope", $.identifier),
        field("key", $._value),
        field("body", $.container_view_block),
      ),
      seq(
        "container",
        field("scope", $.identifier),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.container_view_block),
      ),
    ),

    container_view_block: $ => seq(
      "{",
      repeat($._static_view_statement),
      "}",
    ),

    component_view: $ => choice(
      seq(
        "component",
        field("scope", $.identifier),
        field("body", $.component_view_block),
      ),
      seq(
        "component",
        field("scope", $.identifier),
        field("key", $._value),
        field("body", $.component_view_block),
      ),
      seq(
        "component",
        field("scope", $.identifier),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.component_view_block),
      ),
    ),

    component_view_block: $ => seq(
      "{",
      repeat($._static_view_statement),
      "}",
    ),

    filtered_view: $ => choice(
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._metadata_value),
        field("body", $.filtered_view_block),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._metadata_value),
        field("key", $._value),
        field("body", $.filtered_view_block),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._metadata_value),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.filtered_view_block),
      ),
    ),

    filtered_view_block: $ => seq(
      "{",
      repeat($._filtered_view_statement),
      "}",
    ),

    dynamic_view: $ => choice(
      seq(
        "dynamic",
        field("scope", $._view_value),
        field("body", $.dynamic_view_block),
      ),
      seq(
        "dynamic",
        field("scope", $._view_value),
        field("key", $._value),
        field("body", $.dynamic_view_block),
      ),
      seq(
        "dynamic",
        field("scope", $._view_value),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.dynamic_view_block),
      ),
    ),

    dynamic_view_block: $ => seq(
      "{",
      repeat($._dynamic_view_statement),
      "}",
    ),

    order: _ => token(/[0-9]+(\.[0-9]+)*/),

    dynamic_relationship: $ => choice(
      seq(
        optional(seq(field("order", $.order), ":")),
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
      ),
      seq(
        optional(seq(field("order", $.order), ":")),
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
        field("description", $._metadata_value),
      ),
      seq(
        optional(seq(field("order", $.order), ":")),
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
        field("description", $._metadata_value),
        field("technology", $._metadata_value),
      ),
    ),

    deployment_view: $ => choice(
      seq(
        "deployment",
        field("scope", $._view_value),
        field("environment", $._value),
        field("body", $.deployment_view_block),
      ),
      seq(
        "deployment",
        field("scope", $._view_value),
        field("environment", $._value),
        field("key", $._value),
        field("body", $.deployment_view_block),
      ),
      seq(
        "deployment",
        field("scope", $._view_value),
        field("environment", $._value),
        field("key", $._value),
        field("description", $._metadata_value),
        field("body", $.deployment_view_block),
      ),
    ),

    deployment_view_block: $ => seq(
      "{",
      repeat($._advanced_view_statement),
      "}",
    ),

    custom_view: $ => choice(
      seq("custom", field("body", $.custom_view_block)),
      seq("custom", field("key", $._value), field("body", $.custom_view_block)),
      seq("custom", field("key", $._value), field("title", $._value), field("body", $.custom_view_block)),
      seq(
        "custom",
        field("key", $._value),
        field("title", $._value),
        field("description", $._metadata_value),
        field("body", $.custom_view_block),
      ),
    ),

    custom_view_block: $ => seq(
      "{",
      repeat($._advanced_view_statement),
      "}",
    ),

    image_view: $ => choice(
      seq(
        "image",
        field("scope", $._view_value),
        field("body", $.image_view_block),
      ),
      seq(
        "image",
        field("scope", $._view_value),
        field("key", $._value),
        field("body", $.image_view_block),
      ),
    ),

    image_view_block: $ => seq(
      "{",
      repeat(choice(
        $.plantuml_source,
        $.mermaid_source,
        $.kroki_source,
        $.image_source,
        $.default_statement,
        $.title_statement,
        $.description_statement,
      )),
      "}",
    ),

    styles: $ => seq(
      "styles",
      field("body", $.styles_block),
    ),

    styles_block: $ => seq(
      "{",
      repeat($._style_item),
      "}",
    ),

    _style_item: $ => choice(
      $.element_style,
      $.relationship_style,
      $.light_styles,
      $.dark_styles,
    ),

    light_styles: $ => seq(
      "light",
      field("body", $.style_mode_block),
    ),

    dark_styles: $ => seq(
      "dark",
      field("body", $.style_mode_block),
    ),

    style_mode_block: $ => seq(
      "{",
      repeat(choice(
        $.element_style,
        $.relationship_style,
      )),
      "}",
    ),

    element_style: $ => seq(
      "element",
      field("tag", $._value),
      field("body", $.style_rule_block),
    ),

    relationship_style: $ => seq(
      "relationship",
      field("tag", $._value),
      field("body", $.style_rule_block),
    ),

    style_rule_block: $ => seq(
      "{",
      repeat(choice(
        $.style_setting,
        $.properties_block,
      )),
      "}",
    ),

    style_setting: $ => seq(
      field("name", $.identifier),
      field("value", $._style_value),
    ),

    _style_value: $ => choice(
      $.string,
      $.number,
      $.identifier,
      $.bare_value,
    ),

    properties_block: $ => seq(
      "properties",
      "{",
      repeat($.property_entry),
      "}",
    ),

    property_entry: $ => seq(
      field("name", $._directive_value),
      field("value", $._directive_value),
    ),

    plantuml_source: $ => seq(
      "plantuml",
      field("value", $._directive_value),
    ),

    mermaid_source: $ => seq(
      "mermaid",
      field("value", $._directive_value),
    ),

    kroki_source: $ => seq(
      "kroki",
      field("format", $._directive_value),
      field("value", $._directive_value),
    ),

    image_source: $ => seq(
      "image",
      field("value", $._directive_value),
    ),

    configuration: $ => seq(
      "configuration",
      field("body", $.configuration_block),
    ),

    configuration_block: $ => seq(
      "{",
      repeat(choice(
        $.scope_statement,
        $.visibility_statement,
        $.users_block,
      )),
      "}",
    ),

    scope_statement: $ => seq(
      "scope",
      field("value", $._directive_value),
    ),

    visibility_statement: $ => seq(
      "visibility",
      field("value", $._directive_value),
    ),

    users_block: $ => seq(
      "users",
      "{",
      repeat($.user_entry),
      "}",
    ),

    user_entry: $ => seq(
      field("username", $._directive_value),
      field("role", $._directive_value),
    ),
  },
});
