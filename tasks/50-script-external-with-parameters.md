    # Issue 50: script-external-with-parameters.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/script-external-with-parameters.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/script-external-with-parameters.dsl

    Summary: The upstream Structurizr DSL sample `script-external-with-parameters.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [6, 1]
    (ERROR [0, 10] - [2, 25]
      (ERROR [2, 4] - [2, 23]))
/tmp/structurizr-upstream-dsl-audit/script-external-with-parameters.dsl	Parse:    0.07 ms	   976 bytes/ms	(ERROR [0, 0] - [6, 1])
    ```

    Suggested next grammar area: script/plugin directive blocks
