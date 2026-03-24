  # Issue 4: archetypes-for-defaults.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes-for-defaults.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes-for-defaults.dsl

  Summary: The upstream Structurizr DSL sample `archetypes-for-defaults.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 10] - [5, 27]
    (ERROR [2, 10] - [4, 26]
  (ERROR [6, 20] - [8, 26]
    (ERROR [9, 20] - [9, 68]
(ERROR [12, 16] - [37, 1]
  (ERROR [12, 16] - [12, 28])
  (ERROR [20, 16] - [20, 19])
  (ERROR [26, 16] - [26, 28])
  (ERROR [32, 8] - [32, 9])
  (ERROR [33, 8] - [33, 9])
  (ERROR [34, 8] - [34, 9])
  (ERROR [34, 13] - [34, 14])))
  ```

  Suggested next grammar area: archetypes and custom element/relationship extensions
