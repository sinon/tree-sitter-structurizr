    # Issue 44: multi-line.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/multi-line.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/multi-line.dsl

    Summary: The upstream Structurizr DSL sample `multi-line.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 8] - [6, 19]
            (ERROR [3, 25] - [3, 26])
            (ERROR [4, 27] - [6, 19])))))))
/tmp/structurizr-upstream-dsl-audit/multi-line.dsl	Parse:    0.11 ms	  1242 bytes/ms	(ERROR [3, 8] - [6, 19])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
