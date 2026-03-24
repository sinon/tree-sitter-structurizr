    # Issue 22: exclude-relationships.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/exclude-relationships.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/exclude-relationships.dsl

    Summary: The upstream Structurizr DSL sample `exclude-relationships.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 8] - [4, 24])
          (ERROR [16, 8] - [16, 21]
            (ERROR [16, 8] - [16, 13])
/tmp/structurizr-upstream-dsl-audit/exclude-relationships.dsl	Parse:    0.12 ms	  2958 bytes/ms	(ERROR [4, 8] - [4, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
