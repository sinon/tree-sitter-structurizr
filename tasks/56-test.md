  # Issue 56: test.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/test.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/test.dsl

  Summary: The upstream Structurizr DSL sample `test.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [455, 1]
(ERROR [0, 0] - [4, 13]
  (ERROR [0, 0] - [0, 24])
  (ERROR [1, 0] - [1, 17])
  (ERROR [3, 0] - [3, 4])
  (ERROR [3, 10] - [4, 4])
  (ERROR [4, 10] - [4, 13]))
    (ERROR [32, 8] - [33, 26]
(ERROR [36, 8] - [36, 18])
(ERROR [37, 12] - [37, 23])
(ERROR [38, 16] - [38, 19])
(ERROR [39, 16] - [39, 28])
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
