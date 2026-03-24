    # Issue 28: getting-started-short.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/getting-started-short.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/getting-started-short.dsl

    Summary: The upstream Structurizr DSL sample `getting-started-short.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 8] - [4, 24])
/tmp/structurizr-upstream-dsl-audit/getting-started-short.dsl	Parse:    0.11 ms	  3078 bytes/ms	(ERROR [4, 8] - [4, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
