use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AccessibilitySnapshotInfo, SurfaceFamily, SurfacePlugin,
    SurfacePluginDescriptor,
};
use allwright_surface_desktop::{
    ConnectOptions, DesktopActionInfo, DesktopAppInfo, DesktopAppSessionHandle, DesktopCommand,
    DesktopCommandResult, DesktopConnectInfo, DesktopElementCountInfo, DesktopPlatform,
    DesktopScreenshotInfo, DesktopSessionHandle, DesktopTextInfo, DesktopWaitInfo, LaunchOptions,
    boot_surface, normalize_selector_for_transport,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_AGENT_ENDPOINT: &str = "http://127.0.0.1:8200";
const AGENT_ENTITLEMENTS_FILENAME: &str = "AllwrightAgentUITests.mac-runtime.entitlements";
static AGENT_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopMacPlugin;

impl SurfacePlugin for DesktopMacPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        descriptor()
    }
}

pub fn descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: "desktop-mac",
        family: SurfaceFamily::Desktop,
        version: env!("CARGO_PKG_VERSION"),
        description: "macOS desktop surface plugin backed by the Allwright XCUITest agent.",
    }
}

#[derive(Debug, Clone, Serialize)]
struct AgentRequest<'a> {
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminate_running: Option<bool>,
}

impl<'a> AgentRequest<'a> {
    fn new(command: &'a str) -> Self {
        Self {
            command,
            session_id: None,
            bundle_id: None,
            selector: None,
            value: None,
            key: None,
            text: None,
            visible: None,
            timeout_ms: None,
            terminate_running: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AgentEnvelope {
    ok: bool,
    #[serde(default)]
    result: Value,
    error: Option<String>,
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16, String), String> {
    let endpoint = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "macOS agent endpoint must use http://".to_string())?;
    let (authority, base_path) = endpoint.split_once('/').unwrap_or((endpoint, ""));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>()
                .map_err(|_| format!("invalid macOS agent port in `{authority}`"))?,
        ),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err("macOS agent endpoint host cannot be empty".to_string());
    }
    let path = if base_path.is_empty() {
        "/v1/command".to_string()
    } else {
        format!("/{}/v1/command", base_path.trim_matches('/'))
    };
    Ok((host, port, path))
}

fn invoke_agent<T: Serialize>(
    endpoint: &str,
    request: &T,
    timeout_ms: Option<u32>,
) -> Result<Value, String> {
    let (host, port, path) = parse_http_endpoint(endpoint)?;
    let timeout =
        Duration::from_millis(timeout_ms.unwrap_or(10_000).max(1).saturating_add(2_000) as u64);
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|error| format!("could not connect to macOS agent at {endpoint}: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("could not set macOS agent read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("could not set macOS agent write timeout: {error}"))?;
    let body = serde_json::to_vec(request)
        .map_err(|error| format!("failed to encode macOS agent request: {error}"))?;
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|error| format!("failed to write macOS agent request: {error}"))?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read macOS agent response: {error}"))?;
    let response = String::from_utf8(response)
        .map_err(|error| format!("macOS agent returned non-UTF-8 HTTP data: {error}"))?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "macOS agent returned a malformed HTTP response".to_string())?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(format!("macOS agent returned `{status}`: {body}"));
    }
    let envelope: AgentEnvelope = serde_json::from_str(body)
        .map_err(|error| format!("failed to decode macOS agent response: {error}"))?;
    if envelope.ok {
        Ok(envelope.result)
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "macOS agent returned an unknown error".to_string()))
    }
}

fn result_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("macOS agent response is missing `{key}`"))
}

fn plugin_install_root() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("ALLWRIGHT_HOME") {
        return Ok(PathBuf::from(root).join("plugins/desktop-mac"));
    }
    let home = env::var("HOME").map_err(|_| {
        "HOME is not set; set ALLWRIGHT_HOME so desktop-mac can locate its bundled agent"
            .to_string()
    })?;
    Ok(PathBuf::from(home).join(".allwright/plugins/desktop-mac"))
}

fn bundled_agent_xctestrun() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("ALLWRIGHT_MAC_AGENT_XCTESTRUN") {
        let path = PathBuf::from(path);
        return path
            .is_file()
            .then_some(path)
            .ok_or_else(|| "ALLWRIGHT_MAC_AGENT_XCTESTRUN does not point to a file".to_string());
    }
    let agent_root = plugin_install_root()?.join("agent");
    fs::read_dir(&agent_root)
        .map_err(|error| {
            format!(
                "the desktop-mac plugin has no bundled XCUITest agent at {}: {error}; reinstall it with `allwright plugin install desktop-mac`",
                agent_root.display()
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "xctestrun"))
        .ok_or_else(|| {
            format!(
                "the desktop-mac plugin archive is missing an .xctestrun file under {}",
                agent_root.display()
            )
        })
}

