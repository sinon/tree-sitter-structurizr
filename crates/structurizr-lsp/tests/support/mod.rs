#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use futures::StreamExt;
use line_index::{LineIndex, TextSize, WideEncoding};
use serde_json::{Value, json};
use structurizr_lsp::Backend;
use tower::{Service, ServiceExt};
use tower_lsp_server::{
    ClientSocket, LspService,
    jsonrpc::{Request, Response},
    ls_types::{Position, Uri},
};

pub type TestService = LspService<Backend>;

pub fn new_service() -> (TestService, ClientSocket) {
    LspService::new(Backend::new)
}

pub async fn initialize(service: &mut TestService) -> Value {
    initialize_with_workspace_folders(service, &[]).await
}

pub async fn initialize_with_workspace_folders(
    service: &mut TestService,
    workspace_folders: &[Uri],
) -> Value {
    let response = call_request(
        service,
        Request::build("initialize")
            .params(json!({
                "capabilities": {},
                "workspaceFolders": workspace_folders
                    .iter()
                    .map(|uri| json!({ "uri": uri.as_str(), "name": "test-workspace" }))
                    .collect::<Vec<_>>(),
            }))
            .id(1)
            .finish(),
    )
    .await;

    response_json(response)
}

pub async fn initialized(service: &mut TestService) {
    call_notification(
        service,
        Request::build("initialized")
            .params(json!({}))
            .finish(),
    )
    .await;
}

pub async fn open_document(service: &mut TestService, uri: &Uri, text: &str) {
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

pub async fn request_json(
    service: &mut TestService,
    method: &'static str,
    params: Value,
    id: i64,
) -> Value {
    let response = call_request(service, Request::build(method).params(params).id(id).finish()).await;
    response_json(response)
}

pub async fn next_server_notification(socket: &mut ClientSocket) -> Value {
    let request = socket.next().await.expect("server should send a notification");
    serde_json::to_value(request).expect("server request should serialize")
}

pub fn file_uri(name: &str) -> Uri {
    Uri::from_str(&format!("file:///{name}")).expect("test URI should parse")
}

pub fn file_uri_from_path(path: &Path) -> Uri {
    Uri::from_str(&format!("file://{}", path.to_string_lossy())).expect("file path URI should parse")
}

pub fn workspace_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/lsp/workspaces")
        .join(name)
        .canonicalize()
        .expect("workspace fixture should exist")
}

pub fn position_in(text: &str, needle: &str, byte_offset_within_needle: usize) -> Position {
    let start = text.find(needle).expect("needle should exist in test text");
    let offset = start + byte_offset_within_needle;
    let index = LineIndex::new(text);
    let utf8 = index
        .try_line_col(TextSize::from(u32::try_from(offset).expect("offset should fit in u32")))
        .expect("offset should point at a valid boundary");
    let wide = index
        .to_wide(WideEncoding::Utf16, utf8)
        .expect("offset should map to a UTF-16 position");

    Position::new(wide.line, wide.col)
}

async fn call_request(service: &mut TestService, request: Request) -> Response {
    service
        .ready()
        .await
        .expect("service should become ready")
        .call(request)
        .await
        .expect("request call should succeed")
        .expect("request should produce a response")
}

async fn call_notification(service: &mut TestService, request: Request) {
    let response = service
        .ready()
        .await
        .expect("service should become ready")
        .call(request)
        .await
        .expect("notification call should succeed");

    assert!(response.is_none(), "notifications should not return a response");
}

fn response_json(response: Response) -> Value {
    serde_json::to_value(response).expect("response should serialize")
}
