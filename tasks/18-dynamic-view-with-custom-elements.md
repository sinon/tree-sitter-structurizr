    # Issue 18: dynamic-view-with-custom-elements.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/dynamic-view-with-custom-elements.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/dynamic-view-with-custom-elements.dsl

    Summary: The upstream Structurizr DSL sample `dynamic-view-with-custom-elements.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [9, 5]
    (ERROR [0, 10] - [3, 19]
    (ERROR [4, 8] - [5, 19]
      (ERROR [5, 8] - [5, 9]))
    (ERROR [7, 8] - [7, 9])
    (ERROR [7, 13] - [8, 9])
    (ERROR [8, 13] - [8, 14]))
  (ERROR [18, 0] - [18, 1]))
/tmp/structurizr-upstream-dsl-audit/dynamic-view-with-custom-elements.dsl	Parse:    0.11 ms	  2109 bytes/ms	(ERROR [0, 0] - [9, 5])
    ```

    Suggested next grammar area: dynamic view sequencing and relationship instance grammar
