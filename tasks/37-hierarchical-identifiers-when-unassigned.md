    # Issue 37: hierarchical-identifiers-when-unassigned.dsl

    Source fixture: `structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-when-unassigned.dsl`

    Upstream URL: https://github.com/structurizr/structurizr/blob/4deaec9472083a64733a09e6366ecc09f50b6905/structurizr-dsl/src/test/resources/dsl/hierarchical-identifiers-when-unassigned.dsl

    Summary: The upstream Structurizr DSL sample `hierarchical-identifiers-when-unassigned.dsl` does not yet parse cleanly with the local tree-sitter grammar.

    Symptoms: Contains `ERROR` nodes

    Parse excerpt:

    ```text
    (ERROR [4, 10] - [12, 44]
          (ERROR [13, 16] - [13, 56]
  (ERROR [17, 4] - [18, 1]))
/tmp/structurizr-upstream-dsl-audit/hierarchical-identifiers-when-unassigned.dsl	Parse:    0.13 ms	  2743 bytes/ms	(ERROR [4, 10] - [12, 44])
    ```

    Suggested next grammar area: remaining advanced DSL constructs in this sample
