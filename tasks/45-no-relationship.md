  # Issue 45: no-relationship.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/no-relationship.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/no-relationship.dsl

  Summary: The upstream Structurizr DSL sample `no-relationship.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [87, 1]
  (ERROR [0, 10] - [15, 30]
    (ERROR [13, 24] - [13, 35])
    (ERROR [14, 22] - [14, 26])
    (ERROR [15, 26] - [15, 30]))
    (ERROR [16, 20] - [16, 36]
      (ERROR [16, 20] - [16, 36]))))
(ERROR [18, 16] - [18, 37])
(ERROR [24, 8] - [24, 11])
(ERROR [24, 24] - [24, 35])
(ERROR [25, 22] - [25, 26])
(ERROR [26, 26] - [26, 30])
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
