#!/usr/bin/env -S uv run --script
# /// script
# requires-python = "==3.14.*"
# dependencies = []
# ///

from __future__ import annotations

import argparse
import json
import signal
import subprocess
import sys
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, IO


@dataclass(frozen=True)
class ReplayCase:
    document_path: Path
    workspace_roots: tuple[Path, ...]


REPLAY_TIMEOUT_SECONDS = 30.0
PROCESS_EXIT_TIMEOUT_SECONDS = 10.0


def repository_root() -> Path:
    return Path(__file__).resolve().parent.parent


def replay_cases(root: Path) -> dict[str, ReplayCase]:
    return {
        "small": ReplayCase(
            document_path=root
            / "crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl",
            workspace_roots=(),
        ),
        "large": ReplayCase(
            document_path=root
            / "tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl",
            workspace_roots=(root / "tests/lsp/workspaces/big-bank-plc",),
        ),
        "mega": ReplayCase(
            document_path=root / "tests/lsp/workspaces/benchmark-mega/global-views.dsl",
            workspace_roots=(root / "tests/lsp/workspaces/benchmark-mega",),
        ),
        "mega-multi-root": ReplayCase(
            document_path=root
            / "tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/model.dsl",
            workspace_roots=tuple(
                root / f"tests/lsp/workspaces/benchmark-mega-multi-root/ws-{index:02d}"
                for index in range(24)
            ),
        ),
    }


def encode_message(payload: dict[str, Any]) -> bytes:
    body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
    return header + body


def send_message(stream: IO[bytes], payload: dict[str, Any]) -> None:
    stream.write(encode_message(payload))
    stream.flush()


@contextmanager
def replay_deadline(seconds: float) -> Any:
    def on_timeout(_signum: int, _frame: Any) -> None:
        raise TimeoutError("language server replay exceeded the timeout")

    previous_handler = signal.signal(signal.SIGALRM, on_timeout)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous_handler)


def read_message(stream: IO[bytes]) -> dict[str, Any]:
    headers: dict[str, str] = {}

    while True:
        line = stream.readline()
        if line == b"":
            raise EOFError("language server closed stdout unexpectedly")
        if line in (b"\r\n", b"\n"):
            break

        header_name, header_value = line.decode("ascii").split(":", 1)
        headers[header_name.lower()] = header_value.strip()

    content_length = int(headers["content-length"])
    body = stream.read(content_length)
    if len(body) != content_length:
        raise EOFError(
            "language server closed stdout before the message body completed"
        )

    return json.loads(body)


def expect_response(stream: IO[bytes], request_id: int) -> dict[str, Any]:
    while True:
        message = read_message(stream)
        if message.get("id") != request_id:
            continue
        if "error" in message:
            raise RuntimeError(
                f"language server returned an error for request {request_id}: {message['error']}"
            )
        return message


def expect_notification(stream: IO[bytes], method: str) -> dict[str, Any]:
    while True:
        message = read_message(stream)
        if message.get("method") == method:
            return message


def run_replay(server_binary: Path, case: ReplayCase) -> None:
    document_path = case.document_path.resolve()
    source = document_path.read_text(encoding="utf-8")
    document_uri = document_path.as_uri()
    workspace_folders = [
        {"uri": workspace_root.resolve().as_uri(), "name": workspace_root.name}
        for workspace_root in case.workspace_roots
    ]

    server = subprocess.Popen(
        [str(server_binary), "server"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    if server.stdin is None or server.stdout is None or server.stderr is None:
        raise RuntimeError("failed to open stdio pipes for the language server process")

    try:
        with replay_deadline(REPLAY_TIMEOUT_SECONDS):
            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "capabilities": {},
                        "workspaceFolders": workspace_folders,
                    },
                },
            )
            expect_response(server.stdout, 1)

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "method": "initialized",
                    "params": {},
                },
            )

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": document_uri,
                            "languageId": "Structurizr DSL",
                            "version": 1,
                            "text": source,
                        }
                    },
                },
            )
            expect_notification(server.stdout, "textDocument/publishDiagnostics")

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": {
                            "uri": document_uri,
                            "version": 2,
                        },
                        "contentChanges": [
                            {
                                "text": f"{source}\n",
                            }
                        ],
                    },
                },
            )
            expect_notification(server.stdout, "textDocument/publishDiagnostics")

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "textDocument/documentSymbol",
                    "params": {
                        "textDocument": {
                            "uri": document_uri,
                        }
                    },
                },
            )
            expect_response(server.stdout, 2)

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "shutdown",
                },
            )
            expect_response(server.stdout, 3)

            send_message(
                server.stdin,
                {
                    "jsonrpc": "2.0",
                    "method": "exit",
                },
            )
            server.stdin.close()

            return_code = server.wait(timeout=PROCESS_EXIT_TIMEOUT_SECONDS)
            stderr_output = server.stderr.read().decode("utf-8", errors="replace")
            if return_code != 0:
                raise RuntimeError(
                    f"language server exited with {return_code}: {stderr_output}"
                )
    finally:
        if server.stdin is not None and not server.stdin.closed:
            server.stdin.close()
        if server.poll() is None:
            server.terminate()
            try:
                server.wait(timeout=PROCESS_EXIT_TIMEOUT_SECONDS)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=PROCESS_EXIT_TIMEOUT_SECONDS)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Replay a small canned LSP session against `strz server`.",
    )
    parser.add_argument(
        "--server",
        type=Path,
        default=Path("target/release/strz"),
        help="Path to the built `strz` binary",
    )
    parser.add_argument(
        "--case",
        choices=("small", "large", "mega", "mega-multi-root"),
        default="small",
        help="Which checked-in session case to replay",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    root = repository_root()
    cases = replay_cases(root)
    server_path = arguments.server
    if not server_path.is_absolute():
        server_path = (root / server_path).resolve()

    run_replay(server_path, cases[arguments.case])
    return 0


if __name__ == "__main__":
    sys.exit(main())
