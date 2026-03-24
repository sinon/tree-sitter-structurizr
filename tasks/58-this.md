  # Issue 58: this.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/this.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/this.dsl

  Summary: The upstream Structurizr DSL sample `this.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [70, 1]
  (ERROR [0, 10] - [3, 24]
  (ERROR [5, 8] - [11, 25]
    (ERROR [6, 22] - [8, 13])
    (ERROR [9, 26] - [9, 30]))
    (ERROR [12, 20] - [12, 34]
      (ERROR [12, 30] - [12, 34]))))
(ERROR [17, 8] - [17, 12])
(ERROR [17, 25] - [17, 36])
(ERROR [18, 22] - [18, 26])
(ERROR [19, 16] - [19, 18])
(ERROR [19, 21] - [19, 39])
  ```

  Suggested next grammar area: remaining advanced DSL constructs in this sample
