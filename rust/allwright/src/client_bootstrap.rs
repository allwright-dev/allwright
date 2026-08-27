use std::env;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use tar::Archive;
use tonic::transport::Endpoint;
use zip::ZipArchive;

use crate::proto::{PingRequest, engine_service_client::EngineServiceClient};

use super::types::{Error, Result};

const ALLWRIGHT_AUTO_INSTALL_ENV_VAR: &str = "ALLWRIGHT_AUTO_INSTALL";
const ALLWRIGHT_CLI_PATH_ENV_VAR: &str = "ALLWRIGHT_CLI_PATH";
const ALLWRIGHT_HOME_ENV_VAR: &str = "ALLWRIGHT_HOME";
const ALLWRIGHT_REPOSITORY_ENV_VAR: &str = "ALLWRIGHT_REPOSITORY";
const ALLWRIGHT_VERSION_ENV_VAR: &str = "ALLWRIGHT_VERSION";
const DEFAULT_RELEASE_REPOSITORY: &str = "allwright-dev/allwright";
const DEFAULT_RELEASE_VERSION: &str = env!("CARGO_PKG_VERSION");
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const PING_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Default)]
struct BootstrapState {
    managed_server_addr: Option<String>,
    managed_server: Option<Child>,
}

static BOOTSTRAP_STATE: OnceLock<Mutex<BootstrapState>> = OnceLock::new();

pub(crate) async fn ensure_runtime_ready(server_addr: &str) -> Result<()> {
    if ping_server(server_addr).await.is_ok() {
        return Ok(());
    }

    if !is_local_server_addr(server_addr) {
        return Err(Error::new(format!(
            "allwright could not reach engine server at {server_addr}. Automatic startup is only supported for local addresses."
        )));
    }

    let mut already_managed = false;
    {
        let mut state = bootstrap_state()
            .lock()
            .map_err(|_| Error::new("bootstrap state lock is poisoned"))?;

        if let Some(child) = state.managed_server.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    state.managed_server = None;
                    state.managed_server_addr = None;
                }
                Ok(None) => {
                    if state.managed_server_addr.as_deref() == Some(server_addr) {
                        already_managed = true;
                    } else {
                        let mut child = state.managed_server.take().expect("managed server child");
                        let _ = child.kill();
                        let _ = child.wait();
                        state.managed_server_addr = None;
                    }
                }
                Err(error) => {
                    return Err(Error::new(format!(
                        "failed to inspect managed allwright server process: {error}"
                    )));
                }
            }
        }
    }

    if already_managed {
        return wait_for_server(server_addr).await;
    }

    {
        let cli_path = ensure_cli_available()?;
        ensure_web_plugin(&cli_path)?;
        let listen_addr = cli_listen_addr(server_addr);
        let child = Command::new(&cli_path)
            .arg("serve")
            .arg("--listen-addr")
            .arg(&listen_addr)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                Error::new(format!(
                    "failed to start allwright server with {}: {error}",
                    cli_path.display()
                ))
            })?;

        let mut state = bootstrap_state()
            .lock()
            .map_err(|_| Error::new("bootstrap state lock is poisoned"))?;
        state.managed_server = Some(child);
        state.managed_server_addr = Some(server_addr.to_string());
    }

    wait_for_server(server_addr).await
}

pub(crate) fn shutdown_managed_server() -> Result<()> {
    let mut state = bootstrap_state()
        .lock()
        .map_err(|_| Error::new("bootstrap state lock is poisoned"))?;

    if let Some(mut child) = state.managed_server.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    state.managed_server_addr = None;
    Ok(())
}

fn bootstrap_state() -> &'static Mutex<BootstrapState> {
    BOOTSTRAP_STATE.get_or_init(|| Mutex::new(BootstrapState::default()))
}

