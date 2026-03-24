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
      seq("#", /.*/),
    )),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_.-]*/,

    number: _ => /\d+/,

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

    _definition: $ => choice(
      $.workspace,
      $.model,
      $.views,
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
        $.name_statement,
        $.description_statement,
        $.model,
        $.views,
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

    title_statement: $ => seq(
      "title",
      field("value", $._value),
    ),

    _model_item: $ => choice(
      $.person,
      $.software_system,
      $.relationship,
    ),

    _software_system_item: $ => choice(
      $.container,
      $.relationship,
      $.description_statement,
      $.tags_statement,
    ),

    _container_item: $ => choice(
      $.component,
      $.relationship,
      $.description_statement,
      $.technology_statement,
      $.tags_statement,
    ),

    _component_item: $ => choice(
      $.relationship,
      $.description_statement,
      $.technology_statement,
      $.tags_statement,
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
        $.tags_statement,
        $.relationship,
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
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
      ),
      seq(
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
        field("description", $._metadata_value),
      ),
      seq(
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
        field("description", $._metadata_value),
        field("technology", $._metadata_value),
      ),
      seq(
        field("source", $.identifier),
        "->",
        field("destination", $.identifier),
        field("description", $._metadata_value),
        field("technology", $._metadata_value),
        field("tags", $._metadata_value),
      ),
    ),

    _view_item: $ => choice(
      $.system_landscape_view,
      $.system_context_view,
      $.container_view,
      $.component_view,
      $.filtered_view,
    ),

    _view_value: $ => choice(
      $.identifier,
      $.string,
      $.wildcard,
    ),

    wildcard: _ => choice("*", "*?"),

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
      seq("autoLayout", field("direction", $.identifier)),
      seq("autoLayout", field("direction", $.identifier), field("rank_separation", $.number)),
      seq(
        "autoLayout",
        field("direction", $.identifier),
        field("rank_separation", $.number),
        field("node_separation", $.number),
      ),
    ),

    default_statement: _ => "default",

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
  },
});
