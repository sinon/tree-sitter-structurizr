# Issue 23: filteredviews.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/filteredviews.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/filteredviews.dsl

Summary: The upstream Structurizr DSL sample `filteredviews.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [11, 8] - [11, 12]
          (ERROR [11, 8] - [11, 12]))
        (ERROR [13, 8] - [13, 12]
          (ERROR [13, 8] - [13, 12]))
        (ERROR [14, 8] - [14, 12]
          (ERROR [14, 8] - [14, 12]))
      (ERROR [14, 42] - [14, 48]
        (ERROR [24, 39] - [24, 62]
          (ERROR [24, 39] - [24, 62]))
        (ERROR [25, 17] - [28, 19]
          (ERROR [25, 17] - [25, 30])
          (ERROR [25, 39] - [25, 62])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
