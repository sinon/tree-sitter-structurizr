    # Issue 12: constants-and-variables-from-workspace-extension-parent.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/constants-and-variables-from-workspace-extension-parent.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/constants-and-variables-from-workspace-extension-parent.dsl

    Summary: The upstream Structurizr DSL sample `constants-and-variables-from-workspace-extension-parent.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [5, 1]
    (ERROR [0, 10] - [2, 10]
      (ERROR [2, 4] - [2, 10]))
    (ERROR [3, 4] - [3, 8])
/tmp/structurizr-upstream-dsl-audit/constants-and-variables-from-workspace-extension-parent.dsl	Parse:    0.08 ms	   953 bytes/ms	(ERROR [0, 0] - [5, 1])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
