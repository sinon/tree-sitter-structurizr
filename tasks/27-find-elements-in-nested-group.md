# Issue 27: find-elements-in-nested-group.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/find-elements-in-nested-group.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/find-elements-in-nested-group.dsl

Summary: The upstream Structurizr DSL sample `find-elements-in-nested-group.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 0] - [33, 1]
(ERROR [0, 10] - [7, 22]
      (ERROR [3, 8] - [4, 44]
  (ERROR [7, 8] - [7, 12])
(ERROR [8, 8] - [29, 25]
  (ERROR [10, 8] - [10, 18])
  (ERROR [10, 22] - [10, 27])
  (ERROR [11, 12] - [11, 16])
  (ERROR [11, 20] - [11, 25])
  (ERROR [15, 12] - [15, 16])
  (ERROR [15, 20] - [15, 25])
  (ERROR [19, 12] - [19, 16])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
