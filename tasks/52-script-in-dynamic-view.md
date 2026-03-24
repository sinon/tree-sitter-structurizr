    # Issue 52: script-in-dynamic-view.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/script-in-dynamic-view.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/script-in-dynamic-view.dsl

    Summary: The upstream Structurizr DSL sample `script-in-dynamic-view.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 8] - [4, 24])
          (ERROR [8, 8] - [13, 5]
            (ERROR [8, 24] - [10, 34]
              (ERROR [9, 12] - [9, 26])
/tmp/structurizr-upstream-dsl-audit/script-in-dynamic-view.dsl	Parse:    0.14 ms	  1818 bytes/ms	(ERROR [4, 8] - [4, 24])
    ```

    Suggested next grammar area: dynamic view sequencing and relationship instance grammar