fn bundled_agent_entitlements(agent_root: &Path) -> Result<PathBuf, String> {
    let path = env::var("ALLWRIGHT_MAC_AGENT_ENTITLEMENTS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| agent_root.join(AGENT_ENTITLEMENTS_FILENAME));
    path.is_file().then_some(path.clone()).ok_or_else(|| {
        format!(
            "the desktop-mac plugin archive is missing its runtime signing entitlements at {}",
            path.display()
        )
    })
}

fn run_checked(command: &mut Command, description: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("could not {description}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to {description}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn prepare_locally_signed_agent() -> Result<PathBuf, String> {
    let source_xctestrun = bundled_agent_xctestrun()?;
    let source_root = source_xctestrun
        .parent()
        .ok_or_else(|| "the bundled macOS xctestrun has no parent directory".to_string())?;
    let source_runner = source_root.join("Release/AllwrightAgentUITests-Runner.app");
    if !source_runner.is_dir() {
        return Err(format!(
            "the desktop-mac plugin archive is missing its unsigned runner at {}",
            source_runner.display()
        ));
    }
    let entitlements = bundled_agent_entitlements(source_root)?;
    let signed_root = plugin_install_root()?.join("agent-signed");
    if signed_root.exists() {
        fs::remove_dir_all(&signed_root)
            .map_err(|error| format!("failed to refresh {}: {error}", signed_root.display()))?;
    }
    fs::create_dir_all(&signed_root)
        .map_err(|error| format!("failed to create {}: {error}", signed_root.display()))?;
    run_checked(
        Command::new("ditto").arg(source_root).arg(&signed_root),
        "copy the bundled unsigned macOS runner",
    )?;
    let xctestrun_name = source_xctestrun
        .file_name()
        .ok_or_else(|| "the bundled macOS xctestrun has no file name".to_string())?;
    let xctestrun = signed_root.join(xctestrun_name);
    let runner = signed_root.join("Release/AllwrightAgentUITests-Runner.app");
    run_checked(
        Command::new("codesign")
            .args([
                "--force",
                "--deep",
                "--sign",
                "-",
                "--timestamp=none",
                "--entitlements",
            ])
            .arg(&entitlements)
            .arg(&runner),
        "ad-hoc sign the local Allwright macOS XCTest runner and its nested test frameworks",
    )?;
    run_checked(
        Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&runner),
        "verify the locally signed Allwright macOS XCTest runner",
    )?;
    Ok(xctestrun)
}

fn stop_owned_agent() -> Result<(), String> {
    let mut process = AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "macOS agent process lock is poisoned".to_string())?;
    if let Some(mut child) = process.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

fn start_bundled_agent() -> Result<(), String> {
    stop_owned_agent()?;
    let xctestrun = prepare_locally_signed_agent()?;
    let child = Command::new("xcodebuild")
        .arg("test-without-building")
        .arg("-xctestrun")
        .arg(&xctestrun)
        .args([
            "-destination",
            "platform=macOS",
            "-parallel-testing-enabled",
            "NO",
            "-only-testing:AllwrightAgentUITests/AllwrightAgentUITests/testAgent",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            format!(
                "could not launch bundled macOS agent {}: {error}",
                xctestrun.display()
            )
        })?;
    *AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "macOS agent process lock is poisoned".to_string())? = Some(child);
    Ok(())
}

