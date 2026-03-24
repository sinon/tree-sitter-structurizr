    # Issue 47: parallel2.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/parallel2.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/parallel2.dsl

    Summary: The upstream Structurizr DSL sample `parallel2.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [15, 10] - [22, 17]
            (ERROR [17, 18] - [19, 13]
          (ERROR [24, 20] - [24, 43]
            (ERROR [24, 20] - [24, 21])
            (ERROR [24, 25] - [24, 26])
  (ERROR [27, 12] - [33, 1]
    (ERROR [27, 12] - [27, 13])
    (ERROR [27, 17] - [27, 18])
/tmp/structurizr-upstream-dsl-audit/parallel2.dsl	Parse:    0.19 ms	  3024 bytes/ms	(ERROR [15, 10] - [22, 17])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
