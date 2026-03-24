# Issue 8: archetypes.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes.dsl

Summary: The upstream Structurizr DSL sample `archetypes.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 10] - [8, 19]
    (ERROR [2, 10] - [4, 35]
      (ERROR [5, 16] - [5, 33]
  (ERROR [7, 12] - [7, 21])
  (ERROR [8, 16] - [8, 19]))
(ERROR [9, 12] - [41, 51]
  (ERROR [11, 12] - [11, 24])
  (ERROR [11, 27] - [13, 33])
  (ERROR [13, 36] - [13, 47])
  (ERROR [16, 12] - [18, 38]
    (ERROR [18, 12] - [18, 26]))
  (ERROR [20, 16] - [20, 19])
```

Suggested next grammar area: archetypes and custom element/relationship extensions
