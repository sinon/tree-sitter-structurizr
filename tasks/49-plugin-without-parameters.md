    # Issue 49: plugin-without-parameters.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/plugin-without-parameters.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/plugin-without-parameters.dsl

    Summary: The upstream Structurizr DSL sample `plugin-without-parameters.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [2, 51]
        (ERROR [2, 4] - [2, 51])))))
/tmp/structurizr-upstream-dsl-audit/plugin-without-parameters.dsl	Parse:    0.07 ms	   945 bytes/ms	(ERROR [2, 4] - [2, 51])
    ```

    Suggested next grammar area: script/plugin directive blocks
