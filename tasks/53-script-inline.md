  # Issue 53: script-inline.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/script-inline.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/script-inline.dsl

  Summary: The upstream Structurizr DSL sample `script-inline.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [2, 4] - [3, 44]
      (ERROR [2, 4] - [2, 18])
      (ERROR [3, 17] - [3, 18])
      (ERROR [3, 23] - [3, 34])
      (ERROR [3, 42] - [3, 44]))))
(ERROR [6, 4] - [7, 18]
  (ERROR [6, 4] - [6, 18])
  (ERROR [7, 17] - [7, 18]))
  (ERROR [7, 23] - [10, 16]
    (ERROR [7, 23] - [7, 34])
    (ERROR [7, 42] - [7, 44])
    (ERROR [10, 4] - [10, 16]))
  ```

  Suggested next grammar area: script/plugin directive blocks
