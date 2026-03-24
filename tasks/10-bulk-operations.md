  # Issue 10: bulk-operations.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/bulk-operations.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/bulk-operations.dsl

  Summary: The upstream Structurizr DSL sample `bulk-operations.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [70, 1]
  (ERROR [0, 10] - [8, 36]
    (ERROR [8, 12] - [8, 23])
    (ERROR [9, 16] - [11, 38]
(ERROR [14, 12] - [14, 26])
(ERROR [16, 12] - [16, 13])
(ERROR [16, 20] - [16, 21])
(ERROR [18, 16] - [18, 20])
(ERROR [18, 24] - [18, 28])
(ERROR [19, 16] - [19, 20])
(ERROR [19, 24] - [19, 38])
(ERROR [24, 12] - [24, 13])
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
