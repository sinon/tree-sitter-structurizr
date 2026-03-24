    # Issue 42: include-url.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/include-url.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/include-url.dsl

    Summary: The upstream Structurizr DSL sample `include-url.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes, Contains `MISSING` nodes

    Parse excerpt:

    ```text
    (ERROR [6, 0] - [6, 1]))
/tmp/structurizr-upstream-dsl-audit/include-url.dsl	Parse:    0.08 ms	  2102 bytes/ms	(MISSING "}" [2, 11] - [2, 11])
    ```

    Suggested next grammar area: include and workspace extension directives
