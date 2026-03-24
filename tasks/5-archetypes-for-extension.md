# Issue 5: archetypes-for-extension.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/archetypes-for-extension.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/archetypes-for-extension.dsl

Summary: The upstream Structurizr DSL sample `archetypes-for-extension.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 0] - [32, 1]
(ERROR [0, 10] - [9, 19]
    (ERROR [2, 10] - [4, 26]
      (ERROR [5, 16] - [5, 33]
  (ERROR [8, 12] - [8, 34])
  (ERROR [9, 16] - [9, 19]))
(ERROR [10, 12] - [29, 21]
  (ERROR [14, 16] - [14, 19])
  (ERROR [15, 12] - [17, 21]
    (ERROR [17, 12] - [17, 16]))
  (ERROR [18, 16] - [18, 19])
  (ERROR [19, 12] - [21, 28]
```

Suggested next grammar area: archetypes and custom element/relationship extensions
