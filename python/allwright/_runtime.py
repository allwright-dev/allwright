from __future__ import annotations

import os
import threading
from typing import Any

from ._bootstrap import ensure_runtime_ready, shutdown_managed_server
from ._browser import Browser
from ._page import Page
from ._proto import DEFAULT_SERVER_ADDR, SERVER_ADDR_ENV_VAR, engine_pb2, engine_pb2_grpc
from ._transport import RuntimeClient, StreamHandle
from ._types import AllwrightError, LaunchOptions

_runtime_lock = threading.Lock()
_runtime_client: RuntimeClient | None = None
_server_addr_override: str | None = None


def ping() -> str:
    runtime = get_runtime()
    response = runtime.stub.Ping(engine_pb2.PingRequest())
    return response.message


def launch_browser_with_kind(browser_kind: int, options: LaunchOptions | None = None) -> Browser:
    runtime = get_runtime()
    stream = StreamHandle(runtime.stub.BrowserSession)
    launch_options = options or LaunchOptions()

    command_kwargs: dict[str, Any] = {}
    if launch_options.browser_binary:
        command_kwargs["browser_binary"] = launch_options.browser_binary
    if launch_options.timeout_ms is not None:
        command_kwargs["retry_options"] = retry_options(launch_options.timeout_ms)

    stream.send(
        engine_pb2.BrowserSessionCommand(
            launch_browser=engine_pb2.LaunchBrowserCommand(
                browser_kind=browser_kind,
                **command_kwargs,
            ),
        )
    )

    while True:
        event = stream.recv("receive browser session event during launch")
        match event.WhichOneof("event"):
            case "browser_launched":
                launched = event.browser_launched
                initial_page = Page(
                    runtime=runtime,
                    browser_session_id=event.session_id,
                    session_id=launched.initial_tab_session_id,
                )
                return Browser(
                    runtime=runtime,
                    stream=stream,
                    session_id=event.session_id,
                    browser_name=launched.browser,
                    launch_note=launched.note,
                    cdp_websocket_url="",
                    user_data_dir=launched.user_data_dir,
                    initial_page=initial_page,
                )
            case "error":
                raise AllwrightError(
                    f"browser session error during launch: {event.error.message}"
                )


def set_server_addr(server_addr: str) -> None:
    global _runtime_client, _server_addr_override
    normalized = server_addr.strip()
    with _runtime_lock:
        _server_addr_override = normalized
        if _runtime_client is not None:
            _runtime_client.close()
            _runtime_client = None
        shutdown_managed_server()


def shutdown() -> None:
    global _runtime_client
    with _runtime_lock:
        if _runtime_client is not None:
            _runtime_client.close()
            _runtime_client = None
        shutdown_managed_server()


def get_runtime() -> RuntimeClient:
    global _runtime_client
    with _runtime_lock:
        if _runtime_client is None:
            server_addr = resolve_server_addr()
            resolved_server_addr = ensure_runtime_ready(server_addr)
            _runtime_client = RuntimeClient(resolved_server_addr, engine_pb2_grpc.EngineServiceStub)
        return _runtime_client


def resolve_server_addr() -> str:
    if _server_addr_override:
        return _server_addr_override

    value = os.getenv(SERVER_ADDR_ENV_VAR, "").strip()
    if value:
        return value
    return DEFAULT_SERVER_ADDR


def retry_options(timeout_ms: int | None) -> Any | None:
    if timeout_ms is None or timeout_ms <= 0:
        return None
    return engine_pb2.CommandRetryOptions(timeout_ms=timeout_ms)
