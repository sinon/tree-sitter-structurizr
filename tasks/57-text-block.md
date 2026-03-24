  # Issue 57: text-block.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/text-block.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/text-block.dsl

  Summary: The upstream Structurizr DSL sample `text-block.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 10] - [8, 25]
        (ERROR [3, 8] - [4, 58]
    (ERROR [7, 8] - [7, 21])
    (ERROR [7, 24] - [8, 25]))
  (ERROR [9, 14] - [11, 19]
    (ERROR [9, 14] - [11, 19]))
  (ERROR [11, 22] - [13, 16]
    (ERROR [11, 22] - [13, 16]))
    (ERROR [14, 16] - [14, 41]
      (ERROR [14, 16] - [14, 31])
      (ERROR [14, 33] - [14, 41]))))
(ERROR [16, 12] - [31, 1]
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
