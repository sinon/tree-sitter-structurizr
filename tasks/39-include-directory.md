    # Issue 39: include-directory.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/include-directory.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/include-directory.dsl

    Summary: The upstream Structurizr DSL sample `include-directory.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [3, 53]
        (ERROR [3, 8] - [3, 33])
      (ERROR [6, 8] - [6, 53]
        (ERROR [6, 8] - [6, 33])
      (ERROR [9, 8] - [9, 53]
        (ERROR [9, 8] - [9, 33])
  (ERROR [13, 0] - [13, 1]))
/tmp/structurizr-upstream-dsl-audit/include-directory.dsl	Parse:    0.18 ms	  1805 bytes/ms	(ERROR [2, 4] - [3, 53])
    ```

    Suggested next grammar area: include and workspace extension directives
