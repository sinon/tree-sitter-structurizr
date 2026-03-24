    # Issue 36: hierarchical-identifiers-and-deployment-nodes-3.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-3.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-and-deployment-nodes-3.dsl

    Summary: The upstream Structurizr DSL sample `hierarchical-identifiers-and-deployment-nodes-3.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [9, 8] - [13, 46]
            (ERROR [9, 13] - [12, 21]
              (ERROR [9, 25] - [9, 36])
              (ERROR [10, 12] - [10, 14])
              (ERROR [10, 27] - [10, 31])
              (ERROR [11, 16] - [11, 18])
              (ERROR [11, 31] - [11, 35])
              (ERROR [12, 20] - [12, 21]))
            (ERROR [12, 34] - [12, 38])
            (ERROR [13, 33] - [13, 46]))))))
  (ERROR [16, 12] - [21, 1]))
/tmp/structurizr-upstream-dsl-audit/hierarchical-identifiers-and-deployment-nodes-3.dsl	Parse:    0.17 ms	  2499 bytes/ms	(ERROR [9, 8] - [13, 46])
    ```

    Suggested next grammar area: deployment model and deployment view grammar
