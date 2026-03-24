    # Issue 11: constant.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/constant.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/constant.dsl

    Summary: The upstream Structurizr DSL sample `constant.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [2, 24]
        (ERROR [2, 4] - [2, 24])))))
/tmp/structurizr-upstream-dsl-audit/constant.dsl	Parse:    0.07 ms	   571 bytes/ms	(ERROR [2, 4] - [2, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
