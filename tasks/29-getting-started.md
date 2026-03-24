    # Issue 29: getting-started.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/getting-started.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/getting-started.dsl

    Summary: The upstream Structurizr DSL sample `getting-started.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 8] - [4, 24])
          (ERROR [15, 8] - [15, 21]
            (ERROR [15, 8] - [15, 13])
/tmp/structurizr-upstream-dsl-audit/getting-started.dsl	Parse:    0.10 ms	  2824 bytes/ms	(ERROR [4, 8] - [4, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
