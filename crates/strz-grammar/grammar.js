/**
 * @file Grammar for Structurizr DSL for describing c4 models
 * @author Rob Hand <146272+sinon@users.noreply.github.com>
 * @license MIT OR Apache-2.0
 */

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

const COLOR_STYLE_PROPERTIES = ["background", "color", "colour", "stroke"];

const BOOLEAN_VALUES = ["true", "false"];

const SHAPE_VALUES = [
  "Box",
  "RoundedBox",
  "Circle",
  "Ellipse",
  "Hexagon",
  "Diamond",
  "Cylinder",
  "Bucket",
  "Pipe",
  "Person",
  "Robot",
  "Folder",
  "WebBrowser",
  "Window",
  "Terminal",
  "Shell",
  "MobileDevicePortrait",
  "MobileDeviceLandscape",
  "Component",
];

const BORDER_VALUES = ["Solid", "Dashed", "Dotted"];

const ICON_POSITION_VALUES = ["Top", "Bottom", "Left"];

const LINE_STYLE_VALUES = ["Dashed", "Dotted", "Solid"];

const ROUTING_VALUES = ["Direct", "Curved", "Orthogonal"];

const FILTER_MODES = ["include", "exclude"];

function escapeRegexChar(char) {
  return char.replace(/[|\\{}()[\]^$+*?.-]/g, "\\$&");
}

function caseInsensitivePattern(value) {
  return value
    .split("")
    .map((char) =>
      /[A-Za-z]/.test(char)
        ? `[${char.toLowerCase()}${char.toUpperCase()}]`
        : escapeRegexChar(char),
    )
    .join("");
}

function enumValueChoices(values, { quoted = false } = {}) {
  return values.flatMap((value) => {
    const pattern = caseInsensitivePattern(value);
    return quoted
      ? [new RegExp(pattern), new RegExp(`"${pattern}"`)]
      : [new RegExp(pattern)];
  });
}

function deploymentInstanceBody($) {
  return field("body", $.deployment_instance_block);
}

function deploymentInstanceOptionalBody($) {
  return choice(
    prec(2, seq($._inline_gap, deploymentInstanceBody($))),
    prec(1, deploymentInstanceBody($)),
  );
}

function deploymentInstanceGroupAndTags($) {
  return seq(
    $._inline_gap,
    field("deployment_group", $._value),
    optional(seq($._inline_gap, field("tags", $._tag_value))),
  );
}

function deploymentInstanceGroupedTail($) {
  return seq(
    deploymentInstanceGroupAndTags($),
    optional(deploymentInstanceOptionalBody($)),
  );
}

function deploymentInstanceRule($, keyword) {
  // Upstream deployment instances accept three header shapes:
  //   1. `softwareSystemInstance system`
  //   2. `softwareSystemInstance system { ... }`
  //   3. `softwareSystemInstance system blue "Canary" { ... }`
  //
  // Keep those branches named here so future grammar work can reason about
  // target-only, direct-body, and grouped-header forms separately.
  return seq(
    optional(seq(field("identifier", $._assignment_identifier), "=")),
    keyword,
    $._inline_gap,
    field("target", $.identifier),
    optional(choice(deploymentInstanceOptionalBody($), deploymentInstanceGroupedTail($))),
  );
}

