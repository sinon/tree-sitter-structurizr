#![allow(clippy::significant_drop_tightening)]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use futures::StreamExt;
use line_index::{LineIndex, TextSize, WideEncoding};
use serde_json::json;
use structurizr_lsp::Backend;
use tower::{Service, ServiceExt};
use tower_lsp_server::{
    ClientSocket, LspService,
    jsonrpc::{Request, Response},
    ls_types::{Position, Uri},
};

const SMALL_SESSION_SOURCE: &str =
    include_str!("../tests/fixtures/relationships/named-relationships-ok.dsl");
const LARGE_SESSION_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl");
const MEGA_DOCUMENT_SYMBOL_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/benchmark-mega/workspace_data/ws-12/model/10-systems.dsl");
const MEGA_DYNAMIC_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/benchmark-mega/global-views.dsl");
const MEGA_MULTI_ROOT_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/model.dsl");

const MEGA_WORKSPACE_ROOTS: &[&str] = &["tests/lsp/workspaces/benchmark-mega"];
const MEGA_MULTI_ROOTS: &[&str] = &[
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-00",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-01",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-02",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-03",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-04",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-05",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-06",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-07",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-08",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-09",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-10",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-11",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-12",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-13",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-14",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-15",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-16",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-17",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-18",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-19",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-20",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-21",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-22",
    "tests/lsp/workspaces/benchmark-mega-multi-root/ws-23",
];

type BenchService = LspService<Backend>;

#[derive(Clone, Copy)]
enum SessionRequest {
    Definition {
        needle: &'static str,
        byte_offset_within_needle: usize,
    },
    DocumentSymbols,
}

#[derive(Clone, Copy)]
struct SessionCase {
    name: &'static str,
    relative_document_path: &'static str,
    source: &'static str,
    workspace_roots: &'static [&'static str],
    request: SessionRequest,
}

const SESSION_CASES: &[SessionCase] = &[
    SessionCase {
        name: "small_named_relationship_definition",
        relative_document_path: "crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl",
        source: SMALL_SESSION_SOURCE,
        workspace_roots: &[],
        request: SessionRequest::Definition {
            needle: "include rel",
            byte_offset_within_needle: 8,
        },
    },
    SessionCase {
        name: "large_big_bank_document_symbols",
        relative_document_path: "tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl",
        source: LARGE_SESSION_SOURCE,
        workspace_roots: &["tests/lsp/workspaces/big-bank-plc"],
        request: SessionRequest::DocumentSymbols,
    },
    SessionCase {
        name: "mega_benchmark_document_symbols",
        relative_document_path: "tests/lsp/workspaces/benchmark-mega/workspace_data/ws-12/model/10-systems.dsl",
        source: MEGA_DOCUMENT_SYMBOL_SOURCE,
        workspace_roots: MEGA_WORKSPACE_ROOTS,
        request: SessionRequest::DocumentSymbols,
    },
    SessionCase {
        name: "mega_benchmark_dynamic_definition",
        relative_document_path: "tests/lsp/workspaces/benchmark-mega/global-views.dsl",
        source: MEGA_DYNAMIC_SOURCE,
        workspace_roots: MEGA_WORKSPACE_ROOTS,
        request: SessionRequest::Definition {
            needle: "w13_sys00_api_comp00",
            byte_offset_within_needle: 5,
        },
    },
    SessionCase {
        name: "mega_multi_root_document_symbols",
        relative_document_path: "tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/model.dsl",
        source: MEGA_MULTI_ROOT_SOURCE,
        workspace_roots: MEGA_MULTI_ROOTS,
        request: SessionRequest::DocumentSymbols,
    },
];

// These sessions measure the exact LSP flow contributors feel in practice:
// initialize, open, full-document change, and one bounded follow-up request,
// while scaling from tiny single-file edits to a generated mega corpus and a
// multi-root workspace-folder session.
fn bench_lsp_sessions(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("benchmark runtime should build");
    let mut group = c.benchmark_group("lsp/session");

    for case in SESSION_CASES {
        let throughput =
            u64::try_from(case.source.len()).expect("session fixture size should fit into u64");
        group.throughput(Throughput::Bytes(throughput));
        group.bench_with_input(BenchmarkId::from_parameter(case.name), case, |b, case| {
            b.iter(|| runtime.block_on(run_session(*case)));
        });
    }

    group.finish();
}

