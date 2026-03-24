  # Issue 34: hierarchical-identifiers-and-deployment-nodes-1.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-1.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-1.dsl

  Summary: The upstream Structurizr DSL sample `hierarchical-identifiers-and-deployment-nodes-1.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 1] - [39, 1]
      (ERROR [4, 10] - [10, 42]
        (ERROR [9, 27] - [9, 31])
        (ERROR [10, 32] - [10, 36])
        (ERROR [11, 20] - [11, 45]
          (ERROR [11, 43] - [11, 45]))))
    (ERROR [14, 16] - [15, 45]
      (ERROR [14, 16] - [14, 18])
      (ERROR [14, 32] - [14, 36])
      (ERROR [15, 34] - [15, 45]))))
(ERROR [18, 16] - [18, 18])
(ERROR [18, 23] - [18, 25])
  ```

  Suggested next grammar area: deployment model and deployment view grammar
