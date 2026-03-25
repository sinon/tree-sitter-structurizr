/**
 * @file Grammar for Structurizr DSL for describing c4 models
 * @author Rob Hand <146272+sinon@users.noreply.github.com>
 * @license MIT OR Apache-2.0
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const NAMED_COLORS = [
  "aliceblue",
  "antiquewhite",
  "aqua",
  "aquamarine",
  "azure",
  "beige",
  "bisque",
  "black",
  "blanchedalmond",
  "blue",
  "blueviolet",
  "brown",
  "burlywood",
  "cadetblue",
  "chartreuse",
  "chocolate",
  "coral",
  "cornflowerblue",
  "cornsilk",
  "crimson",
  "cyan",
  "darkblue",
  "darkcyan",
  "darkgoldenrod",
  "darkgray",
  "darkgreen",
  "darkgrey",
  "darkkhaki",
  "darkmagenta",
  "darkolivegreen",
  "darkorange",
  "darkorchid",
  "darkred",
  "darksalmon",
  "darkseagreen",
  "darkslateblue",
  "darkslategray",
  "darkslategrey",
  "darkturquoise",
  "darkviolet",
  "deeppink",
  "deepskyblue",
  "dimgray",
  "dimgrey",
  "dodgerblue",
  "firebrick",
  "floralwhite",
  "forestgreen",
  "fuchsia",
  "gainsboro",
  "ghostwhite",
  "gold",
  "goldenrod",
  "gray",
  "green",
  "greenyellow",
  "grey",
  "honeydew",
  "hotpink",
  "indianred",
  "indigo",
  "ivory",
  "khaki",
  "lavender",
  "lavenderblush",
  "lawngreen",
  "lemonchiffon",
  "lightblue",
  "lightcoral",
  "lightcyan",
  "lightgoldenrodyellow",
  "lightgray",
  "lightgreen",
  "lightgrey",
  "lightpink",
  "lightsalmon",
  "lightseagreen",
  "lightskyblue",
  "lightslategray",
  "lightslategrey",
  "lightsteelblue",
  "lightyellow",
  "lime",
  "limegreen",
  "linen",
  "magenta",
  "maroon",
  "mediumaquamarine",
  "mediumblue",
  "mediumorchid",
  "mediumpurple",
  "mediumseagreen",
  "mediumslateblue",
  "mediumspringgreen",
  "mediumturquoise",
  "mediumvioletred",
  "midnightblue",
  "mintcream",
  "mistyrose",
  "moccasin",
  "navajowhite",
  "navy",
  "oldlace",
  "olive",
  "olivedrab",
  "orange",
  "orangered",
  "orchid",
  "palegoldenrod",
  "palegreen",
  "paleturquoise",
  "palevioletred",
  "papayawhip",
  "peachpuff",
  "peru",
  "pink",
  "plum",
  "powderblue",
  "purple",
  "rebeccapurple",
  "red",
  "rosybrown",
  "royalblue",
  "saddlebrown",
  "salmon",
  "sandybrown",
  "seagreen",
  "seashell",
  "sienna",
  "silver",
  "skyblue",
  "slateblue",
  "slategray",
  "slategrey",
  "snow",
  "springgreen",
  "steelblue",
  "tan",
  "teal",
  "thistle",
  "tomato",
  "turquoise",
  "violet",
  "wheat",
  "white",
  "whitesmoke",
  "yellow",
  "yellowgreen",
];

const COLOR_STYLE_PROPERTIES = [
  "background",
  "color",
  "colour",
  "stroke",
];

export default grammar({
  name: "structurizr",

  extras: $ => [
    /\s/,
    $.comment,
    $._line_continuation,
  ],

  conflicts: $ => [
    [$.container_instance_simple, $.container_instance_grouped],
    [$.software_system_instance_simple, $.software_system_instance_grouped],
    [$.person],
    [$.software_system],
    [$.container],
    [$.component],
  ],

  rules: {
    // Structurizr files are mostly a sequence of top-level blocks and directives,
    // with `workspace { ... }` as the usual outer envelope.
    source_file: $ => repeat($._definition),

    // The DSL accepts line comments and C-style block comments. Hash comments are
    // only treated as comments when followed by whitespace so color values such as
    // `#ffffff` remain available to styles.
    comment: _ => token(choice(
      seq("//", /.*/),
      seq("#", /[ \t].*/),
      seq(
        "/*",
        /[^*]*\*+([^/*][^*]*\*+)*/,
        "/",
      ),
    )),

    _line_continuation: _ => token(seq("\\", /\r?\n/, /[ \t]*/)),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_.-]*/,

    _assignment_identifier: $ => prec(1, choice(
      $.identifier,
      alias("group", $.identifier),
      alias("person", $.identifier),
      alias("softwareSystem", $.identifier),
      alias("softwaresystem", $.identifier),
      alias("container", $.identifier),
      alias("component", $.identifier),
      alias("deploymentEnvironment", $.identifier),
      alias("deploymentGroup", $.identifier),
      alias("deploymentNode", $.identifier),
      alias("infrastructureNode", $.identifier),
      alias("containerInstance", $.identifier),
      alias("softwareSystemInstance", $.identifier),
    )),

    number: _ => /\d+/,

    hex_color: _ => token(prec(2, /#[A-Fa-f0-9]{6}/)),

    named_color: _ => token(prec(2, choice(...NAMED_COLORS))),

    bare_value: _ => /[^\s{}"]+/,

    string: _ => token(seq(
      '"',
      repeat(choice(
        /[^"\\\n]+/,
        /\\./,
        seq("\\", /\r?\n/, /[ \t]*/),
      )),
      '"',
    )),

    text_block_string: _ => token(seq(
      '"""',
      repeat(choice(
        /[^"]+/,
        /"[^"]/,
        /""[^"]/,
      )),
      '"""',
    )),

    _value: $ => choice(
      $.string,
      $.identifier,
    ),

    _metadata_value: $ => $.string,

    _tag_value: $ => choice(
      $.string,
      $.identifier,
    ),

    _directive_value: $ => choice(
      $.string,
      $.text_block_string,
      $.bare_value,
      $.identifier,
    ),

    _definition: $ => choice(
      $.workspace,
      $.model,
      $.views,
      $.include_directive,
      $.const_directive,
      $.constant_directive,
      $.var_directive,
      $.identifiers_directive,
      $.implied_relationships_directive,
    ),

    // A workspace can be declared bare, named/described inline, or extend another
    // workspace. Most of the rest of the language hangs off this envelope.
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

    // The spec organizes content into `model`, `views`, and optional supporting
    // directives/configuration, so the workspace block keeps those concerns separate.
    workspace_block: $ => seq(
      "{",
      repeat(choice(
        $.include_directive,
        $.const_directive,
        $.constant_directive,
        $.var_directive,
        $.identifiers_directive,
        $.implied_relationships_directive,
        $.docs_directive,
        $.adrs_directive,
        $.name_statement,
        $.description_statement,
        $.properties_block,
        $.model,
        $.views,
        $.configuration,
      )),
      "}",
    ),

    // The model section is where people, systems, containers, components,
    // relationships, and custom/archetyped elements are declared.
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
      $.group,
      $.person,
      $.software_system,
      $.custom_element,
      $.archetype_instance,
      $.deployment_environment,
      $.include_directive,
      $.const_directive,
      $.constant_directive,
      $.var_directive,
      $.elements_directive,
      $.element_directive,
      $.identifiers_directive,
      $.properties_block,
      $.relationship,
    ),

    _software_system_item: $ => choice(
      $.group,
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
      $.group,
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
      $.group,
      $.elements_directive,
      $.element_directive,
      $.relationship,
      $.description_statement,
      $.technology_statement,
      $.tag_statement,
      $.tags_statement,
      $.metadata_statement,
    ),

    _deployment_item: $ => choice(
      $.group,
      $.deployment_group,
      $.deployment_node,
      $.relationship,
    ),

    _deployment_node_item: $ => choice(
      $.group,
      $.deployment_node,
      $.infrastructure_node,
      $.container_instance,
      $.software_system_instance,
      $.instance_of,
      $.relationship,
      $.tag_statement,
      $.tags_statement,
    ),

    _custom_block_item: $ => choice(
      $.group,
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

    _group_item: $ => choice(
      $.group,
      $.person,
      $.software_system,
      $.container,
      $.component,
      $.custom_element,
      $.archetype_instance,
      $.deployment_node,
      $.infrastructure_node,
      $.container_instance,
      $.software_system_instance,
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

    // Structurizr model elements share a common shape: optional identifier
    // assignment, a keyword, a few positional metadata slots, and an optional body.
    person: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
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
          field("tags", $._tag_value),
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
          field("tags", $._tag_value),
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
        field("identifier", $._assignment_identifier),
        "=",
      )),
      choice(
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
        ),
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._tag_value),
        ),
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
          field("body", $.software_system_block),
        ),
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
          field("description", $._metadata_value),
          field("body", $.software_system_block),
        ),
        seq(
          choice("softwareSystem", "softwaresystem"),
          field("name", $._value),
          field("description", $._metadata_value),
          field("tags", $._tag_value),
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
        field("identifier", $._assignment_identifier),
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
          field("tags", $._tag_value),
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
          field("tags", $._tag_value),
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
        field("identifier", $._assignment_identifier),
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
          field("tags", $._tag_value),
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
          field("tags", $._tag_value),
          field("body", $.component_block),
        ),
      ),
    ),

    component_block: $ => seq(
      "{",
      repeat($._component_item),
      "}",
    ),

    group: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "group",
      field("name", $._value),
      optional(field("body", $.group_block)),
    ),

    group_block: $ => seq(
      "{",
      repeat($._group_item),
      "}",
    ),

    deployment_environment: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "deploymentEnvironment",
      field("name", $._value),
      field("body", $.deployment_environment_block),
    ),

    deployment_environment_block: $ => seq(
      "{",
      repeat($._deployment_item),
      "}",
    ),

    deployment_group: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "deploymentGroup",
      field("name", $._value),
    ),

    deployment_node: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "deploymentNode",
      field("name", $._value),
      repeat(field("attribute", $._deployment_node_attribute)),
      optional(field("body", $.deployment_node_block)),
    ),

    infrastructure_node: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "infrastructureNode",
      field("name", $._value),
      repeat(field("attribute", $._deployment_node_attribute)),
      optional(field("body", $.deployment_node_block)),
    ),

    _deployment_node_attribute: $ => choice(
      $._metadata_value,
      $.number,
    ),

    deployment_node_block: $ => seq(
      "{",
      repeat($._deployment_node_item),
      "}",
    ),

    container_instance: $ => choice(
      $.container_instance_simple,
      $.container_instance_grouped,
    ),

    container_instance_simple: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "containerInstance",
      field("target", $.identifier),
    ),

    container_instance_grouped: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "containerInstance",
      field("target", $.identifier),
      field("deployment_group", $.identifier),
    ),

    software_system_instance: $ => choice(
      $.software_system_instance_simple,
      $.software_system_instance_grouped,
    ),

    software_system_instance_simple: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      "softwareSystemInstance",
      field("target", $.identifier),
    ),

    software_system_instance_grouped: $ => seq(
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      "softwareSystemInstance",
      field("target", $.identifier),
      field("deployment_group", $.identifier),
    ),

    instance_of: $ => seq(
      optional(seq(
        field("identifier", $._assignment_identifier),
        "=",
      )),
      "instanceOf",
      field("target", $.identifier),
    ),

    // Relationships appear both as top-level model statements and nested inside
    // element bodies. The grammar keeps them permissive enough to cover plain `->`
    // as well as archetyped operators like `--https->`.
    relationship: $ => prec.right(choice(
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
        field("attribute", $._tag_value),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
        optional(field("body", $.relationship_block)),
      ),
      seq(
        optional(seq(
          field("identifier", $._assignment_identifier),
          "=",
        )),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
        field("attribute", $._tag_value),
        optional(field("body", $.relationship_block)),
      ),
    )),

    _relationship_endpoint: $ => choice(
      $.identifier,
      $.this_keyword,
    ),

    this_keyword: _ => "this",

    relationship_operator: $ => choice(
      "->",
      "-/>",
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
        $.properties_block,
        $.perspectives_block,
        $.nested_relationship,
      )),
      "}",
    ),

    nested_relationship: $ => choice(
      seq(
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
      ),
      seq(
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
      ),
      seq(
        field("source", $._relationship_endpoint),
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
      ),
      seq(
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
      ),
      seq(
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
      ),
      seq(
        field("operator", $.relationship_operator),
        field("destination", $._relationship_endpoint),
        field("attribute", $._metadata_value),
        field("attribute", $._metadata_value),
      ),
    ),

    // Archetypes and custom elements are Structurizr's extension points. This slice
    // aims to preserve their overall shape for editor tooling, even where richer
    // semantics are still being filled in.
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
      optional(seq(
        field("identifier", $.identifier),
        "=",
      )),
      field("base", $.archetype_base),
      optional(field("body", $.archetype_body)),
    ),

    archetype_base: $ => choice(
      "person",
      choice("softwareSystem", "softwaresystem"),
      "container",
      "component",
      "element",
      "group",
      $.relationship_operator,
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
        $.properties_block,
        $.perspectives_block,
      )),
      "}",
    ),

    custom_element: $ => seq(
      optional(seq(field("identifier", $._assignment_identifier), "=")),
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
      optional(seq(field("identifier", $._assignment_identifier), "=")),
      field("kind", $.identifier),
      field("name", $._value),
      repeat(field("metadata", $._metadata_value)),
      optional(field("body", $.custom_element_block)),
    ),

    element_directive: $ => seq(
      optional(seq(field("identifier", $._assignment_identifier), "=")),
      "!element",
      field("target", $._directive_value),
      field("body", $.element_directive_block),
    ),

    element_directive_block: $ => seq(
      "{",
      repeat(choice(
        $._custom_block_item,
        $.deployment_node,
        $.infrastructure_node,
        $.container_instance,
        $.software_system_instance,
        $.properties_block,
        $.perspectives_block,
      )),
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

    // The views section contains several related families of view definitions that
    // mostly differ by their header fields while sharing a common set of statements.
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
      $.properties_block,
      $.const_directive,
      $.constant_directive,
      $.var_directive,
      $.styles,
      $.theme_statement,
      $.themes_statement,
    ),

    _view_value: $ => choice(
      $.identifier,
      $.string,
      $.wildcard,
      $.bare_value,
    ),

    wildcard: _ => choice("*", "*?"),

    layout_direction: _ => choice("tb", "bt", "lr", "rl"),

    _static_view_statement: $ => choice(
      $.include_statement,
      $.exclude_statement,
      $.animation_statement,
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
      $.dynamic_relationship_reference,
      $.dynamic_parallel_block,
      $.auto_layout_statement,
      $.default_statement,
      $.title_statement,
      $.description_statement,
    ),

    _advanced_view_statement: $ => choice(
      $.include_statement,
      $.exclude_statement,
      $.animation_statement,
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
      choice("autoLayout", "autolayout"),
      seq(choice("autoLayout", "autolayout"), field("direction", $.layout_direction)),
      seq(choice("autoLayout", "autolayout"), field("direction", $.layout_direction), field("rank_separation", $.number)),
      seq(
        choice("autoLayout", "autolayout"),
        field("direction", $.layout_direction),
        field("rank_separation", $.number),
        field("node_separation", $.number),
      ),
    ),

    default_statement: _ => "default",

    animation_statement: $ => seq(
      "animation",
      field("body", $.animation_block),
    ),

    animation_block: $ => seq(
      "{",
      repeat(field("value", $._animation_value)),
      "}",
    ),

    _animation_value: $ => choice(
      $.identifier,
      $.bare_value,
      $.wildcard,
      $.string,
    ),

    // These directives are intentionally modeled as lightweight syntactic forms.
    // They matter for editor tooling and fixture coverage even when downstream
    // consumers do not execute or resolve them.
    include_directive: $ => seq(
      "!include",
      field("value", $._directive_value),
    ),

    const_directive: $ => seq(
      "!const",
      field("name", $._directive_value),
      field("value", $._directive_value),
    ),

    constant_directive: $ => seq(
      "!constant",
      field("name", $._directive_value),
      field("value", $._directive_value),
    ),

    var_directive: $ => seq(
      "!var",
      field("name", $._directive_value),
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
        field("tags", $._tag_value),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._tag_value),
        field("key", $._value),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._tag_value),
        field("key", $._value),
        field("description", $._metadata_value),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._tag_value),
        field("body", $.filtered_view_block),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._tag_value),
        field("key", $._value),
        field("body", $.filtered_view_block),
      ),
      seq(
        "filtered",
        field("base_key", $._value),
        field("mode", $.identifier),
        field("tags", $._tag_value),
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

    dynamic_relationship_reference: $ => seq(
      optional(seq(field("order", $.order), ":")),
      field("relationship", $.identifier),
      field("description", $._metadata_value),
    ),

    dynamic_parallel_block: $ => seq(
      "{",
      repeat1(choice(
        $.dynamic_relationship,
        $.dynamic_relationship_reference,
        $.dynamic_parallel_block,
      )),
      "}",
    ),

    // `custom` and `image` views sit outside the core C4 view set but are important
    // to parse because they show up in real workspaces and affect editor behavior.
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

    theme_statement: $ => seq(
      "theme",
      field("value", $._directive_value),
    ),

    themes_statement: $ => seq(
      "themes",
      repeat1(field("value", $._directive_value)),
    ),

    // Style rules are effectively key/value bags scoped by tag, so the grammar keeps
    // them deliberately generic instead of trying to encode every allowed property.
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

    color_value: $ => choice(
      $.hex_color,
      $.named_color,
    ),

    style_setting: $ => choice(
      prec(1, seq(
        field("name", alias(choice(...COLOR_STYLE_PROPERTIES), $.identifier)),
        field("value", $.color_value),
      )),
      seq(
        field("name", $.identifier),
        field("value", $._style_value),
      ),
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

    perspectives_block: $ => seq(
      "perspectives",
      "{",
      repeat($.perspective_entry),
      "}",
    ),

    perspective_entry: $ => seq(
      field("name", $._directive_value),
      field("description", $._directive_value),
    ),

    // Image views can point at several source syntaxes; these are modeled as small,
    // explicit statements so query authors can target them later.
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

    // Configuration is comparatively small in the DSL, but it affects visibility and
    // audience semantics that downstream tools may want to surface.
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
