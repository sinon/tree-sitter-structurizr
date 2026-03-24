    # Issue 41: include-implied-relationship.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/include-implied-relationship.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/include-implied-relationship.dsl

    Summary: The upstream Structurizr DSL sample `include-implied-relationship.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [11, 10] - [11, 13]
              (ERROR [11, 12] - [11, 13]))
                (ERROR [17, 21] - [17, 23])
/tmp/structurizr-upstream-dsl-audit/include-implied-relationship.dsl	Parse:    0.13 ms	  2471 bytes/ms	(ERROR [11, 10] - [11, 13])
    ```

    Suggested next grammar area: include and workspace extension directives
