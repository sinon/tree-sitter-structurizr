#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicI64, Ordering},
    },
};

use futures::StreamExt;
use line_index::{LineIndex, TextSize, WideEncoding};
use serde_json::{Value, json};
use structurizr_lsp::Backend;
use tokio::time::{Duration, timeout};
use tower::{Service, ServiceExt};
use tower_lsp_server::{
    ClientSocket, LspService,
    jsonrpc::{Request, Response},
    ls_types::{Position, Uri},
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, writer::BoxMakeWriter},
    prelude::*,
};

pub type TestService = LspService<Backend>;

const LOG_FORMAT_ENV: &str = "STRZ_LOG_FORMAT";
const LOG_FILE_ENV: &str = "STRZ_LOG_FILE";
const TEST_LOG_ENV: &str = "STRZ_TEST_LOG";
const DEFAULT_CURSOR_MARKER_NAME: &str = "__default__";

static TEST_TRACING_INITIALIZED: OnceLock<()> = OnceLock::new();
static NEXT_REQUEST_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Clone)]
struct SharedFileWriter {
    file: Arc<Mutex<File>>,
}

impl Write for SharedFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("test log file lock should not be poisoned"))?
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("test log file lock should not be poisoned"))?
            .flush()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat {
    Compact,
    Json,
}

impl LogFormat {
    fn from_env() -> Self {
        match env::var(LOG_FORMAT_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("json") => Self::Json,
            _ => Self::Compact,
        }
    }
}

struct OutputWriter {
    make_writer: BoxMakeWriter,
    supports_ansi: bool,
}

pub fn new_service() -> (TestService, ClientSocket) {
    init_test_tracing("structurizr-lsp-tests");
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
        json_rpc_request(
            "initialize",
            json!({
                "capabilities": {},
                "workspaceFolders": workspace_folders
                    .iter()
                    .map(|uri| json!({ "uri": uri.as_str(), "name": "test-workspace" }))
                    .collect::<Vec<_>>(),
            }),
        ),
    )
    .await;

    response_json(response)
}

pub async fn initialized(service: &mut TestService) {
    call_notification(
        service,
        Request::build("initialized").params(json!({})).finish(),
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

pub async fn change_document(service: &mut TestService, uri: &Uri, version: i32, text: &str) {
    call_notification(
        service,
        Request::build("textDocument/didChange")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str(),
                    "version": version,
                },
                "contentChanges": [{ "text": text }],
            }))
            .finish(),
    )
    .await;
}

pub async fn close_document(service: &mut TestService, uri: &Uri) {
    call_notification(
        service,
        Request::build("textDocument/didClose")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str(),
                }
            }))
            .finish(),
    )
    .await;
}

pub async fn request_json(service: &mut TestService, method: &'static str, params: Value) -> Value {
    let response = call_request(service, json_rpc_request(method, params)).await;
    response_json(response)
}

pub async fn next_server_notification(socket: &mut ClientSocket) -> Value {
    let request = socket
        .next()
        .await
        .expect("server should send a notification");
    serde_json::to_value(request).expect("server request should serialize")
}

pub async fn next_server_notification_with_timeout(
    socket: &mut ClientSocket,
    wait: Duration,
) -> Value {
    timeout(wait, next_server_notification(socket))
        .await
        .expect("server notification should arrive before timeout")
}

pub async fn next_publish_diagnostics_for_uri(
    socket: &mut ClientSocket,
    expected_uri: &str,
) -> Value {
    for _ in 0..8 {
        let notification =
            next_server_notification_with_timeout(socket, Duration::from_secs(2)).await;

        if notification["method"] == "textDocument/publishDiagnostics"
            && notification["params"]["uri"] == expected_uri
        {
            return notification;
        }
    }

    panic!("did not receive diagnostics for `{expected_uri}`");
}

pub fn file_uri(name: &str) -> Uri {
    Uri::from_str(&format!("file:///{name}")).expect("test URI should parse")
}

pub fn file_uri_from_path(path: &Path) -> Uri {
    Uri::from_file_path(path).expect("file path URI should parse")
}

pub fn workspace_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/lsp/workspaces")
        .join(name)
        .canonicalize()
        .expect("workspace fixture should exist")
}

#[derive(Debug, Clone)]
pub struct AnnotatedSource {
    source: String,
    cursor_offsets: BTreeMap<String, usize>,
}

impl AnnotatedSource {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn position(&self, name: &str) -> Position {
        let offset = self
            .cursor_offsets
            .get(name)
            .unwrap_or_else(|| panic!("cursor marker `{name}` should exist"));
        position_at_offset(&self.source, *offset)
    }

    pub fn only_position(&self) -> Position {
        assert_eq!(
            self.cursor_offsets.len(),
            1,
            "annotated source should contain exactly one cursor marker"
        );
        let offset = *self
            .cursor_offsets
            .values()
            .next()
            .expect("one cursor marker should exist");
        position_at_offset(&self.source, offset)
    }
}

