## Issue

`/Users/rob/dev/zed-structurizr/big-bank-plc/internet-banking-system.dsl` expects Cmd-click on relationship endpoints like `customer -> webApplication` to navigate to definitions in `model/people-and-software-systems.dsl`.

This falls inside the current bounded go-to-definition surface, so the gap should be treated as a regression in implemented behavior rather than a new feature request.

## Root Cause

The LSP backend already wires `textDocument/definition` and resolves supported references through workspace facts in `crates/structurizr-lsp/src/server.rs` and `crates/structurizr-lsp/src/handlers/navigation.rs`.

The analysis layer already extracts `RelationshipSource` and `RelationshipDestination` references in `crates/structurizr-analysis/src/extract/symbols.rs`, so the likely failure is one of:

- the reference is not being resolved in the workspace index for this fixture shape
- candidate workspace instances disagree, causing the handler to return no result conservatively
- downstream Zed integration is not invoking the LSP path expected for this cursor site

## Options

- Reproduce the failure in an LSP integration test using the existing Big Bank fixture shape, then patch the failing analysis or navigation layer.
- Add extra logging and a temporary reduced fixture first, then implement the narrowest fix once the failing layer is known.
- If the LSP responds correctly and the bug is downstream, create a follow-up task in `zed-structurizr` instead of widening the server.

## Proposed Option

Start with a focused regression test in `crates/structurizr-lsp/tests/` against a reduced multi-file workspace that mirrors the Big Bank relationship case.

If the test fails in-repo, fix the relevant analysis or workspace-resolution path and keep the bounded contract unchanged. If it passes, move the follow-up into downstream Zed integration work rather than changing the LSP blindly.
