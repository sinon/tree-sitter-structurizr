    # Issue 7: archetypes-from-workspace-extension-parent.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes-from-workspace-extension-parent.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes-from-workspace-extension-parent.dsl

    Summary: The upstream Structurizr DSL sample `archetypes-from-workspace-extension-parent.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 10] - [5, 27]
      (ERROR [2, 10] - [4, 26]
    (ERROR [6, 20] - [8, 26]
      (ERROR [9, 20] - [9, 68]
  (ERROR [12, 16] - [33, 1]
    (ERROR [12, 16] - [12, 28])
    (ERROR [20, 16] - [20, 19])
    (ERROR [26, 16] - [26, 28])
/tmp/structurizr-upstream-dsl-audit/archetypes-from-workspace-extension-parent.dsl	Parse:    0.19 ms	  4473 bytes/ms	(ERROR [0, 10] - [5, 27])
    ```

    Suggested next grammar area: archetypes and custom element/relationship extensions
