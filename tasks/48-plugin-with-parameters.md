    # Issue 48: plugin-with-parameters.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/plugin-with-parameters.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/plugin-with-parameters.dsl

    Summary: The upstream Structurizr DSL sample `plugin-with-parameters.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 4] - [2, 53]
        (ERROR [2, 4] - [2, 51]))
  (ERROR [6, 0] - [6, 1]))
/tmp/structurizr-upstream-dsl-audit/plugin-with-parameters.dsl	Parse:    0.09 ms	  1088 bytes/ms	(ERROR [2, 4] - [2, 53])
    ```

    Suggested next grammar area: script/plugin directive blocks