export default grammar({
  name: "structurizr",

  extras: ($) => [/\s/, $.comment, $._line_continuation],

  // Give downstream highlighters a canonical word token so literal keyword
  // patterns do not bleed into longer identifiers like `securityComponent`.
  word: ($) => $.identifier,

  conflicts: ($) => [
    [$.person],
    [$.software_system],
    [$.container],
    [$.component],
    [$._source_model_fragment_item, $._source_software_system_fragment_item],
  ],

  rules: {
    // Real-world DSL projects often split large models into include fragments that
    // start directly with block-body content rather than a wrapped `workspace`,
    // `model`, or `views` envelope. Accept those structural fragment forms at the
    // file root so editor tooling can parse included files on their own.
    source_file: ($) => repeat($._source_item),

    // The DSL accepts line comments and C-style block comments. Hash comments are
    // only treated as comments when followed by whitespace so color values such as
    // `#ffffff` remain available to styles.
    comment: (_) =>
      token(
        choice(
          seq("//", /.*/),
          seq("#", /[ \t].*/),
          seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
        ),
      ),

    _line_continuation: (_) => token(seq("\\", /\r?\n/, /[ \t]*/)),

    // The upstream parser tokenizes one logical statement per line unless the line
    // ends with an explicit continuation marker. Reuse a same-line gap for
    // deployment instance headers so optional group/tag slots do not absorb the next
    // deployment item on a following line.
    _inline_gap: (_) => token.immediate(choice(/[ \t]+/, seq("\\", /\r?\n/, /[ \t]*/))),

    identifier: (_) => /[A-Za-z_][A-Za-z0-9_.-]*/,

    _assignment_identifier: ($) =>
      prec(
        1,
        choice(
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
        ),
      ),

    number: (_) => /\d+/,

    hex_color: (_) => token(prec(2, /#[A-Fa-f0-9]{6}/)),

    named_color: (_) => token(prec(2, choice(...NAMED_COLORS))),

    boolean_value: (_) => token(prec(2, choice(...BOOLEAN_VALUES))),

    shape_value: (_) =>
      token(
        prec(2, choice(...enumValueChoices(SHAPE_VALUES, { quoted: true }))),
      ),

    border_value: (_) =>
      token(
        prec(2, choice(...enumValueChoices(BORDER_VALUES, { quoted: true }))),
      ),

    icon_position_value: (_) =>
      token(
        prec(
          2,
          choice(...enumValueChoices(ICON_POSITION_VALUES, { quoted: true })),
        ),
      ),

    line_style_value: (_) =>
      token(
        prec(
          2,
          choice(...enumValueChoices(LINE_STYLE_VALUES, { quoted: true })),
        ),
      ),

    routing_value: (_) =>
      token(
        prec(2, choice(...enumValueChoices(ROUTING_VALUES, { quoted: true }))),
      ),

    filter_mode: (_) => token(prec(2, choice(...FILTER_MODES))),

    bare_value: (_) => /[^\s{}"]+/,

    string: (_) =>
      token(
        seq(
          '"',
          repeat(choice(/[^"\\\n]+/, /\\./, seq("\\", /\r?\n/, /[ \t]*/))),
          '"',
        ),
      ),

    text_block_string: (_) =>
      token(seq('"""', repeat(choice(/[^"]+/, /"[^"]/, /""[^"]/)), '"""')),

    _value: ($) => choice($.string, $.identifier),

    _metadata_value: ($) => $.string,

    _tag_value: ($) => choice($.string, $.identifier),

    _relationship_tag_value: ($) =>
      choice($.string, alias(choice("Current", "Future"), $.identifier)),

    _directive_value: ($) =>
      choice($.string, $.text_block_string, $.bare_value, $.identifier),

    // Documentation importers and ADR importer selectors are identifier-shaped
    // values in the DSL: either a built-in token such as `adrtools` or a fully
    // qualified Java class name. Keeping them separate from `_directive_value`
    // preserves flexible path parsing without treating hyphenated bare values
    // such as `adr-tools` as first-class supported importer syntax.
    java_fully_qualified_name: (_) =>
      token(
        prec(
          3,
          seq(
            /[A-Za-z_$][A-Za-z0-9_$]*/,
            repeat1(seq(".", /[A-Za-z_$][A-Za-z0-9_$]*/)),
          ),
        ),
      ),

    decision_importer_type: (_) => choice("adrtools", "madr", "log4brains"),

    _definition: ($) =>
      choice(
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

    _source_item: ($) =>
      choice(
        $._definition,
        $._source_model_fragment_item,
        $._source_software_system_fragment_item,
        $._source_style_fragment_item,
      ),

    _source_model_fragment_item: ($) =>
      choice(
        $.group,
        $.person,
        $.software_system,
        $.deployment_environment,
        $.relationship,
      ),

    _source_software_system_fragment_item: ($) =>
      choice(
        $.group,
        $.container,
        $.deployment_environment,
        $.relationship,
        $.docs_directive,
        $.adrs_directive,
        $.description_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
      ),

    _source_style_fragment_item: ($) =>
      choice(
        $.element_style,
        $.relationship_style,
        $.light_styles,
        $.dark_styles,
        $.theme_statement,
        $.themes_statement,
      ),

    // A workspace can be declared bare, named/described inline, or extend another
    // workspace. Most of the rest of the language hangs off this envelope.
    workspace: ($) =>
      choice(
        seq("workspace", field("body", $.workspace_block)),
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
    workspace_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
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
          ),
        ),
        "}",
      ),

    // The model section is where people, systems, containers, components,
    // relationships, and custom/archetyped elements are declared.
    model: ($) => seq("model", field("body", $.model_block)),

    model_block: ($) => seq("{", repeat($._model_item), "}"),

    views: ($) => seq("views", field("body", $.views_block)),

    views_block: ($) => seq("{", repeat($._view_item), "}"),

    name_statement: ($) => seq("name", field("value", $._value)),

    description_statement: ($) => seq("description", field("value", $._value)),

    technology_statement: ($) => seq("technology", field("value", $._value)),

    tags_statement: ($) => seq("tags", field("value", $._value)),

    tag_statement: ($) => seq("tag", field("value", $._value)),

    metadata_statement: ($) => seq("metadata", field("value", $._value)),

    url_statement: ($) => seq("url", field("value", $._directive_value)),

    value_statement: ($) => seq("value", field("value", $._directive_value)),

    title_statement: ($) => seq("title", field("value", $._value)),

    _model_item: ($) =>
      choice(
        $.archetypes,
        $.enterprise,
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
        $.relationships_directive,
        $.element_directive,
        $.identifiers_directive,
        $.implied_relationships_directive,
        $.properties_block,
        $.relationship,
      ),

    _software_system_item: ($) =>
      choice(
        $.group,
        $.container,
        $.custom_element,
        $.archetype_instance,
        $.include_directive,
        $.deployment_environment,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.docs_directive,
        $.adrs_directive,
        $.description_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
      ),

    _container_item: ($) =>
      choice(
        $.group,
        $.component,
        $.custom_element,
        $.archetype_instance,
        $.include_directive,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.docs_directive,
        $.adrs_directive,
        $.description_statement,
        $.technology_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
      ),

    _component_item: ($) =>
      choice(
        $.group,
        $.include_directive,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.description_statement,
        $.technology_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
      ),

    _deployment_item: ($) =>
      choice(
        $.group,
        $.include_directive,
        $.deployment_group,
        $.deployment_node,
        $.relationship,
      ),

    _deployment_node_item: ($) =>
      choice(
        $.group,
        $.include_directive,
        $.deployment_node,
        $.infrastructure_node,
        $.container_instance,
        $.software_system_instance,
        $.instance_of,
        $.relationship,
        $.tag_statement,
        $.tags_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
      ),

    _custom_block_item: ($) =>
      choice(
        $.group,
        $.person,
        $.software_system,
        $.container,
        $.component,
        $.custom_element,
        $.archetype_instance,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.description_statement,
        $.technology_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
        $.docs_directive,
        $.adrs_directive,
      ),

    _group_item: ($) =>
      choice(
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
        $.include_directive,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.description_statement,
        $.technology_statement,
        $.tag_statement,
        $.tags_statement,
        $.metadata_statement,
        $.url_statement,
        $.properties_block,
        $.perspectives_block,
        $.docs_directive,
        $.adrs_directive,
      ),

    _enterprise_item: ($) =>
      choice(
        $.group,
        $.person,
        $.software_system,
        $.custom_element,
        $.archetype_instance,
        $.include_directive,
        $.elements_directive,
        $.relationships_directive,
        $.element_directive,
        $.relationship,
        $.properties_block,
        $.docs_directive,
        $.adrs_directive,
      ),

    // Structurizr model elements share a common shape: optional identifier
    // assignment, a keyword, a few positional metadata slots, and an optional body.
    person: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        choice(
          seq("person", field("name", $._value)),
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
          seq("person", field("name", $._value), field("body", $.person_block)),
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

    person_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.description_statement,
            $.tag_statement,
            $.tags_statement,
            $.relationship,
            $.metadata_statement,
            $.url_statement,
            $.properties_block,
            $.perspectives_block,
          ),
        ),
        "}",
      ),

    software_system: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
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

    software_system_block: ($) =>
      seq("{", repeat($._software_system_item), "}"),

    container: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        choice(
          seq("container", field("name", $._value)),
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

    container_block: ($) => seq("{", repeat($._container_item), "}"),

    component: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        choice(
          seq("component", field("name", $._value)),
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

    component_block: ($) => seq("{", repeat($._component_item), "}"),

    group: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "group",
        field("name", $._value),
        optional(field("body", $.group_block)),
      ),

    group_block: ($) => seq("{", repeat($._group_item), "}"),

    enterprise: ($) =>
      seq(
        "enterprise",
        field("name", $._value),
        optional(field("body", $.enterprise_block)),
      ),

    enterprise_block: ($) => seq("{", repeat($._enterprise_item), "}"),

    deployment_environment: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "deploymentEnvironment",
        field("name", $._value),
        optional(field("body", $.deployment_environment_block)),
      ),

    deployment_environment_block: ($) =>
      seq("{", repeat($._deployment_item), "}"),

    deployment_group: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "deploymentGroup",
        field("name", $._value),
      ),

    deployment_node: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "deploymentNode",
        field("name", $._value),
        repeat(field("attribute", $._deployment_node_attribute)),
        optional(field("body", $.deployment_node_block)),
      ),

    infrastructure_node: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "infrastructureNode",
        field("name", $._value),
        repeat(field("attribute", $._deployment_node_attribute)),
        optional(field("body", $.deployment_node_block)),
      ),

    _deployment_node_attribute: ($) => choice($._metadata_value, $.number),

    deployment_instance_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.relationship,
            $.tag_statement,
            $.tags_statement,
            $.url_statement,
            $.properties_block,
            $.perspectives_block,
            $.health_check_statement,
          ),
        ),
        "}",
      ),

    deployment_node_block: ($) =>
      seq("{", repeat($._deployment_node_item), "}"),

    container_instance: ($) => deploymentInstanceRule($, "containerInstance"),

    software_system_instance: ($) =>
      deploymentInstanceRule($, "softwareSystemInstance"),

    instance_of: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "instanceOf",
        field("target", $.identifier),
        optional(field("body", $.deployment_instance_block)),
      ),

    // Relationships appear both as top-level model statements and nested inside
    // element bodies. The grammar keeps them permissive enough to cover plain `->`
    // as well as archetyped operators like `--https->`.
    relationship: ($) =>
      prec.right(
        choice(
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("source", $._relationship_endpoint),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("source", $._relationship_endpoint),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("source", $._relationship_endpoint),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            field("attribute", $._metadata_value),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("source", $._relationship_endpoint),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            field("attribute", $._metadata_value),
            field("attribute", $._relationship_tag_value),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            field("attribute", $._metadata_value),
            optional(field("body", $.relationship_block)),
          ),
          seq(
            optional(seq(field("identifier", $._assignment_identifier), "=")),
            field("operator", $.relationship_operator),
            field("destination", $._relationship_endpoint),
            field("attribute", $._metadata_value),
            field("attribute", $._metadata_value),
            field("attribute", $._relationship_tag_value),
            optional(field("body", $.relationship_block)),
          ),
        ),
      ),

    _relationship_endpoint: ($) => choice($.identifier, $.this_keyword),

    this_keyword: (_) => "this",

    relationship_operator: ($) =>
      choice(
        "->",
        "-/>",
        seq("--", field("archetype", $.relationship_archetype_name), "->"),
      ),

    relationship_archetype_name: (_) => /[A-Za-z_][A-Za-z0-9_.]*/,

    relationship_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.tag_statement,
            $.tags_statement,
            $.description_statement,
            $.technology_statement,
            $.url_statement,
            $.properties_block,
            $.perspectives_block,
            $.nested_relationship,
          ),
        ),
        "}",
      ),

    nested_relationship: ($) =>
      choice(
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
    archetypes: ($) => seq("archetypes", field("body", $.archetypes_block)),

    archetypes_block: ($) => seq("{", repeat($.archetype_definition), "}"),

    archetype_definition: ($) =>
      seq(
        optional(seq(field("identifier", $.identifier), "=")),
        field("base", $.archetype_base),
        optional(field("body", $.archetype_body)),
      ),

    archetype_base: ($) =>
      choice(
        "person",
        choice("softwareSystem", "softwaresystem"),
        "container",
        "component",
        "element",
        "group",
        $.relationship_operator,
        $.identifier,
      ),

    archetype_body: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.description_statement,
            $.technology_statement,
            $.tag_statement,
            $.tags_statement,
            $.metadata_statement,
            $.properties_block,
            $.perspectives_block,
          ),
        ),
        "}",
      ),

    custom_element: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "element",
        field("name", $._value),
        repeat(field("attribute", $._metadata_value)),
        optional(field("body", $.custom_element_block)),
      ),

    custom_element_block: ($) => seq("{", repeat($._custom_block_item), "}"),

    archetype_instance: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        field("kind", $.identifier),
        field("name", $._value),
        repeat(field("metadata", $._metadata_value)),
        optional(field("body", $.custom_element_block)),
      ),

    element_directive: ($) =>
      seq(
        optional(seq(field("identifier", $._assignment_identifier), "=")),
        "!element",
        field("target", $._directive_value),
        field("body", $.element_directive_block),
      ),

    element_directive_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $._custom_block_item,
            $.deployment_node,
            $.infrastructure_node,
            $.container_instance,
            $.software_system_instance,
          ),
        ),
        "}",
      ),

    elements_directive: ($) =>
      seq(
        "!elements",
        field("expression", $._directive_value),
        field("body", $.elements_block),
      ),

    relationships_directive: ($) =>
      seq(
        "!relationships",
        field("expression", $._directive_value),
        field("body", $.relationships_block),
      ),

    elements_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.relationship,
            $.tag_statement,
            $.tags_statement,
            $.description_statement,
            $.technology_statement,
            $.url_statement,
            $.properties_block,
            $.perspectives_block,
          ),
        ),
        "}",
      ),

    relationships_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.tag_statement,
            $.tags_statement,
            $.description_statement,
            $.technology_statement,
            $.url_statement,
            $.properties_block,
            $.perspectives_block,
          ),
        ),
        "}",
      ),

    // The views section contains several related families of view definitions that
    // mostly differ by their header fields while sharing a common set of statements.
    _view_item: ($) =>
      choice(
        $.include_directive,
        $.system_landscape_view,
        $.system_context_view,
        $.container_view,
        $.component_view,
        $.filtered_view,
        $.dynamic_view,
        $.deployment_view,
        $.custom_view,
        $.image_view,
        $.branding,
        $.terminology,
        $.properties_block,
        $.const_directive,
        $.constant_directive,
        $.var_directive,
        $.styles,
        $.theme_statement,
        $.themes_statement,
      ),

    _view_value: ($) =>
      choice($.identifier, $.string, $.wildcard, $.bare_value),

    wildcard: (_) => choice("*", "*?"),

    layout_direction: (_) => choice("tb", "bt", "lr", "rl"),

    _static_view_statement: ($) =>
      choice(
        $.include_statement,
        $.exclude_statement,
        $.animation_statement,
        $.auto_layout_statement,
        $.properties_block,
        $.default_statement,
        $.title_statement,
        $.description_statement,
      ),

    _filtered_view_statement: ($) =>
      choice(
        $.properties_block,
        $.default_statement,
        $.title_statement,
        $.description_statement,
      ),

    _dynamic_view_statement: ($) =>
      choice(
        $.dynamic_relationship,
        $.dynamic_relationship_reference,
        $.dynamic_parallel_block,
        $.auto_layout_statement,
        $.properties_block,
        $.default_statement,
        $.title_statement,
        $.description_statement,
      ),

    _advanced_view_statement: ($) =>
      choice(
        $.include_statement,
        $.exclude_statement,
        $.animation_statement,
        $.auto_layout_statement,
        $.properties_block,
        $.default_statement,
        $.title_statement,
        $.description_statement,
      ),

    include_statement: ($) =>
      seq("include", repeat1(field("value", $._view_value))),

    exclude_statement: ($) =>
      seq("exclude", repeat1(field("value", $._view_value))),

    auto_layout_statement: ($) =>
      choice(
        choice("autoLayout", "autolayout"),
        seq(
          choice("autoLayout", "autolayout"),
          field("direction", $.layout_direction),
        ),
        seq(
          choice("autoLayout", "autolayout"),
          field("direction", $.layout_direction),
          field("rank_separation", $.number),
        ),
        seq(
          choice("autoLayout", "autolayout"),
          field("direction", $.layout_direction),
          field("rank_separation", $.number),
          field("node_separation", $.number),
        ),
      ),

    default_statement: (_) => "default",

    animation_statement: ($) =>
      seq("animation", field("body", $.animation_block)),

    animation_block: ($) =>
      seq("{", repeat(field("value", $._animation_value)), "}"),

    _animation_value: ($) =>
      choice($.identifier, $.bare_value, $.wildcard, $.string),

    // These directives are intentionally modeled as lightweight syntactic forms.
    // They matter for editor tooling and fixture coverage even when downstream
    // consumers do not execute or resolve them.
    include_directive: ($) =>
      seq("!include", field("value", $._directive_value)),

    const_directive: ($) =>
      seq(
        "!const",
        field("name", $._directive_value),
        field("value", $._directive_value),
      ),

    constant_directive: ($) =>
      seq(
        "!constant",
        field("name", $._directive_value),
        field("value", $._directive_value),
      ),

    var_directive: ($) =>
      seq(
        "!var",
        field("name", $._directive_value),
        field("value", $._directive_value),
      ),

    identifiers_directive: ($) =>
      seq("!identifiers", field("value", $._directive_value)),

    implied_relationships_directive: ($) =>
      seq("!impliedRelationships", field("value", $._directive_value)),

    docs_directive: ($) =>
      choice(
        prec.dynamic(
          1,
          seq(
            "!docs",
            field("path", $._directive_value),
            field("importer", $.java_fully_qualified_name),
          ),
        ),
        seq("!docs", field("path", $._directive_value)),
      ),

    adrs_directive: ($) =>
      choice(
        prec.dynamic(
          1,
          seq(
            "!adrs",
            field("path", $._directive_value),
            field(
              "importer",
              choice($.decision_importer_type, $.java_fully_qualified_name),
            ),
          ),
        ),
        seq("!adrs", field("path", $._directive_value)),
      ),

    system_landscape_view: ($) =>
      choice(
        seq(
          choice("systemLandscape", "systemlandscape"),
          field("body", $.system_landscape_view_block),
        ),
        seq(
          choice("systemLandscape", "systemlandscape"),
          field("key", $._value),
          field("body", $.system_landscape_view_block),
        ),
        seq(
          choice("systemLandscape", "systemlandscape"),
          field("key", $._value),
          field("description", $._metadata_value),
          field("body", $.system_landscape_view_block),
        ),
      ),

    system_landscape_view_block: ($) =>
      seq("{", repeat($._static_view_statement), "}"),

    system_context_view: ($) =>
      choice(
        seq(
          choice("systemContext", "systemcontext"),
          field("scope", $.identifier),
          field("body", $.system_context_view_block),
        ),
        seq(
          choice("systemContext", "systemcontext"),
          field("scope", $.identifier),
          field("key", $._value),
          field("body", $.system_context_view_block),
        ),
        seq(
          choice("systemContext", "systemcontext"),
          field("scope", $.identifier),
          field("key", $._value),
          field("description", $._metadata_value),
          field("body", $.system_context_view_block),
        ),
      ),

    system_context_view_block: ($) =>
      seq("{", repeat($._static_view_statement), "}"),

    container_view: ($) =>
      choice(
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

    container_view_block: ($) =>
      seq("{", repeat($._static_view_statement), "}"),

    component_view: ($) =>
      choice(
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

    component_view_block: ($) =>
      seq("{", repeat($._static_view_statement), "}"),

    filtered_view: ($) =>
      choice(
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
        ),
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
          field("key", $._value),
        ),
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
          field("key", $._value),
          field("description", $._metadata_value),
        ),
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
          field("body", $.filtered_view_block),
        ),
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
          field("key", $._value),
          field("body", $.filtered_view_block),
        ),
        seq(
          "filtered",
          field("base_key", $._value),
          field("mode", $.filter_mode),
          field("tags", $._tag_value),
          field("key", $._value),
          field("description", $._metadata_value),
          field("body", $.filtered_view_block),
        ),
      ),

    filtered_view_block: ($) =>
      seq("{", repeat($._filtered_view_statement), "}"),

    dynamic_view: ($) =>
      choice(
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

    dynamic_view_block: ($) => seq("{", repeat($._dynamic_view_statement), "}"),

    order: (_) => token(/[0-9]+(\.[0-9]+)*/),

    dynamic_relationship: ($) =>
      choice(
        prec.right(
          seq(
            optional(seq(field("order", $.order), ":")),
            field("source", $.identifier),
            "->",
            field("destination", $.identifier),
            optional(field("body", $.dynamic_relationship_block)),
          ),
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

    dynamic_relationship_reference: ($) =>
      seq(
        optional(seq(field("order", $.order), ":")),
        field("relationship", $.identifier),
        field("description", $._metadata_value),
      ),

    dynamic_parallel_block: ($) =>
      seq(
        "{",
        repeat1(
          choice(
            $.dynamic_relationship,
            $.dynamic_relationship_reference,
            $.dynamic_parallel_block,
          ),
        ),
        "}",
      ),

    dynamic_relationship_block: ($) =>
      seq(
        "{",
        repeat1(
          choice(
            $.url_statement,
            $.properties_block,
            $.dynamic_relationship,
            $.dynamic_relationship_reference,
            $.dynamic_parallel_block,
          ),
        ),
        "}",
      ),

    // `custom` and `image` views sit outside the core C4 view set but are important
    // to parse because they show up in real workspaces and affect editor behavior.
    deployment_view: ($) =>
      choice(
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

    deployment_view_block: ($) =>
      seq("{", repeat($._advanced_view_statement), "}"),

    custom_view: ($) =>
      choice(
        seq("custom", field("body", $.custom_view_block)),
        seq(
          "custom",
          field("key", $._value),
          field("body", $.custom_view_block),
        ),
        seq(
          "custom",
          field("key", $._value),
          field("title", $._value),
          field("body", $.custom_view_block),
        ),
        seq(
          "custom",
          field("key", $._value),
          field("title", $._value),
          field("description", $._metadata_value),
          field("body", $.custom_view_block),
        ),
      ),

    custom_view_block: ($) => seq("{", repeat($._advanced_view_statement), "}"),

    image_view: ($) =>
      choice(
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

    image_view_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.plantuml_source,
            $.mermaid_source,
            $.kroki_source,
            $.image_source,
            $.light_image_sources,
            $.dark_image_sources,
            // Upstream allows renderer properties inside one `image` view, not
            // only at the surrounding `views` level. Keep that form in the
            // grammar so semantic validation can inspect those local overrides.
            $.properties_block,
            $.default_statement,
            $.title_statement,
            $.description_statement,
          ),
        ),
        "}",
      ),

    light_image_sources: ($) =>
      seq("light", field("body", $.image_source_block)),

    dark_image_sources: ($) => seq("dark", field("body", $.image_source_block)),

    image_source_block: ($) =>
      seq(
        "{",
        repeat1(
          choice(
            $.plantuml_source,
            $.mermaid_source,
            $.kroki_source,
            $.image_source,
          ),
        ),
        "}",
      ),

    branding: ($) => seq("branding", field("body", $.branding_block)),

    branding_block: ($) =>
      seq("{", repeat(choice($.logo_statement, $.font_statement)), "}"),

    logo_statement: ($) => seq("logo", field("value", $._directive_value)),

    font_statement: ($) =>
      seq(
        "font",
        field("name", $._directive_value),
        field("url", $._directive_value),
      ),

    theme_statement: ($) => seq("theme", field("value", $._directive_value)),

    themes_statement: ($) =>
      prec.right(seq("themes", repeat1(field("value", $._directive_value)))),

    // Style rules are effectively key/value bags scoped by tag, so the grammar keeps
    // them deliberately generic instead of trying to encode every allowed property.
    styles: ($) => seq("styles", field("body", $.styles_block)),

    styles_block: ($) => seq("{", repeat($._style_item), "}"),

    _style_item: ($) =>
      choice(
        $.include_directive,
        $.element_style,
        $.relationship_style,
        $.light_styles,
        $.dark_styles,
        $.theme_statement,
        $.themes_statement,
      ),

    light_styles: ($) => seq("light", field("body", $.style_mode_block)),

    dark_styles: ($) => seq("dark", field("body", $.style_mode_block)),

    style_mode_block: ($) =>
      seq(
        "{",
        repeat(
          choice($.include_directive, $.element_style, $.relationship_style),
        ),
        "}",
      ),

    element_style: ($) =>
      seq("element", field("tag", $._value), field("body", $.style_rule_block)),

    relationship_style: ($) =>
      seq(
        "relationship",
        field("tag", $._value),
        field("body", $.style_rule_block),
      ),

    style_rule_block: ($) =>
      seq("{", repeat(choice($.style_setting, $.properties_block)), "}"),

    color_value: ($) => choice($.hex_color, $.named_color),

    style_setting: ($) =>
      choice(
        prec(
          1,
          seq(
            field(
              "name",
              alias(choice(...COLOR_STYLE_PROPERTIES), $.identifier),
            ),
            field("value", $.color_value),
          ),
        ),
        seq(
          field("name", alias("shape", $.identifier)),
          field("value", $.shape_value),
        ),
        seq(
          field("name", alias("border", $.identifier)),
          field("value", $.border_value),
        ),
        seq(
          field("name", alias("iconPosition", $.identifier)),
          field("value", $.icon_position_value),
        ),
        seq(
          field("name", alias("style", $.identifier)),
          field("value", $.line_style_value),
        ),
        seq(
          field("name", alias("routing", $.identifier)),
          field("value", $.routing_value),
        ),
        seq(
          field(
            "name",
            alias(
              choice("metadata", "description", "dashed", "jump"),
              $.identifier,
            ),
          ),
          field("value", $.boolean_value),
        ),
        seq(field("name", $.identifier), field("value", $._style_value)),
      ),

    _style_value: ($) =>
      choice($.string, $.number, $.boolean_value, $.identifier, $.bare_value),

    properties_block: ($) =>
      seq("properties", "{", repeat($.property_entry), "}"),

    property_entry: ($) =>
      seq(
        field("name", $._directive_value),
        field("value", $._directive_value),
      ),

    perspectives_block: ($) =>
      seq(
        "perspectives",
        "{",
        repeat(choice($.perspective_entry, $.perspective_definition)),
        "}",
      ),

    perspective_entry: ($) =>
      choice(
        seq(
          field("name", $._directive_value),
          field("description", $._directive_value),
        ),
        prec(
          1,
          seq(
            field("name", $._directive_value),
            field("description", $._directive_value),
            field("value", $._directive_value),
          ),
        ),
      ),

    perspective_definition: ($) =>
      seq(
        "perspective",
        field("name", $._directive_value),
        field("body", $.perspective_block),
      ),

    perspective_block: ($) =>
      seq(
        "{",
        repeat1(choice($.value_statement, $.description_statement)),
        "}",
      ),

    health_check_statement: ($) =>
      choice(
        seq(
          "healthCheck",
          field("name", $._directive_value),
          field("url", $._directive_value),
        ),
        seq(
          "healthCheck",
          field("name", $._directive_value),
          field("url", $._directive_value),
          field("interval", $.number),
        ),
        seq(
          "healthCheck",
          field("name", $._directive_value),
          field("url", $._directive_value),
          field("interval", $.number),
          field("timeout", $.number),
        ),
      ),

    // Image views can point at several source syntaxes; these are modeled as small,
    // explicit statements so query authors can target them later.
    plantuml_source: ($) => seq("plantuml", field("value", $._directive_value)),

    mermaid_source: ($) => seq("mermaid", field("value", $._directive_value)),

    kroki_source: ($) =>
      seq(
        "kroki",
        field("format", $._directive_value),
        field("value", $._directive_value),
      ),

    image_source: ($) => seq("image", field("value", $._directive_value)),

    terminology: ($) => seq("terminology", field("body", $.terminology_block)),

    terminology_block: ($) => seq("{", repeat($.terminology_entry), "}"),

    terminology_entry: ($) =>
      seq(
        field(
          "kind",
          alias(
            choice(
              "enterprise",
              "person",
              "softwareSystem",
              "container",
              "component",
              "deploymentNode",
              "infrastructureNode",
              "relationship",
              "metadata",
            ),
            $.identifier,
          ),
        ),
        field("value", $._directive_value),
      ),

    // Configuration is comparatively small in the DSL, but it affects visibility and
    // audience semantics that downstream tools may want to surface.
    configuration: ($) =>
      seq("configuration", field("body", $.configuration_block)),

    configuration_block: ($) =>
      seq(
        "{",
        repeat(
          choice($.scope_statement, $.visibility_statement, $.users_block),
        ),
        "}",
      ),

    scope_statement: ($) => seq("scope", field("value", $._directive_value)),

    visibility_statement: ($) =>
      seq("visibility", field("value", $._directive_value)),

    users_block: ($) => seq("users", "{", repeat($.user_entry), "}"),

    user_entry: ($) =>
      seq(
        field("username", $._directive_value),
        field("role", $._directive_value),
      ),
  },
});