fn connect(options: &ConnectOptions) -> Result<DesktopConnectInfo, String> {
    if options.platform != DesktopPlatform::Mac {
        return Err("desktop-mac only supports the macOS platform".to_string());
    }
    let endpoint = options
        .agent_endpoint
        .as_deref()
        .filter(|endpoint| !endpoint.trim().is_empty())
        .unwrap_or(DEFAULT_AGENT_ENDPOINT)
        .trim_end_matches('/')
        .to_string();
    let mut status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
    let mut started_bundled_agent = false;
    if status.is_err() && endpoint == DEFAULT_AGENT_ENDPOINT {
        start_bundled_agent()?;
        started_bundled_agent = true;
        let deadline = Instant::now()
            + Duration::from_millis(options.timeout_ms.unwrap_or(30_000).max(1) as u64);
        while Instant::now() < deadline {
            status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
            if status.is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
    }
    let status = status.map_err(|error| {
        if started_bundled_agent {
            format!(
                "{error}; the bundled macOS XCUITest agent did not become ready. Ensure Xcode is installed and the process running Xcode has UI automation permission in macOS Privacy & Security settings"
            )
        } else {
            error
        }
    })?;
    let host_name = result_string(&status, "device_name")?.to_string();
    Ok(DesktopConnectInfo {
        host_name,
        note: "connected to the native Allwright macOS XCUITest agent".to_string(),
        desktop_session: DesktopSessionHandle {
            platform: DesktopPlatform::Mac,
            endpoint,
            session_id: unique_id("mac-xctest"),
        },
        initial_app: DesktopAppInfo {
            note: "macOS desktop connected; launch an application before using the context"
                .to_string(),
            app_session: DesktopAppSessionHandle {
                app_session_id: unique_id("mac-pending"),
                app_id: None,
            },
        },
    })
}

fn launch_app(
    desktop_session: &DesktopSessionHandle,
    options: &LaunchOptions,
) -> Result<DesktopAppInfo, String> {
    if options.app_id.trim().is_empty() {
        return Err("desktop-mac launch requires app_id".to_string());
    }
    let mut request = AgentRequest::new("launch");
    request.bundle_id = Some(&options.app_id);
    request.terminate_running = Some(options.terminate_running);
    request.timeout_ms = options.timeout_ms;
    let result = invoke_agent(&desktop_session.endpoint, &request, options.timeout_ms)?;
    Ok(DesktopAppInfo {
        note: format!("launched macOS app `{}` through XCUITest", options.app_id),
        app_session: DesktopAppSessionHandle {
            app_session_id: result_string(&result, "session_id")?.to_string(),
            app_id: Some(options.app_id.clone()),
        },
    })
}

fn agent_action(
    command: &str,
    desktop_session: &DesktopSessionHandle,
    app_session: &DesktopAppSessionHandle,
    selector: &str,
    value: Option<&str>,
    key: Option<&str>,
    text: Option<&str>,
    visible: Option<bool>,
    timeout_ms: Option<u32>,
) -> Result<Value, String> {
    let selector = normalize_selector_for_transport(selector);
    let mut request = AgentRequest::new(command);
    request.session_id = Some(&app_session.app_session_id);
    request.selector = Some(&selector);
    request.value = value;
    request.key = key;
    request.text = text;
    request.visible = visible;
    request.timeout_ms = timeout_ms;
    invoke_agent(&desktop_session.endpoint, &request, timeout_ms)
}

fn action_info(selector: String, action: &str) -> DesktopActionInfo {
    DesktopActionInfo {
        selector,
        note: format!("{action} macOS element through XCUITest"),
    }
}

fn handle_plugin_command(command: DesktopCommand) -> Result<DesktopCommandResult, String> {
    match command {
        DesktopCommand::Connect(options) => connect(&options).map(DesktopCommandResult::Connect),
        DesktopCommand::LaunchApp {
            desktop_session,
            options,
        } => launch_app(&desktop_session, &options).map(DesktopCommandResult::LaunchApp),
        DesktopCommand::CloseApp {
            desktop_session,
            app_session,
        } => {
            if app_session.app_id.is_some() {
                let mut request = AgentRequest::new("close");
                request.session_id = Some(&app_session.app_session_id);
                invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            }
            Ok(DesktopCommandResult::CloseApp)
        }
        DesktopCommand::ClickElement {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            agent_action(
                "click",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::ClickElement(action_info(
                selector, "clicked",
            )))
        }
        DesktopCommand::CountElements {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            let result = agent_action(
                "count",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::CountElements(
                DesktopElementCountInfo {
                    selector,
                    count: result.get("count").and_then(Value::as_u64).unwrap_or(0) as u32,
                    note: "counted macOS elements through XCUITest".to_string(),
                },
            ))
        }
        DesktopCommand::FocusElement {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            agent_action(
                "focus",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::FocusElement(action_info(
                selector, "focused",
            )))
        }
        DesktopCommand::FillElement {
            desktop_session,
            app_session,
            selector,
            value,
            timeout_ms,
        } => {
            agent_action(
                "fill",
                &desktop_session,
                &app_session,
                &selector,
                Some(&value),
                None,
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::FillElement(action_info(
                selector, "filled",
            )))
        }
        DesktopCommand::PressKey {
            desktop_session,
            app_session,
            selector,
            key,
            text,
            timeout_ms,
        } => {
            agent_action(
                "press",
                &desktop_session,
                &app_session,
                &selector,
                None,
                Some(&key),
                text.as_deref(),
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::PressKey(action_info(
                selector,
                "sent key input to",
            )))
        }
        DesktopCommand::GetText {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            let result = agent_action(
                "text",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                None,
                timeout_ms,
            )?;
            let info = DesktopTextInfo {
                selector,
                text: result
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                note: "read macOS accessibility text through XCUITest".to_string(),
            };
            Ok(DesktopCommandResult::GetText(info))
        }
        DesktopCommand::GetInnerText {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            let result = agent_action(
                "text",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::GetInnerText(DesktopTextInfo {
                selector,
                text: result
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                note: "read macOS accessibility text through XCUITest".to_string(),
            }))
        }
        DesktopCommand::WaitForSelector {
            desktop_session,
            app_session,
            selector,
            visible,
            timeout_ms,
        } => {
            agent_action(
                "wait",
                &desktop_session,
                &app_session,
                &selector,
                None,
                None,
                None,
                Some(visible),
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::WaitForSelector(DesktopWaitInfo {
                selector,
                visible,
                note: "waited for macOS element through XCUITest".to_string(),
            }))
        }
        DesktopCommand::Screenshot {
            desktop_session,
            app_session,
            timeout_ms,
        } => {
            let mut request = AgentRequest::new("screenshot");
            request.session_id = Some(&app_session.app_session_id);
            request.timeout_ms = timeout_ms;
            let result = invoke_agent(&desktop_session.endpoint, &request, timeout_ms)?;
            let png_data = base64::engine::general_purpose::STANDARD
                .decode(result_string(&result, "png_base64")?)
                .map_err(|error| {
                    format!("macOS agent returned invalid screenshot data: {error}")
                })?;
            Ok(DesktopCommandResult::Screenshot(DesktopScreenshotInfo {
                png_data,
                note: "captured macOS screenshot through XCUITest".to_string(),
            }))
        }
        DesktopCommand::AccessibilitySnapshot {
            desktop_session,
            app_session,
            format,
            mode,
        } => {
            let mut request = AgentRequest::new("source");
            request.session_id = Some(&app_session.app_session_id);
            request.value = Some(&mode);
            let result = invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            let json_snapshot = result_string(&result, "snapshot")?;
            let snapshot = match format.as_str() {
                "" | "json" => json_snapshot.to_string(),
                "yaml" => {
                    let value: Value = serde_json::from_str(json_snapshot).map_err(|error| {
                        format!("macOS agent returned invalid snapshot JSON: {error}")
                    })?;
                    serde_yaml::to_string(&value)
                        .map_err(|error| format!("failed to encode macOS snapshot YAML: {error}"))?
                }
                other => {
                    return Err(format!(
                        "unsupported accessibility snapshot format `{other}`"
                    ));
                }
            };
            Ok(DesktopCommandResult::AccessibilitySnapshot(
                AccessibilitySnapshotInfo {
                    snapshot,
                    format: if format.is_empty() {
                        "json".to_string()
                    } else {
                        format
                    },
                },
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DesktopPluginEnvelope {
    ok: bool,
    result: Option<DesktopCommandResult>,
    error: Option<String>,
}

fn plugin_response(result: Result<DesktopCommandResult, String>) -> *mut c_char {
    let payload = match result {
        Ok(result) => DesktopPluginEnvelope {
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => DesktopPluginEnvelope {
            ok: false,
            result: None,
            error: Some(error),
        },
    };
    let json = serde_json::to_string(&payload).unwrap_or_else(|error| {
        json!({"ok": false, "result": null, "error": format!("failed to serialize plugin response: {error}")}).to_string()
    });
    CString::new(json)
        .expect("JSON cannot contain NUL")
        .into_raw()
}

pub async fn boot() -> String {
    boot_surface("macos", 10).await
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_api_version() -> u32 {
    ALLWRIGHT_PLUGIN_API_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_id() -> *const c_char {
    c"desktop-mac".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allwright_plugin_invoke(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return plugin_response(Err("plugin request pointer is null".to_string()));
    }
    let request = match unsafe { CStr::from_ptr(request_json) }.to_str() {
        Ok(request) => request,
        Err(error) => {
            return plugin_response(Err(format!("plugin request is not valid UTF-8: {error}")));
        }
    };
    let command = match serde_json::from_str::<DesktopCommand>(request) {
        Ok(command) => command,
        Err(error) => {
            return plugin_response(Err(format!(
                "failed to parse desktop plugin request JSON: {error}"
            )));
        }
    };
    plugin_response(handle_plugin_command(command))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allwright_plugin_free_string(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            let _ = CString::from_raw(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_macos_runtime() {
        assert_eq!(boot().await, "macos ready");
    }

    #[test]
    fn exposes_runtime_plugin_descriptor() {
        assert_eq!(descriptor().id, "desktop-mac");
        assert_eq!(descriptor().family, SurfaceFamily::Desktop);
    }

    #[test]
    fn parses_agent_endpoints() {
        assert_eq!(
            parse_http_endpoint(DEFAULT_AGENT_ENDPOINT).unwrap(),
            ("127.0.0.1".to_string(), 8200, "/v1/command".to_string())
        );
    }
}
