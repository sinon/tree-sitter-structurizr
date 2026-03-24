    # Issue 35: hierarchical-identifiers-and-deployment-nodes-2.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-2.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-2.dsl

    Summary: The upstream Structurizr DSL sample `hierarchical-identifiers-and-deployment-nodes-2.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [7, 13] - [9, 18]
              (ERROR [7, 25] - [7, 36])
              (ERROR [8, 12] - [8, 14])
              (ERROR [8, 27] - [8, 31])
              (ERROR [9, 16] - [9, 18]))
            (ERROR [9, 21] - [9, 43]
              (ERROR [9, 31] - [9, 35])
          (ERROR [10, 43] - [11, 17]
  (ERROR [15, 4] - [17, 1]))
/tmp/structurizr-upstream-dsl-audit/hierarchical-identifiers-and-deployment-nodes-2.dsl	Parse:    0.15 ms	  2102 bytes/ms	(ERROR [7, 13] - [9, 18])
    ```

    Suggested next grammar area: deployment model and deployment view grammar