pub fn annotated_source(text: &str) -> AnnotatedSource {
    let mut source = String::with_capacity(text.len());
    let mut cursor_offsets = BTreeMap::new();
    let mut remaining = text;

    while let Some(marker_start) = remaining.find("<CURSOR") {
        source.push_str(&remaining[..marker_start]);
        let marker = &remaining[marker_start..];
        let offset = source.len();

        if let Some(rest) = marker.strip_prefix("<CURSOR>") {
            insert_cursor_marker(&mut cursor_offsets, DEFAULT_CURSOR_MARKER_NAME, offset);
            remaining = rest;
            continue;
        }

        let named_marker = marker
            .strip_prefix("<CURSOR:")
            .expect("cursor markers should use `<CURSOR>` or `<CURSOR:name>`");
        let marker_end = named_marker
            .find('>')
            .expect("named cursor marker should end with `>`");
        let marker_name = &named_marker[..marker_end];
        assert!(
            !marker_name.is_empty(),
            "named cursor marker should include a marker name"
        );
        insert_cursor_marker(&mut cursor_offsets, marker_name, offset);
        remaining = &named_marker[marker_end + 1..];
    }

    source.push_str(remaining);
    AnnotatedSource {
        source,
        cursor_offsets,
    }
}

pub fn position_in(text: &str, needle: &str, byte_offset_within_needle: usize) -> Position {
    let start = text.find(needle).expect("needle should exist in test text");
    let offset = start + byte_offset_within_needle;
    position_at_offset(text, offset)
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

    assert!(
        response.is_none(),
        "notifications should not return a response"
    );
}

fn response_json(response: Response) -> Value {
    serde_json::to_value(response).expect("response should serialize")
}

fn insert_cursor_marker(
    cursor_offsets: &mut BTreeMap<String, usize>,
    marker_name: &str,
    offset: usize,
) {
    assert!(
        cursor_offsets
            .insert(marker_name.to_owned(), offset)
            .is_none(),
        "cursor marker `{marker_name}` should be unique"
    );
}

fn json_rpc_request(method: &'static str, params: Value) -> Request {
    Request::build(method)
        .params(params)
        .id(next_request_id())
        .finish()
}

fn next_request_id() -> i64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn position_at_offset(text: &str, offset: usize) -> Position {
    let index = LineIndex::new(text);
    let utf8 = index
        .try_line_col(TextSize::from(
            u32::try_from(offset).expect("offset should fit in u32"),
        ))
        .expect("offset should point at a valid boundary");
    let wide = index
        .to_wide(WideEncoding::Utf16, utf8)
        .expect("offset should map to a UTF-16 position");

    Position::new(wide.line, wide.col)
}

fn init_test_tracing(test_name: &str) {
    if TEST_TRACING_INITIALIZED.get().is_some() || !test_observability_requested() {
        return;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = LogFormat::from_env();
    let output = output_writer_for_tests(test_name);

    let _ = install_test_subscriber(filter, format, output);
    let _ = TEST_TRACING_INITIALIZED.set(());
}

fn test_observability_requested() -> bool {
    env::var_os(EnvFilter::DEFAULT_ENV).is_some()
        || env::var_os(LOG_FORMAT_ENV).is_some()
        || env::var_os(LOG_FILE_ENV).is_some()
        || env::var_os(TEST_LOG_ENV).is_some()
}

fn output_writer_for_tests(test_name: &str) -> OutputWriter {
    if let Some(path) = env::var_os(LOG_FILE_ENV) {
        return file_output_writer(Path::new(&path));
    }

    if env::var_os(TEST_LOG_ENV).is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp")
            .join(format!("{test_name}.log"));
        return file_output_writer(&path);
    }

    OutputWriter {
        make_writer: BoxMakeWriter::new(io::stderr),
        supports_ansi: true,
    }
}

fn file_output_writer(path: &Path) -> OutputWriter {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).expect("test log directory should be creatable");
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .expect("test log file should be creatable");
    let shared_writer = SharedFileWriter {
        file: Arc::new(Mutex::new(file)),
    };

    OutputWriter {
        make_writer: BoxMakeWriter::new(move || shared_writer.clone()),
        supports_ansi: false,
    }
}

fn install_test_subscriber(
    filter: EnvFilter,
    format: LogFormat,
    output: OutputWriter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let OutputWriter {
        make_writer,
        supports_ansi,
    } = output;
    let base_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_ansi(supports_ansi)
        .with_writer(make_writer);

    match format {
        LogFormat::Compact => tracing_subscriber::registry()
            .with(filter)
            .with(base_layer.compact())
            .try_init()
            .map_err(Into::into),
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(base_layer.json())
            .try_init()
            .map_err(Into::into),
    }
}
