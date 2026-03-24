    # Issue 61: unexpected-tokens-in-workspace.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/unexpected-tokens-in-workspace.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/unexpected-tokens-in-workspace.dsl

    Summary: The upstream Structurizr DSL sample `unexpected-tokens-in-workspace.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [2, 25]
/tmp/structurizr-upstream-dsl-audit/unexpected-tokens-in-workspace.dsl	Parse:    0.09 ms	   471 bytes/ms	(ERROR [2, 4] - [2, 25])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
