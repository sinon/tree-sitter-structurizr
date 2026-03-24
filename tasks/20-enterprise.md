    # Issue 20: enterprise.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/enterprise.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/enterprise.dsl

    Summary: The upstream Structurizr DSL sample `enterprise.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [3, 8] - [3, 27]
  (ERROR [8, 0] - [8, 1]))
/tmp/structurizr-upstream-dsl-audit/enterprise.dsl	Parse:    0.09 ms	  1065 bytes/ms	(ERROR [3, 8] - [3, 27])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