async fn wait_for_server(server_addr: &str) -> Result<()> {
    let start = Instant::now();
    loop {
        if ping_server(server_addr).await.is_ok() {
            return Ok(());
        }
        if start.elapsed() >= STARTUP_TIMEOUT {
            let _ = shutdown_managed_server();
            return Err(Error::new(format!(
                "timed out waiting for allwright server at {server_addr} to become ready"
            )));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn ping_server(server_addr: &str) -> Result<()> {
    let endpoint = Endpoint::from_shared(server_addr.to_string())
        .map_err(|error| Error::new(format!("invalid allwright server address {server_addr}: {error}")))?;
    let channel = tokio::time::timeout(PING_TIMEOUT, endpoint.connect())
        .await
        .map_err(|_| Error::new(format!("timed out connecting to allwright server at {server_addr}")))?
        .map_err(Error::from)?;
    let mut engine = EngineServiceClient::new(channel);
    tokio::time::timeout(PING_TIMEOUT, engine.ping(tonic::Request::new(PingRequest {})))
        .await
        .map_err(|_| Error::new(format!("timed out pinging allwright server at {server_addr}")))?
        .map_err(Error::from)?;
    Ok(())
}

fn ensure_cli_available() -> Result<PathBuf> {
    if let Some(cli_path) = resolve_existing_cli_path()? {
        return Ok(cli_path);
    }
    if !auto_install_enabled() {
        return Err(Error::new(
            "allwright CLI was not found. Install it first or set ALLWRIGHT_CLI_PATH.",
        ));
    }
    install_cli()
}

fn resolve_existing_cli_path() -> Result<Option<PathBuf>> {
    if let Ok(raw) = env::var(ALLWRIGHT_CLI_PATH_ENV_VAR) {
        let path = PathBuf::from(raw.trim());
        if is_executable_file(&path) {
            return Ok(Some(path));
        }
    }

    let bundled = allwright_home()?.join("bin").join(cli_filename());
    if is_executable_file(&bundled) {
        return Ok(Some(bundled));
    }

    Ok(find_in_path(cli_filename()))
}

fn install_cli() -> Result<PathBuf> {
    let install_dir = allwright_home()?.join("bin");
    fs::create_dir_all(&install_dir).map_err(|error| {
        Error::new(format!(
            "failed to create allwright CLI install directory {}: {error}",
            install_dir.display()
        ))
    })?;

    let cli_path = install_dir.join(cli_filename());
    let version_tag = resolve_release_tag()?;
    let asset_name = cli_asset_name(&version_tag)?;
    let asset_bytes = download_release_asset(&version_tag, &asset_name)?;
    unpack_cli_archive(&asset_name, &asset_bytes, &cli_path)?;

    if !is_executable_file(&cli_path) {
        return Err(Error::new(format!(
            "downloaded allwright CLI archive {asset_name} but did not produce {}",
            cli_path.display()
        )));
    }

    Ok(cli_path)
}

fn ensure_web_plugin(cli_path: &Path) -> Result<()> {
    let plugin_path = allwright_home()?
        .join("plugins")
        .join("web")
        .join("lib")
        .join(web_plugin_filename());
    if plugin_path.exists() {
        return Ok(());
    }

    let version = env::var(ALLWRIGHT_VERSION_ENV_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASE_VERSION.to_string());
    let status = Command::new(cli_path)
        .arg("plugin")
        .arg("install")
        .arg("web")
        .arg("--version")
        .arg(normalize_release_version(&version))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| {
            Error::new(format!(
                "failed to install the allwright `web` plugin with {}: {error}",
                cli_path.display()
            ))
        })?;

    if !status.success() || !plugin_path.exists() {
        return Err(Error::new(
            "allwright attempted to install the `web` plugin automatically, but the install did not complete successfully",
        ));
    }

    Ok(())
}

fn resolve_release_tag() -> Result<String> {
    let version = env::var(ALLWRIGHT_VERSION_ENV_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASE_VERSION.to_string());
    if version.trim() == "latest" {
        return fetch_latest_release_tag();
    }
    Ok(normalize_release_tag(&version))
}

fn fetch_latest_release_tag() -> Result<String> {
    let repository = env::var(ALLWRIGHT_REPOSITORY_ENV_VAR)
        .unwrap_or_else(|_| DEFAULT_RELEASE_REPOSITORY.to_string());
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let response = release_client()?
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| Error::new(format!("failed to resolve latest allwright release: {error}")))?;
    let payload: Value = serde_json::from_reader(response)
        .map_err(|error| Error::new(format!("failed to decode latest allwright release metadata: {error}")))?;
    let tag = payload
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new("latest allwright release metadata did not include tag_name"))?;
    Ok(tag.to_string())
}

fn cli_asset_name(version_tag: &str) -> Result<String> {
    let target = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => {
            return Err(Error::new(format!(
                "automatic allwright CLI install is not supported on os={os}, arch={arch}"
            )))
        }
    };
    let extension = if env::consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };
    Ok(format!("allwright-{version_tag}-{target}.{extension}"))
}

