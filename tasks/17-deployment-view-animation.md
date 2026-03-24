  # Issue 17: deployment-view-animation.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/deployment-view-animation.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/deployment-view-animation.dsl

  Summary: The upstream Structurizr DSL sample `deployment-view-animation.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [74, 1]
      (ERROR [2, 10] - [4, 48]
        (ERROR [3, 13] - [3, 27])
        (ERROR [5, 16] - [5, 24]
    (ERROR [7, 12] - [8, 24]
      (ERROR [7, 12] - [7, 14])
      (ERROR [8, 16] - [8, 19])
(ERROR [12, 8] - [12, 14])
(ERROR [12, 18] - [14, 12])
(ERROR [14, 25] - [14, 36])
(ERROR [15, 12] - [15, 14])
(ERROR [15, 27] - [15, 31])
  ```

  Suggested next grammar area: deployment model and deployment view grammar
