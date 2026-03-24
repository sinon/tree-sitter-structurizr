    # Issue 43: multi-line-with-error.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/multi-line-with-error.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/multi-line-with-error.dsl

    Summary: The upstream Structurizr DSL sample `multi-line-with-error.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 8] - [3, 26]
            (ERROR [3, 25] - [3, 26]))
            (ERROR [4, 27] - [7, 25]
              (ERROR [4, 27] - [6, 21]))
  (ERROR [11, 0] - [11, 1]))
/tmp/structurizr-upstream-dsl-audit/multi-line-with-error.dsl	Parse:    0.10 ms	  2330 bytes/ms	(ERROR [3, 8] - [3, 26])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
