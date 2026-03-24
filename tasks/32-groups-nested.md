# Issue 32: groups-nested.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/groups-nested.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/groups-nested.dsl

Summary: The upstream Structurizr DSL sample `groups-nested.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 10] - [8, 17]
    (ERROR [2, 10] - [3, 18]
      (ERROR [4, 12] - [4, 40]
        (ERROR [4, 39] - [4, 40]))))
  (ERROR [7, 8] - [7, 13])
  (ERROR [8, 12] - [8, 17]))
(ERROR [8, 33] - [36, 30]
  (ERROR [9, 16] - [9, 17])
  (ERROR [10, 20] - [10, 25])
  (ERROR [11, 24] - [11, 29])
  (ERROR [13, 32] - [13, 37])
  (ERROR [15, 40] - [15, 45])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
