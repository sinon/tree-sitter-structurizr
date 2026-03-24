  # Issue 15: deployment-groups-flat.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/deployment-groups-flat.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/deployment-groups-flat.dsl

  Summary: The upstream Structurizr DSL sample `deployment-groups-flat.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [35, 1]
  (ERROR [0, 10] - [10, 29]
        (ERROR [3, 8] - [3, 24])
              (ERROR [5, 16] - [5, 35]
    (ERROR [10, 18] - [10, 29]))
  (ERROR [10, 56] - [11, 26]
    (ERROR [11, 22] - [11, 26]))
    (ERROR [12, 16] - [13, 42]
      (ERROR [12, 25] - [12, 37])
      (ERROR [13, 25] - [13, 42]))))
(ERROR [15, 22] - [15, 26])
(ERROR [16, 25] - [16, 37])
  ```

  Suggested next grammar area: deployment model and deployment view grammar
