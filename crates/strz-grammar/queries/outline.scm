(workspace
  "workspace" @context
  name: (_) @name) @item

(workspace
  "workspace" @name) @item

(model
  "model" @name) @item

(views
  "views" @name) @item

(branding
  "branding" @name) @item

(terminology
  "terminology" @name) @item

(styles
  "styles" @name) @item

(configuration
  "configuration" @name) @item

(archetypes
  "archetypes" @name) @item

(person
  "person" @context
  name: (_) @name) @item

(group
  "group" @context
  name: (_) @name) @item

(enterprise
  "enterprise" @context
  name: (_) @name) @item

(archetype_definition
  identifier: (identifier) @name) @item

(archetype_instance
  kind: (identifier) @context
  name: (_) @name) @item

(software_system
  [
    "softwareSystem"
    "softwaresystem"
  ] @context
  name: (_) @name) @item

(container
  "container" @context
  name: (_) @name) @item

(component
  "component" @context
  name: (_) @name) @item

(custom_element
  "element" @context
  name: (_) @name) @item

(deployment_environment
  "deploymentEnvironment" @context
  name: (_) @name) @item

(deployment_group
  "deploymentGroup" @context
  name: (_) @name) @item

(deployment_node
  "deploymentNode" @context
  name: (_) @name) @item

(infrastructure_node
  "infrastructureNode" @context
  name: (_) @name) @item

(container_instance
  "containerInstance" @context
  target: (identifier) @name) @item

(software_system_instance
  "softwareSystemInstance" @context
  target: (identifier) @name) @item

(instance_of
  "instanceOf" @context
  target: (identifier) @name) @item

(relationship
  identifier: (identifier) @name) @item

(docs_directive
  "!docs" @context
  path: (_) @name) @item

(adrs_directive
  "!adrs" @context
  path: (_) @name) @item

(include_directive
  "!include" @context
  value: (_) @name) @item

(const_directive
  "!const" @context
  name: (_) @name) @item

(constant_directive
  "!constant" @context
  name: (_) @name) @item

(var_directive
  "!var" @context
  name: (_) @name) @item

(elements_directive
  "!elements" @context
  expression: (_) @name) @item

(relationships_directive
  "!relationships" @context
  expression: (_) @name) @item

(element_directive
  "!element" @context
  target: (_) @name) @item

(system_landscape_view
  "systemLandscape" @name) @item

(system_context_view
  "systemContext" @context
  scope: (identifier) @name) @item

(container_view
  "container" @context
  scope: (identifier) @name) @item

(component_view
  "component" @context
  scope: (identifier) @name) @item

(filtered_view
  "filtered" @context
  base_key: (_) @name) @item

(dynamic_view
  "dynamic" @context
  scope: (_) @name) @item

(deployment_view
  "deployment" @context
  environment: (_) @name) @item

(custom_view
  "custom" @name) @item

(image_view
  "image" @context
  scope: (_) @name) @item

(element_style
  "element" @context
  tag: (_) @name) @item

(relationship_style
  "relationship" @context
  tag: (_) @name) @item

(light_styles
  "light" @name) @item

(dark_styles
  "dark" @name) @item

(perspective_definition
  "perspective" @context
  name: (_) @name) @item
