    # Issue 6: archetypes-for-implicit-relationships.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes-for-implicit-relationships.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes-for-implicit-relationships.dsl

    Summary: The upstream Structurizr DSL sample `archetypes-for-implicit-relationships.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [0, 0] - [14, 1]
    (ERROR [0, 10] - [4, 26]
      (ERROR [1, 10] - [3, 22]
    (ERROR [5, 16] - [11, 23]
      (ERROR [9, 8] - [9, 9])
      (ERROR [10, 8] - [10, 9])
      (ERROR [11, 12] - [11, 19])
      (ERROR [11, 22] - [11, 23]))
/tmp/structurizr-upstream-dsl-audit/archetypes-for-implicit-relationships.dsl	Parse:    0.14 ms	  2078 bytes/ms	(ERROR [0, 0] - [14, 1])
    ```

    Suggested next grammar area: archetypes and custom element/relationship extensions
