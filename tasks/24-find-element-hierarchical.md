    # Issue 24: find-element-hierarchical.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/find-element-hierarchical.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/find-element-hierarchical.dsl

    Summary: The upstream Structurizr DSL sample `find-element-hierarchical.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [9, 16] - [10, 32]
                      (ERROR [9, 16] - [9, 17])
                      (ERROR [9, 25] - [9, 26])
      (ERROR [16, 12] - [18, 36]
        (ERROR [16, 12] - [16, 13])
        (ERROR [16, 21] - [16, 24])
  (ERROR [20, 12] - [30, 1]
    (ERROR [23, 8] - [23, 9])
    (ERROR [23, 17] - [23, 22])
/tmp/structurizr-upstream-dsl-audit/find-element-hierarchical.dsl	Parse:    0.19 ms	  2924 bytes/ms	(ERROR [9, 16] - [10, 32])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
