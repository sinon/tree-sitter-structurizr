# Issue 1: amazon-web-services-local.dsl

Source fixture: `structurizr-dsl/src/test/resources/dsl/amazon-web-services-local.dsl`

Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/amazon-web-services-local.dsl

Summary: The upstream Structurizr DSL sample `amazon-web-services-local.dsl` does not yet parse cleanly with the local tree-sitter grammar.

Symptoms: Contains `ERROR` nodes

Parse excerpt:

```text
(ERROR [0, 40] - [25, 60]
    (ERROR [4, 10] - [5, 193]
      (ERROR [5, 26] - [5, 40])
        (ERROR [6, 27] - [8, 26]
          (ERROR [7, 12] - [7, 20])
          (ERROR [8, 12] - [8, 26]))
  (ERROR [11, 8] - [11, 12])
  (ERROR [11, 25] - [11, 36])
  (ERROR [12, 12] - [12, 15])
  (ERROR [12, 28] - [12, 32])
  (ERROR [13, 16] - [13, 22])
  (ERROR [13, 35] - [13, 39])
```

Suggested next grammar area: remaining advanced DSL constructs in this sample
