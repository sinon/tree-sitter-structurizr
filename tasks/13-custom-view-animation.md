    # Issue 13: custom-view-animation.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/custom-view-animation.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/custom-view-animation.dsl

    Summary: The upstream Structurizr DSL sample `custom-view-animation.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [7, 5]
    (ERROR [0, 10] - [3, 19]
    (ERROR [4, 8] - [4, 19]
    (ERROR [6, 8] - [6, 9])
    (ERROR [6, 13] - [6, 14]))
        (ERROR [10, 15] - [14, 21]
          (ERROR [15, 16] - [16, 17]
            (ERROR [15, 16] - [16, 17]))))))
  (ERROR [20, 8] - [31, 1]
    (ERROR [24, 12] - [24, 21])
    (ERROR [25, 16] - [26, 17])))
/tmp/structurizr-upstream-dsl-audit/custom-view-animation.dsl	Parse:    0.17 ms	  2950 bytes/ms	(ERROR [0, 0] - [7, 5])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
