(comment) @comment

(string) @string

(hex_color) @string.special

(named_color) @string.special

(number) @number

(order) @number

(wildcard) @constant.builtin

(layout_direction) @constant.builtin

(this_keyword) @variable.special

(relationship_operator) @operator

(default_statement) @keyword

[
  "!include"
  "!identifiers"
  "!impliedRelationships"
  "!docs"
  "!adrs"
  "!elements"
  "!element"
] @preproc

[
  "workspace"
  "extends"
  "model"
  "views"
  "configuration"
  "archetypes"
  "person"
  "softwareSystem"
  "softwaresystem"
  "container"
  "component"
  "deploymentEnvironment"
  "deploymentGroup"
  "deploymentNode"
  "infrastructureNode"
  "containerInstance"
  "softwareSystemInstance"
  "description"
  "technology"
  "tag"
  "tags"
  "metadata"
  "url"
  "name"
  "title"
  "include"
  "exclude"
  "autoLayout"
  "autolayout"
  "animation"
  "systemLandscape"
  "systemContext"
  "filtered"
  "dynamic"
  "deployment"
  "custom"
  "styles"
  "light"
  "dark"
  "element"
  "relationship"
  "properties"
  "theme"
  "themes"
  "plantuml"
  "mermaid"
  "kroki"
  "image"
  "scope"
  "visibility"
  "users"
] @keyword

; Unquoted `url https://...` values parse as `bare_value`, so capture the value
; explicitly to keep URL statements visually coherent in editors.
(url_statement
  value: [
    (bare_value)
    (string)
  ] @string.special.url)

[
  "{"
  "}"
] @punctuation.bracket

":" @punctuation.delimiter

"=" @operator

"->" @operator

(person
  identifier: (identifier) @variable)

(software_system
  identifier: (identifier) @type)

(container
  identifier: (identifier) @type)

(component
  identifier: (identifier) @type)

(custom_element
  identifier: (identifier) @type)

(archetype_definition
  identifier: (identifier) @type)

(archetype_instance
  kind: (identifier) @type)

(deployment_environment
  identifier: (identifier) @type)

(deployment_group
  identifier: (identifier) @type)

(deployment_node
  identifier: (identifier) @type)

(infrastructure_node
  identifier: (identifier) @type)

(relationship
  source: (identifier) @variable
  destination: (identifier) @variable)

(dynamic_relationship
  source: (identifier) @variable
  destination: (identifier) @variable)

(include_statement
  value: (identifier) @variable)

(animation_block
  value: (identifier) @variable)

(style_setting
  name: (identifier) @property)

(property_entry
  name: [
    (identifier)
    (bare_value)
    (string)
  ] @property)
