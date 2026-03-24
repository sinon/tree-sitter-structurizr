  # Issue 25: find-element.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/find-element.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/find-element.dsl

  Summary: The upstream Structurizr DSL sample `find-element.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [44, 1]
      (ERROR [2, 10] - [6, 60]
        (ERROR [4, 8] - [4, 9])
        (ERROR [7, 20] - [7, 30]
          (ERROR [7, 23] - [7, 28])
(ERROR [12, 8] - [12, 9])
(ERROR [13, 22] - [13, 26])
(ERROR [14, 16] - [14, 34])
(ERROR [15, 23] - [15, 28])
(ERROR [20, 8] - [20, 9])
(ERROR [20, 17] - [20, 23])
(ERROR [21, 22] - [21, 26])
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