fn download_release_asset(version_tag: &str, asset_name: &str) -> Result<Vec<u8>> {
    let repository = env::var(ALLWRIGHT_REPOSITORY_ENV_VAR)
        .unwrap_or_else(|_| DEFAULT_RELEASE_REPOSITORY.to_string());
    let url = format!("https://github.com/{repository}/releases/download/{version_tag}/{asset_name}");
    let mut response = release_client()?
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| Error::new(format!("failed to download allwright CLI asset {asset_name}: {error}")))?;
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|error| Error::new(format!("failed to read allwright CLI asset {asset_name}: {error}")))?;
    Ok(bytes)
}

fn unpack_cli_archive(asset_name: &str, asset_bytes: &[u8], destination: &Path) -> Result<()> {
    if asset_name.ends_with(".tar.gz") {
        let decoder = GzDecoder::new(Cursor::new(asset_bytes));
        let mut archive = Archive::new(decoder);
        for entry in archive.entries().map_err(|error| Error::new(format!("failed to read CLI archive entries: {error}")))? {
            let mut entry = entry.map_err(|error| Error::new(format!("failed to open CLI archive entry: {error}")))?;
            let entry_path = entry
                .path()
                .map_err(|error| Error::new(format!("failed to read CLI archive entry path: {error}")))?;
            if entry_path == Path::new("bin").join(cli_filename()) {
                entry.unpack(destination).map_err(|error| {
                    Error::new(format!(
                        "failed to unpack the allwright CLI into {}: {error}",
                        destination.display()
                    ))
                })?;
                set_executable(destination)?;
                return Ok(());
            }
        }
        return Err(Error::new("allwright CLI archive did not contain bin/allwright"));
    }

    let mut archive =
        ZipArchive::new(Cursor::new(asset_bytes)).map_err(|error| Error::new(format!("failed to open CLI zip archive: {error}")))?;
    let mut file = archive
        .by_name(&format!("bin/{}", cli_filename()))
        .map_err(|error| Error::new(format!("failed to find the allwright CLI in the downloaded zip archive: {error}")))?;
    let mut output = fs::File::create(destination)
        .map_err(|error| Error::new(format!("failed to create {}: {error}", destination.display())))?;
    std::io::copy(&mut file, &mut output)
        .map_err(|error| Error::new(format!("failed to extract the allwright CLI: {error}")))?;
    set_executable(destination)?;
    Ok(())
}

fn release_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(format!("allwright-core/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| Error::new(format!("failed to build allwright release client: {error}")))
}

fn cli_listen_addr(server_addr: &str) -> String {
    server_addr
        .strip_prefix("http://")
        .or_else(|| server_addr.strip_prefix("https://"))
        .unwrap_or(server_addr)
        .to_string()
}

fn normalize_release_tag(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('v') {
        trimmed.to_string()
    } else {
        format!("v{trimmed}")
    }
}

fn normalize_release_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').to_string()
}

fn allwright_home() -> Result<PathBuf> {
    if let Ok(home) = env::var(ALLWRIGHT_HOME_ENV_VAR) {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let home =
        env::var("HOME").map_err(|_| Error::new("HOME is not set and ALLWRIGHT_HOME was not provided"))?;
    Ok(PathBuf::from(home).join(".allwright"))
}

fn auto_install_enabled() -> bool {
    env::var(ALLWRIGHT_AUTO_INSTALL_ENV_VAR)
        .map(|value| !matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true)
}

fn find_in_path(filename: &str) -> Option<PathBuf> {
    let path_value = env::var_os("PATH")?;
    for entry in env::split_paths(&path_value) {
        let candidate = entry.join(filename);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_local_server_addr(server_addr: &str) -> bool {
    let normalized = server_addr
        .strip_prefix("http://")
        .or_else(|| server_addr.strip_prefix("https://"))
        .unwrap_or(server_addr);
    let without_auth = normalized
        .rsplit_once('@')
        .map(|(_, tail)| tail)
        .unwrap_or(normalized);
    let host = without_auth
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(without_auth)
        .trim_matches(['[', ']']);
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn cli_filename() -> &'static str {
    if env::consts::OS == "windows" {
        "allwright.exe"
    } else {
        "allwright"
    }
}

fn web_plugin_filename() -> &'static str {
    match env::consts::OS {
        "macos" => "liballwright_surface_web.dylib",
        "linux" => "liballwright_surface_web.so",
        "windows" => "allwright_surface_web.dll",
        _ => "allwright_surface_web.unknown",
    }
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| Error::new(format!("failed to inspect {}: {error}", path.display())))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| Error::new(format!("failed to mark {} executable: {error}", path.display())))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}
