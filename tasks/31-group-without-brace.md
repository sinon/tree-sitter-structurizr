    # Issue 31: group-without-brace.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/group-without-brace.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/group-without-brace.dsl

    Summary: The upstream Structurizr DSL sample `group-without-brace.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 8] - [3, 20]
/tmp/structurizr-upstream-dsl-audit/group-without-brace.dsl	Parse:    0.08 ms	   700 bytes/ms	(ERROR [3, 8] - [3, 20])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
