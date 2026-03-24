# Issue 9: big-bank-plc.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/big-bank-plc.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/big-bank-plc.dsl

Summary: The upstream Structurizr DSL sample `big-bank-plc.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 0] - [248, 1]
    (ERROR [8, 10] - [69, 82]
      (ERROR [15, 22] - [19, 33]
        (ERROR [15, 24] - [15, 38])
        (ERROR [16, 12] - [16, 17])
        (ERROR [16, 20] - [16, 34])
        (ERROR [17, 12] - [17, 15])
        (ERROR [17, 18] - [17, 32])
        (ERROR [19, 12] - [19, 33]))
      (ERROR [19, 36] - [19, 50])
      (ERROR [20, 38] - [23, 30]
        (ERROR [21, 16] - [21, 25])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
