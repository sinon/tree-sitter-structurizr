    # Issue 3: archetypes-for-custom-elements.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes-for-custom-elements.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes-for-custom-elements.dsl

    Summary: The upstream Structurizr DSL sample `archetypes-for-custom-elements.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [16, 1]
    (ERROR [0, 10] - [5, 24]
      (ERROR [2, 10] - [4, 36]
    (ERROR [6, 16] - [13, 14]
      (ERROR [10, 8] - [10, 9])
      (ERROR [11, 8] - [11, 9])
      (ERROR [11, 12] - [11, 26])
      (ERROR [13, 8] - [13, 9])
      (ERROR [13, 13] - [13, 14]))
/tmp/structurizr-upstream-dsl-audit/archetypes-for-custom-elements.dsl	Parse:    0.12 ms	  2510 bytes/ms	(ERROR [0, 0] - [16, 1])
    ```

    Suggested next grammar area: archetypes and custom element/relationship extensions
