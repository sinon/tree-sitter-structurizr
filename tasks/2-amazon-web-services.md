# Issue 2: amazon-web-services.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/amazon-web-services.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/amazon-web-services.dsl

Summary: The upstream Structurizr DSL sample `amazon-web-services.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 40] - [55, 41]
  (ERROR [2, 10] - [12, 48]
    (ERROR [3, 26] - [3, 40])
      (ERROR [4, 27] - [8, 22]
        (ERROR [5, 12] - [5, 20])
        (ERROR [8, 8] - [8, 22]))
    (ERROR [10, 25] - [10, 36])
  (ERROR [15, 16] - [15, 22])
  (ERROR [15, 35] - [15, 39])
  (ERROR [18, 20] - [18, 25])
  (ERROR [18, 30] - [18, 48])
  (ERROR [22, 20] - [22, 23])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
