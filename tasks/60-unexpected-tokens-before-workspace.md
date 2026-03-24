    # Issue 60: unexpected-tokens-before-workspace.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/unexpected-tokens-before-workspace.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/unexpected-tokens-before-workspace.dsl

    Summary: The upstream Structurizr DSL sample `unexpected-tokens-before-workspace.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [0, 11]
    (ERROR [0, 0] - [0, 11]))
/tmp/structurizr-upstream-dsl-audit/unexpected-tokens-before-workspace.dsl	Parse:    0.07 ms	   359 bytes/ms	(ERROR [0, 0] - [0, 11])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
