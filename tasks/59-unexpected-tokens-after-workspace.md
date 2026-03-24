    # Issue 59: unexpected-tokens-after-workspace.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/unexpected-tokens-after-workspace.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/unexpected-tokens-after-workspace.dsl

    Summary: The upstream Structurizr DSL sample `unexpected-tokens-after-workspace.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 0] - [3, 11]
    (ERROR [3, 0] - [3, 11])))
/tmp/structurizr-upstream-dsl-audit/unexpected-tokens-after-workspace.dsl	Parse:    0.07 ms	   364 bytes/ms	(ERROR [3, 0] - [3, 11])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
