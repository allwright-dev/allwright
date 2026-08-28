from __future__ import annotations

import io
import json
import os
import platform
import shutil
import socket
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.request
import zipfile
from pathlib import Path

import grpc

from ._proto import DEFAULT_SERVER_ADDR, engine_pb2, engine_pb2_grpc
from ._types import AllwrightError

ALLWRIGHT_AUTO_INSTALL_ENV_VAR = "ALLWRIGHT_AUTO_INSTALL"
ALLWRIGHT_CLI_PATH_ENV_VAR = "ALLWRIGHT_CLI_PATH"
ALLWRIGHT_HOME_ENV_VAR = "ALLWRIGHT_HOME"
ALLWRIGHT_REPOSITORY_ENV_VAR = "ALLWRIGHT_REPOSITORY"
ALLWRIGHT_VERSION_ENV_VAR = "ALLWRIGHT_VERSION"
DEFAULT_RELEASE_REPOSITORY = "allwright-dev/allwright"
DEFAULT_RELEASE_VERSION = "0.0.41"

_bootstrap_lock = threading.Lock()
_managed_server: subprocess.Popen[bytes] | None = None
_managed_server_addr: str | None = None
_managed_server_base_addr: str | None = None


def ensure_runtime_ready(server_addr: str) -> str:
    normalized = (server_addr or DEFAULT_SERVER_ADDR).strip()
    expected_version = expected_runtime_version()
    status = ping_server(normalized)
    if status is not None:
        if status["version"] == expected_version:
            return normalized
        if not is_local_server_addr(normalized):
            raise AllwrightError(
                f"allwright server at {normalized} is running version "
                f"{display_version(status['version'])} but this client expects {expected_version}"
            )
    if not is_local_server_addr(normalized):
        raise AllwrightError(
            f"allwright could not reach engine server at {normalized}. "
            "Automatic startup is only supported for local addresses."
        )

    global _managed_server, _managed_server_addr, _managed_server_base_addr
    with _bootstrap_lock:
        if _managed_server is not None and _managed_server.poll() is None:
            if _managed_server_base_addr == normalized and _managed_server_addr is not None:
                return wait_for_server(_managed_server_addr, expected_version)
            _managed_server.kill()
            _managed_server.wait(timeout=5)
        else:
            _managed_server = None
            _managed_server_addr = None
            _managed_server_base_addr = None

        cli_path = ensure_cli_available(expected_version)
        ensure_web_plugin(cli_path, expected_version)
        resolved_server_addr = normalized
        if status is not None and status["version"] != expected_version:
            resolved_server_addr = allocate_managed_server_addr(normalized)
        _managed_server = subprocess.Popen(
            [str(cli_path), "serve", "--listen-addr", cli_listen_addr(resolved_server_addr)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        _managed_server_addr = resolved_server_addr
        _managed_server_base_addr = normalized

    return wait_for_server(resolved_server_addr, expected_version)


def shutdown_managed_server() -> None:
    global _managed_server, _managed_server_addr, _managed_server_base_addr
    with _bootstrap_lock:
        if _managed_server is not None and _managed_server.poll() is None:
            _managed_server.kill()
            _managed_server.wait(timeout=5)
        _managed_server = None
        _managed_server_addr = None
        _managed_server_base_addr = None


def wait_for_server(server_addr: str, expected_version: str) -> str:
    deadline = time.time() + 20
    while time.time() < deadline:
        status = ping_server(server_addr)
        if status is not None and status["version"] == expected_version:
            return server_addr
        time.sleep(0.25)
    shutdown_managed_server()
    raise AllwrightError(
        f"timed out waiting for allwright server at {server_addr} "
        f"to become ready with version {expected_version}"
    )


def ping_server(server_addr: str) -> dict[str, str] | None:
    channel = grpc.insecure_channel(server_addr)
    stub = engine_pb2_grpc.EngineServiceStub(channel)
    try:
        response = stub.Ping(engine_pb2.PingRequest(), timeout=1)
        return {"version": normalize_release_version(getattr(response, "version", ""))}
    except grpc.RpcError:
        return None
    finally:
        channel.close()


def ensure_cli_available(expected_version: str) -> Path:
    env_path = os.getenv(ALLWRIGHT_CLI_PATH_ENV_VAR, "").strip()
    if env_path:
        path = Path(env_path)
        if path.is_file() and cli_version_matches(path, expected_version):
            return path

    bundled = allwright_home() / "bin" / cli_filename()
    if bundled.is_file() and cli_version_matches(bundled, expected_version):
        return bundled

    repo_local = repo_local_cli_path()
    if repo_local is not None and repo_local.is_file() and cli_version_matches(repo_local, expected_version):
        return repo_local

    found = shutil.which(cli_filename())
    if found and cli_version_matches(Path(found), expected_version):
        return Path(found)

    if not auto_install_enabled():
        raise AllwrightError(
            "allwright CLI was not found. Install it first or set ALLWRIGHT_CLI_PATH."
        )
    return install_cli()


def install_cli() -> Path:
    install_dir = allwright_home() / "bin"
    install_dir.mkdir(parents=True, exist_ok=True)
    cli_path = install_dir / cli_filename()
    version_tag = resolve_release_tag()
    asset_name = cli_asset_name(version_tag)
    asset_bytes = download_release_asset(version_tag, asset_name)
    unpack_cli_archive(asset_name, asset_bytes, cli_path)
    cli_path.chmod(0o755)
    return cli_path


def ensure_web_plugin(cli_path: Path, expected_version: str) -> None:
    plugin_path = allwright_home() / "plugins" / "web" / "lib" / web_plugin_filename()
    if plugin_path.is_file() and installed_plugin_version("web") == expected_version:
        return

    completed = subprocess.run(
        [str(cli_path), "plugin", "install", "web", "--version", expected_version],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0 or not plugin_path.is_file():
        raise AllwrightError(
            "allwright attempted to install the `web` plugin automatically, "
            "but the install did not complete successfully"
        )
    if installed_plugin_version("web") != expected_version:
        raise AllwrightError(
            "allwright attempted to install the `web` plugin automatically, "
            f"but version {expected_version} is still not active"
        )


def resolve_release_tag() -> str:
    version = os.getenv(ALLWRIGHT_VERSION_ENV_VAR, "").strip() or DEFAULT_RELEASE_VERSION
    if version == "latest":
        repository = os.getenv(ALLWRIGHT_REPOSITORY_ENV_VAR, "").strip() or DEFAULT_RELEASE_REPOSITORY
        with urllib.request.urlopen(
            urllib.request.Request(
                f"https://api.github.com/repos/{repository}/releases/latest",
                headers={"User-Agent": f"allwright-python/{DEFAULT_RELEASE_VERSION}"},
            )
        ) as response:
            payload = json.load(response)
        tag = payload.get("tag_name")
        if not isinstance(tag, str) or not tag.strip():
            raise AllwrightError("latest allwright release metadata did not include tag_name")
        return tag
    return normalize_release_tag(version)


def cli_asset_name(version_tag: str) -> str:
    system = platform.system()
    machine = platform.machine()
    normalized_machine = {
        "amd64": "x86_64",
        "arm64": "aarch64",
    }.get(machine.lower(), machine)
    platform_key = (system, normalized_machine)
    targets = {
        ("Darwin", "arm64"): "aarch64-apple-darwin",
        ("Darwin", "aarch64"): "aarch64-apple-darwin",
        ("Darwin", "x86_64"): "x86_64-apple-darwin",
        ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
        ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
    }
    target = targets.get(platform_key)
    if target is None and os.name == "nt":
        if normalized_machine == "aarch64":
            target = "aarch64-pc-windows-msvc"
        else:
            target = "x86_64-pc-windows-msvc"
    if target is None:
        raise AllwrightError(
            f"automatic allwright CLI install is not supported on {platform_key[0]}/{platform_key[1]}"
        )
    extension = "zip" if os.name == "nt" else "tar.gz"
    return f"allwright-{version_tag}-{target}.{extension}"


def download_release_asset(version_tag: str, asset_name: str) -> bytes:
    repository = os.getenv(ALLWRIGHT_REPOSITORY_ENV_VAR, "").strip() or DEFAULT_RELEASE_REPOSITORY
    request = urllib.request.Request(
        f"https://github.com/{repository}/releases/download/{version_tag}/{asset_name}",
        headers={"User-Agent": f"allwright-python/{DEFAULT_RELEASE_VERSION}"},
    )
    with urllib.request.urlopen(request) as response:
        return response.read()


def unpack_cli_archive(asset_name: str, asset_bytes: bytes, destination: Path) -> None:
    if asset_name.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(asset_bytes)) as archive:
            for member_name in archive.namelist():
                if normalize_archive_path(member_name) != f"bin/{cli_filename()}":
                    continue
                with archive.open(member_name) as source, destination.open("wb") as target:
                    shutil.copyfileobj(source, target)
                return
        raise AllwrightError("allwright CLI archive did not contain the expected binary")
        return

    with tarfile.open(fileobj=io.BytesIO(asset_bytes), mode="r:gz") as archive:
        for member in archive.getmembers():
            if normalize_archive_path(member.name) != f"bin/{cli_filename()}":
                continue
            source = archive.extractfile(member)
            if source is None:
                break
            with source, destination.open("wb") as target:
                shutil.copyfileobj(source, target)
            return
    raise AllwrightError("allwright CLI archive did not contain the expected binary")


def normalize_archive_path(name: str) -> str | None:
    parts: list[str] = []
    for part in Path(name).parts:
        if part in {"", "."}:
            continue
        if part == "..":
            return None
        parts.append(part)
    if not parts:
        return None
    return "/".join(parts)


def allwright_home() -> Path:
    configured = os.getenv(ALLWRIGHT_HOME_ENV_VAR, "").strip()
    if configured:
        return Path(configured)
    return Path.home() / ".allwright"


def auto_install_enabled() -> bool:
    return os.getenv(ALLWRIGHT_AUTO_INSTALL_ENV_VAR, "").strip().lower() not in {
        "0",
        "false",
        "no",
    }


def cli_listen_addr(server_addr: str) -> str:
    if server_addr.startswith("http://"):
        return server_addr[len("http://") :]
    if server_addr.startswith("https://"):
        return server_addr[len("https://") :]
    return server_addr


def normalize_release_tag(version: str) -> str:
    version = version.strip()
    return version if version.startswith("v") else f"v{version}"


def normalize_release_version(version: str) -> str:
    return version.strip().removeprefix("v")


def expected_runtime_version() -> str:
    return normalize_release_version(
        os.getenv(ALLWRIGHT_VERSION_ENV_VAR, "").strip() or DEFAULT_RELEASE_VERSION
    )


def is_local_server_addr(server_addr: str) -> bool:
    host = cli_listen_addr(server_addr).rsplit(":", 1)[0].strip("[]")
    return host in {"127.0.0.1", "localhost", "::1"}


def installed_plugin_version(plugin_id: str) -> str | None:
    manifest = allwright_home() / "plugins.txt"
    if not manifest.is_file():
        return None
    for line in manifest.read_text().splitlines():
        trimmed = line.strip()
        if not trimmed or trimmed.startswith("#"):
            continue
        parts = trimmed.split("\t", 2)
        if len(parts) < 3 or parts[0] != plugin_id:
            continue
        return normalize_release_version(parts[2])
    return None


def allocate_managed_server_addr(server_addr: str) -> str:
    host = local_binding_host(server_addr)
    with socket.create_server((host, 0), family=socket.AF_INET6 if ":" in host else socket.AF_INET) as listener:
        port = listener.getsockname()[1]
    if ":" in host:
        return f"[{host}]:{port}"
    return f"{host}:{port}"


def local_binding_host(server_addr: str) -> str:
    host = cli_listen_addr(server_addr).rsplit(":", 1)[0].strip("[]")
    return "::1" if host == "::1" else "127.0.0.1"


def display_version(version: str) -> str:
    return version or "unknown"


def repo_local_cli_path() -> Path | None:
    repo_root = Path(__file__).resolve().parents[2]
    for candidate in (
        repo_root / "target" / "debug" / cli_filename(),
        repo_root / "target" / "release" / cli_filename(),
    ):
        if candidate.is_file():
            return candidate
    return None


def cli_version_matches(cli_path: Path, expected_version: str) -> bool:
    try:
        completed = subprocess.run(
            [str(cli_path), "--version"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            check=False,
        )
    except OSError as error:
        raise AllwrightError(
            f"inspect allwright CLI version via {cli_path}: {error}"
        ) from error
    if completed.returncode != 0:
        return False
    for token in completed.stdout.split():
        if token and token[0].isdigit():
            return normalize_release_version(token) == expected_version
    return False


def cli_filename() -> str:
    return "allwright.exe" if os.name == "nt" else "allwright"


def web_plugin_filename() -> str:
    if os.name == "nt":
        return "allwright_surface_web.dll"
    if os.uname().sysname == "Darwin":
        return "liballwright_surface_web.dylib"
    return "liballwright_surface_web.so"