async fn run_session(case: SessionCase) {
    let repo_root = repo_root();
    let document_path = repo_root.join(case.relative_document_path);
    let document_uri = file_uri_from_path(&document_path);
    let workspace_folders = case
        .workspace_roots
        .iter()
        .map(|relative_root| file_uri_from_path(&repo_root.join(relative_root)))
        .collect::<Vec<_>>();
    let changed_text = format!("{}\n", case.source);
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &workspace_folders).await;
    initialized(&mut service).await;
    open_document(&mut service, &document_uri, case.source).await;
    wait_for_publish_diagnostics(&mut socket).await;
    change_document(&mut service, &document_uri, 2, &changed_text).await;
    wait_for_publish_diagnostics(&mut socket).await;

    match case.request {
        SessionRequest::Definition {
            needle,
            byte_offset_within_needle,
        } => {
            let position = position_in(case.source, needle, byte_offset_within_needle);
            let response = call_request(
                &mut service,
                Request::build("textDocument/definition")
                    .params(json!({
                        "textDocument": { "uri": document_uri.as_str() },
                        "position": position,
                    }))
                    .id(2)
                    .finish(),
            )
            .await;
            black_box(response);
        }
        SessionRequest::DocumentSymbols => {
            let response = call_request(
                &mut service,
                Request::build("textDocument/documentSymbol")
                    .params(json!({
                        "textDocument": { "uri": document_uri.as_str() },
                    }))
                    .id(2)
                    .finish(),
            )
            .await;
            black_box(response);
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root should exist")
}

fn new_service() -> (BenchService, ClientSocket) {
    LspService::new(Backend::new)
}

async fn initialize_with_workspace_folders(service: &mut BenchService, workspace_folders: &[Uri]) {
    let response = call_request(
        service,
        Request::build("initialize")
            .params(json!({
                "capabilities": {},
                "workspaceFolders": workspace_folders
                    .iter()
                    .map(|uri| json!({ "uri": uri.as_str(), "name": "bench-workspace" }))
                    .collect::<Vec<_>>(),
            }))
            .id(1)
            .finish(),
    )
    .await;

    black_box(response);
}

async fn initialized(service: &mut BenchService) {
    call_notification(
        service,
        Request::build("initialized").params(json!({})).finish(),
    )
    .await;
}

async fn open_document(service: &mut BenchService, uri: &Uri, text: &str) {
    call_notification(
        service,
        Request::build("textDocument/didOpen")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str(),
                    "languageId": "Structurizr DSL",
                    "version": 1,
                    "text": text,
                }
            }))
            .finish(),
    )
    .await;
}

async fn change_document(service: &mut BenchService, uri: &Uri, version: i32, text: &str) {
    call_notification(
        service,
        Request::build("textDocument/didChange")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str(),
                    "version": version,
                },
                "contentChanges": [
                    {
                        "text": text,
                    }
                ],
            }))
            .finish(),
    )
    .await;
}

async fn wait_for_publish_diagnostics(socket: &mut ClientSocket) {
    let message = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let request = socket
                .next()
                .await
                .expect("server should send a notification");
            let message = serde_json::to_value(request).expect("server request should serialize");

            if message["method"] == "textDocument/publishDiagnostics" {
                return message;
            }
        }
    })
    .await
    .expect("server should publish diagnostics within timeout");

    black_box(message);
}

fn file_uri_from_path(path: &Path) -> Uri {
    Uri::from_file_path(path).expect("file path URI should parse")
}

fn position_in(text: &str, needle: &str, byte_offset_within_needle: usize) -> Position {
    let start = text
        .find(needle)
        .expect("needle should exist in benchmark text");
    let offset = start + byte_offset_within_needle;
    let index = LineIndex::new(text);
    let utf8 = index
        .try_line_col(TextSize::from(
            u32::try_from(offset).expect("offset should fit into u32"),
        ))
        .expect("offset should point at a valid boundary");
    let wide = index
        .to_wide(WideEncoding::Utf16, utf8)
        .expect("offset should map to a UTF-16 position");

    Position::new(wide.line, wide.col)
}

async fn call_request(service: &mut BenchService, request: Request) -> Response {
    service
        .ready()
        .await
        .expect("service should become ready")
        .call(request)
        .await
        .expect("request call should succeed")
        .expect("request should produce a response")
}

async fn call_notification(service: &mut BenchService, request: Request) {
    let response = service
        .ready()
        .await
        .expect("service should become ready")
        .call(request)
        .await
        .expect("notification call should succeed");

    assert!(
        response.is_none(),
        "notifications should not return a response"
    );
}

criterion_group!(benches, bench_lsp_sessions);
criterion_main!(benches);
