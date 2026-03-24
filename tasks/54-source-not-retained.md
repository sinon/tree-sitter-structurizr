    # Issue 54: source-not-retained.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/source-not-retained.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/source-not-retained.dsl

    Summary: The upstream Structurizr DSL sample `source-not-retained.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [3, 36]
        (ERROR [3, 8] - [3, 36]))))
  (ERROR [10, 0] - [10, 1]))
/tmp/structurizr-upstream-dsl-audit/source-not-retained.dsl	Parse:    0.10 ms	  1254 bytes/ms	(ERROR [2, 4] - [3, 36])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
