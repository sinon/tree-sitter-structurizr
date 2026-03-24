    # Issue 30: group-url.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/group-url.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/group-url.dsl

    Summary: The upstream Structurizr DSL sample `group-url.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 41] - [4, 17]
              (ERROR [5, 16] - [5, 41]
  (ERROR [10, 0] - [10, 1]))
/tmp/structurizr-upstream-dsl-audit/group-url.dsl	Parse:    0.11 ms	  1524 bytes/ms	(ERROR [3, 41] - [4, 17])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
