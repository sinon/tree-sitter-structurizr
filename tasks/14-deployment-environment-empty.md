    # Issue 14: deployment-environment-empty.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/deployment-environment-empty.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/deployment-environment-empty.dsl

    Summary: The upstream Structurizr DSL sample `deployment-environment-empty.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [8, 1]
    (ERROR [0, 10] - [2, 34]
      (ERROR [2, 23] - [2, 34]))
    (ERROR [4, 8] - [5, 31]
      (ERROR [4, 8] - [4, 9])
      (ERROR [4, 17] - [4, 19])
      (ERROR [5, 12] - [5, 14])
      (ERROR [5, 27] - [5, 31]))
/tmp/structurizr-upstream-dsl-audit/deployment-environment-empty.dsl	Parse:    0.10 ms	  1696 bytes/ms	(ERROR [0, 0] - [8, 1])
    ```

    Suggested next grammar area: deployment model and deployment view grammar
