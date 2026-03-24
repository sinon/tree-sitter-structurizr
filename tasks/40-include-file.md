    # Issue 40: include-file.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/include-file.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/include-file.dsl

    Summary: The upstream Structurizr DSL sample `include-file.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes, Contains `MISSING` nodes

    Parse excerpt:

    ```text
    (ERROR [6, 0] - [6, 1]))
/tmp/structurizr-upstream-dsl-audit/include-file.dsl	Parse:    0.09 ms	   769 bytes/ms	(MISSING "}" [2, 11] - [2, 11])
    ```

    Suggested next grammar area: include and workspace extension directives
