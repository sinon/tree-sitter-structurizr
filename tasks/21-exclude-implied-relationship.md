    # Issue 21: exclude-implied-relationship.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/exclude-implied-relationship.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/exclude-implied-relationship.dsl

    Summary: The upstream Structurizr DSL sample `exclude-implied-relationship.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [11, 10] - [11, 13]
              (ERROR [11, 12] - [11, 13]))
/tmp/structurizr-upstream-dsl-audit/exclude-implied-relationship.dsl	Parse:    0.10 ms	  2853 bytes/ms	(ERROR [11, 10] - [11, 13])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
