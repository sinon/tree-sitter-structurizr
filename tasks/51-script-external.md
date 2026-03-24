    # Issue 51: script-external.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/script-external.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/script-external.dsl

    Summary: The upstream Structurizr DSL sample `script-external.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [4, 19]
        (ERROR [2, 4] - [4, 19])))))
/tmp/structurizr-upstream-dsl-audit/script-external.dsl	Parse:    0.07 ms	  1099 bytes/ms	(ERROR [2, 4] - [4, 19])
    ```

    Suggested next grammar area: script/plugin directive blocks
