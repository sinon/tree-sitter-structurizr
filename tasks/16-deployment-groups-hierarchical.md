  # Issue 16: deployment-groups-hierarchical.dsl

  Source fixture: `structurizr-dsl/src/test/resources/dsl/deployment-groups-hierarchical.dsl`

  Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/deployment-groups-hierarchical.dsl

  Summary: The upstream Structurizr DSL sample `deployment-groups-hierarchical.dsl` does not yet parse cleanly with the local tree-sitter grammar.

  Symptoms: Contains `ERROR` nodes

  Parse excerpt:

  ```text
  (ERROR [0, 0] - [50, 1]
  (ERROR [0, 10] - [13, 26]
        (ERROR [5, 8] - [5, 24])
              (ERROR [7, 16] - [7, 35]
    (ERROR [12, 18] - [12, 29])
    (ERROR [13, 22] - [13, 26]))
    (ERROR [14, 16] - [15, 57]
      (ERROR [14, 25] - [14, 33])
      (ERROR [14, 48] - [14, 52])
      (ERROR [15, 25] - [15, 33])
      (ERROR [15, 48] - [15, 57]))))
(ERROR [17, 22] - [17, 26])
  ```

  Suggested next grammar area: deployment model and deployment view grammar
