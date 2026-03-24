    # Issue 55: static-view-animation.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/static-view-animation.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/static-view-animation.dsl

    Summary: The upstream Structurizr DSL sample `static-view-animation.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [10, 24] - [14, 21]
              (ERROR [15, 16] - [16, 17]
                (ERROR [15, 16] - [16, 17]))))))
      (ERROR [20, 8] - [26, 19]
        (ERROR [24, 12] - [24, 21])
        (ERROR [25, 16] - [26, 17]))))
  (ERROR [28, 8] - [31, 1]))
/tmp/structurizr-upstream-dsl-audit/static-view-animation.dsl	Parse:    0.23 ms	  2235 bytes/ms	(ERROR [10, 24] - [14, 21])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
