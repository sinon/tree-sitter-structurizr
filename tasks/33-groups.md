# Issue 33: groups.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/groups.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/groups.dsl

Summary: The upstream Structurizr DSL sample `groups.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 0] - [50, 1]
(ERROR [0, 10] - [11, 39]
  (ERROR [2, 10] - [10, 40]
      (ERROR [3, 23] - [3, 39])
      (ERROR [3, 58] - [4, 28]
        (ERROR [4, 23] - [4, 28]))
    (ERROR [10, 23] - [10, 28])
(ERROR [12, 16] - [21, 25]
  (ERROR [14, 16] - [14, 23])
  (ERROR [14, 24] - [14, 27])
  (ERROR [14, 31] - [14, 38])
  (ERROR [14, 39] - [14, 47])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
