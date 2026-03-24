    # Issue 63: workspace-with-bom-model.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/workspace-with-bom-model.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/workspace-with-bom-model.dsl

    Summary: The upstream Structurizr DSL sample `workspace-with-bom-model.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [2, 8] - [2, 24])
/tmp/structurizr-upstream-dsl-audit/workspace-with-bom-model.dsl	Parse:    0.14 ms	  1612 bytes/ms	(ERROR [2, 8] - [2, 24])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
