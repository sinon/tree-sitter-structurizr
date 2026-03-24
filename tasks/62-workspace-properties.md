    # Issue 62: workspace-properties.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/workspace-properties.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/workspace-properties.dsl

    Summary: The upstream Structurizr DSL sample `workspace-properties.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [6, 1]
    (ERROR [0, 10] - [2, 16])
/tmp/structurizr-upstream-dsl-audit/workspace-properties.dsl	Parse:    0.08 ms	   932 bytes/ms	(ERROR [0, 0] - [6, 1])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
