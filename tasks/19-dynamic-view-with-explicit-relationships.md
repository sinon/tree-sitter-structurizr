    # Issue 19: dynamic-view-with-explicit-relationships.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/dynamic-view-with-explicit-relationships.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/dynamic-view-with-explicit-relationships.dsl

    Summary: The upstream Structurizr DSL sample `dynamic-view-with-explicit-relationships.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 10] - [10, 19]
            (ERROR [6, 11] - [6, 14]
              (ERROR [6, 13] - [6, 14]))
            (ERROR [6, 27] - [7, 16])
      (ERROR [10, 8] - [10, 9])
      (ERROR [10, 13] - [10, 14])
      (ERROR [10, 18] - [10, 19]))
      (ERROR [11, 12] - [11, 24]
  (ERROR [13, 4] - [13, 5])
          (ERROR [22, 12] - [22, 22]
  (ERROR [36, 0] - [36, 1]))
/tmp/structurizr-upstream-dsl-audit/dynamic-view-with-explicit-relationships.dsl	Parse:    0.20 ms	  2940 bytes/ms	(ERROR [0, 10] - [10, 19])
    ```

    Suggested next grammar area: dynamic view sequencing and relationship instance grammar
