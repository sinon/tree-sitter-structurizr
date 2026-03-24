    # Issue 38: identifiers.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/identifiers.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/identifiers.dsl

    Summary: The upstream Structurizr DSL sample `identifiers.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [6, 8] - [6, 24])
              (ERROR [7, 12] - [7, 23])
                    (ERROR [8, 20] - [8, 26]
                      (ERROR [8, 22] - [8, 26]))
/tmp/structurizr-upstream-dsl-audit/identifiers.dsl	Parse:    0.12 ms	  2147 bytes/ms	(ERROR [6, 8] - [6, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
