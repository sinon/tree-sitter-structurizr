    # Issue 26: find-elements-in-flat-group.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/find-elements-in-flat-group.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/find-elements-in-flat-group.dsl

    Summary: The upstream Structurizr DSL sample `find-elements-in-flat-group.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [16, 1]
    (ERROR [0, 10] - [12, 24]
            (ERROR [5, 16] - [5, 31]
              (ERROR [5, 16] - [5, 21])
      (ERROR [11, 8] - [11, 9])
      (ERROR [11, 16] - [11, 23])
      (ERROR [12, 12] - [12, 16])
      (ERROR [12, 20] - [12, 24]))
/tmp/structurizr-upstream-dsl-audit/find-elements-in-flat-group.dsl	Parse:    0.13 ms	  2013 bytes/ms	(ERROR [0, 0] - [16, 1])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
