    # Issue 46: parallel1.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/parallel1.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/parallel1.dsl

    Summary: The upstream Structurizr DSL sample `parallel1.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 8] - [4, 24])
        (ERROR [17, 10] - [23, 13]
            (ERROR [18, 31] - [20, 25]
          (ERROR [25, 16] - [25, 27]
            (ERROR [25, 16] - [25, 19])
            (ERROR [25, 23] - [25, 26])
      (ERROR [28, 12] - [28, 22])))
  (ERROR [30, 4] - [32, 1]))
/tmp/structurizr-upstream-dsl-audit/parallel1.dsl	Parse:    0.18 ms	  3905 bytes/ms	(ERROR [4, 8] - [4, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
