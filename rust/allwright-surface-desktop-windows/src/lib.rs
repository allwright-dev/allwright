use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AccessibilitySnapshotInfo, CaptureInfo, SurfaceFamily,
    SurfacePlugin, SurfacePluginDescriptor,
};
use allwright_surface_desktop::{
    ConnectOptions, DesktopActionInfo, DesktopAppInfo, DesktopAppSessionHandle, DesktopCommand,
    DesktopCommandResult, DesktopConnectInfo, DesktopElementCountInfo, DesktopNavigationInfo,
    DesktopPlatform, DesktopScreenshotInfo, DesktopSessionHandle, DesktopTextInfo, DesktopWaitInfo,
    LaunchOptions, boot_surface, normalize_selector_for_transport,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    env,
    ffi::{CStr, CString, c_char},
    io::{Read, Write},
    net::{Shutdown, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_AGENT_ENDPOINT: &str = "http://127.0.0.1:8300";
static AGENT_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopWindowsPlugin;
impl SurfacePlugin for DesktopWindowsPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        descriptor()
    }
}
pub fn descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: "desktop-windows",
        family: SurfaceFamily::Desktop,
        version: env!("CARGO_PKG_VERSION"),
        description: "Windows desktop surface plugin backed by a bundled FlaUI UIA3 agent.",
    }
}

#[derive(Debug, Serialize)]
struct AgentRequest<'a> {
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute_name: Option<&'a str>,
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
            app_id: None,
            url: None,
            selector: None,
            kind: None,
            attribute_name: None,
            value: None,
            key: None,
            text: None,
            visible: None,
            timeout_ms: None,
            terminate_running: None,
        }
    }
}
#[derive(Debug, Deserialize)]
struct AgentEnvelope {
    ok: bool,
    #[serde(default)]
    result: Value,
    error: Option<String>,
}

fn unique_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}
fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16, String), String> {
    let endpoint = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "Windows agent endpoint must use http://".to_string())?;
    let (authority, base_path) = endpoint.split_once('/').unwrap_or((endpoint, ""));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse()
                .map_err(|_| format!("invalid Windows agent port in `{authority}`"))?,
        ),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err("Windows agent endpoint host cannot be empty".to_string());
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
        .map_err(|e| format!("could not connect to Windows agent at {endpoint}: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("set agent read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("set agent write timeout: {e}"))?;
    let body =
        serde_json::to_vec(request).map_err(|e| format!("encode Windows agent request: {e}"))?;
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|e| format!("write Windows agent request: {e}"))?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("read Windows agent response: {e}"))?;
    let response = String::from_utf8(response)
        .map_err(|e| format!("Windows agent returned non-UTF-8 data: {e}"))?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "Windows agent returned malformed HTTP".to_string())?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(format!("Windows agent returned `{status}`: {body}"));
    }
    let envelope: AgentEnvelope =
        serde_json::from_str(body).map_err(|e| format!("decode Windows agent response: {e}"))?;
    if envelope.ok {
        Ok(envelope.result)
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "Windows agent returned an unknown error".to_string()))
    }
}
fn result_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Windows agent response is missing `{key}`"))
}
fn plugin_install_root() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("ALLWRIGHT_HOME") {
        return Ok(PathBuf::from(root).join("plugins/desktop-windows"));
    }
    let root = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|_| {
            "USERPROFILE is not set; set ALLWRIGHT_HOME so desktop-windows can find its agent"
                .to_string()
        })?;
    Ok(PathBuf::from(root).join(".allwright/plugins/desktop-windows"))
}
fn bundled_agent_path() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("ALLWRIGHT_WINDOWS_AGENT") {
        let path = PathBuf::from(path);
        return path
            .is_file()
            .then_some(path)
            .ok_or_else(|| "ALLWRIGHT_WINDOWS_AGENT does not point to a file".to_string());
    }
    let path = plugin_install_root()?.join("agent/Allwright.WindowsAgent.exe");
    path.is_file().then_some(path.clone()).ok_or_else(|| format!("the desktop-windows plugin archive is missing {}; reinstall it with `allwright plugin install desktop-windows`", path.display()))
}
fn start_bundled_agent() -> Result<(), String> {
    if env::consts::OS != "windows" {
        return Err("desktop-windows only supports Windows".to_string());
    }
    let mut process = AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent process lock is poisoned".to_string())?;
    if let Some(child) = process.as_mut() {
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    *process = Some(
        Command::new(bundled_agent_path()?)
            .arg("--port")
            .arg("8300")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not launch bundled Windows agent: {e}"))?,
    );
    Ok(())
}
fn connect(options: &ConnectOptions) -> Result<DesktopConnectInfo, String> {
    if options.platform != DesktopPlatform::Windows {
        return Err("desktop-windows only supports Windows".to_string());
    }
    let endpoint = options
        .agent_endpoint
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(DEFAULT_AGENT_ENDPOINT)
        .trim_end_matches('/')
        .to_string();
    let mut status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
    if status.is_err() && endpoint == DEFAULT_AGENT_ENDPOINT {
        start_bundled_agent()?;
        let deadline = Instant::now()
            + Duration::from_millis(options.timeout_ms.unwrap_or(30_000).max(1) as u64);
        while Instant::now() < deadline {
            status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
            if status.is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
    }
    let status = status.map_err(|e| {
        format!("{e}; the UIA3 agent did not become ready in the interactive Windows session")
    })?;
    Ok(DesktopConnectInfo {
        host_name: result_string(&status, "host_name")?.to_string(),
        note: "connected to the native Allwright FlaUI UIA3 agent".to_string(),
        desktop_session: DesktopSessionHandle {
            platform: DesktopPlatform::Windows,
            endpoint,
            session_id: unique_id("windows-uia3"),
        },
        initial_app: DesktopAppInfo {
            note: "Windows desktop connected; launch an application before using the context"
                .to_string(),
            app_session: DesktopAppSessionHandle {
                app_session_id: unique_id("windows-pending"),
                app_id: None,
            },
        },
    })
}
fn launch_app(
    desktop: &DesktopSessionHandle,
    options: &LaunchOptions,
) -> Result<DesktopAppInfo, String> {
    if options.app_id.trim().is_empty() {
        return Err("desktop-windows launch requires app_id".to_string());
    }
    let mut request = AgentRequest::new("launch");
    request.app_id = Some(&options.app_id);
    request.terminate_running = Some(options.terminate_running);
    request.timeout_ms = options.timeout_ms;
    let result = invoke_agent(&desktop.endpoint, &request, options.timeout_ms)?;
    Ok(DesktopAppInfo {
        note: format!("launched Windows app `{}` through UIA3", options.app_id),
        app_session: DesktopAppSessionHandle {
            app_session_id: result_string(&result, "session_id")?.to_string(),
            app_id: Some(options.app_id.clone()),
        },
    })
}
fn agent_action(
    command: &str,
    desktop: &DesktopSessionHandle,
    app: &DesktopAppSessionHandle,
    selector: &str,
    value: Option<&str>,
    key: Option<&str>,
    text: Option<&str>,
    visible: Option<bool>,
    timeout_ms: Option<u32>,
) -> Result<Value, String> {
    let normalized = normalize_selector_for_transport(selector);
    let mut request = AgentRequest::new(command);
    request.session_id = Some(&app.app_session_id);
    request.selector = Some(&normalized);
    request.value = value;
    request.key = key;
    request.text = text;
    request.visible = visible;
    request.timeout_ms = timeout_ms;
    invoke_agent(&desktop.endpoint, &request, timeout_ms)
}
fn action_info(selector: String, action: &str) -> DesktopActionInfo {
    DesktopActionInfo {
        selector,
        note: format!("{action} Windows element through UIA3"),
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
                let mut r = AgentRequest::new("close");
                r.session_id = Some(&app_session.app_session_id);
                invoke_agent(&desktop_session.endpoint, &r, Some(10_000))?;
            }
            Ok(DesktopCommandResult::CloseApp)
        }
        DesktopCommand::NavigateApp {
            desktop_session,
            app_session,
            url,
            timeout_ms,
        } => {
            let mut request = AgentRequest::new("open_url");
            request.session_id = Some(&app_session.app_session_id);
            request.url = Some(&url);
            request.timeout_ms = timeout_ms;
            invoke_agent(&desktop_session.endpoint, &request, timeout_ms)?;
            Ok(DesktopCommandResult::NavigateApp(DesktopNavigationInfo {
                url,
                note: "opened URL through the Windows shell".to_string(),
            }))
        }
        DesktopCommand::Capture {
            desktop_session,
            app_session,
            kind,
            selector,
            attribute_name,
            timeout_ms,
        } => {
            let selector = normalize_selector_for_transport(&selector);
            let mut request = AgentRequest::new("capture");
            request.session_id = Some(&app_session.app_session_id);
            request.selector = Some(&selector);
            request.kind = Some(&kind);
            request.attribute_name = Some(&attribute_name);
            request.timeout_ms = timeout_ms;
            let result = invoke_agent(&desktop_session.endpoint, &request, timeout_ms)?;
            let capture: CaptureInfo = serde_json::from_value(result).map_err(|error| {
                format!("Windows agent returned an invalid capture result: {error}")
            })?;
            Ok(DesktopCommandResult::Capture(capture))
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
            let value = agent_action(
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
                    count: value.get("count").and_then(Value::as_u64).unwrap_or(0) as u32,
                    note: "counted Windows elements through UIA3".to_string(),
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
            let value = agent_action(
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
            Ok(DesktopCommandResult::GetText(DesktopTextInfo {
                selector,
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                note: "read Windows accessibility text through UIA3".to_string(),
            }))
        }
        DesktopCommand::GetInnerText {
            desktop_session,
            app_session,
            selector,
            timeout_ms,
        } => {
            let value = agent_action(
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
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                note: "read Windows accessibility text through UIA3".to_string(),
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
                note: "waited for Windows element through UIA3".to_string(),
            }))
        }
        DesktopCommand::Screenshot {
            desktop_session,
            app_session,
            timeout_ms,
        } => {
            let mut r = AgentRequest::new("screenshot");
            r.session_id = Some(&app_session.app_session_id);
            r.timeout_ms = timeout_ms;
            let value = invoke_agent(&desktop_session.endpoint, &r, timeout_ms)?;
            let png_data = base64::engine::general_purpose::STANDARD
                .decode(result_string(&value, "png_base64")?)
                .map_err(|e| format!("Windows agent returned invalid screenshot data: {e}"))?;
            Ok(DesktopCommandResult::Screenshot(DesktopScreenshotInfo {
                png_data,
                note: "captured Windows app screenshot through UIA3".to_string(),
            }))
        }
        DesktopCommand::AccessibilitySnapshot {
            desktop_session,
            app_session,
            format,
            mode,
        } => {
            let mut r = AgentRequest::new("source");
            r.session_id = Some(&app_session.app_session_id);
            r.value = Some(&mode);
            let value = invoke_agent(&desktop_session.endpoint, &r, Some(10_000))?;
            let raw = result_string(&value, "snapshot")?;
            let snapshot = match format.as_str() {
                "" | "json" => raw.to_string(),
                "yaml" => {
                    let value: Value = serde_json::from_str(raw).map_err(|e| {
                        format!("Windows agent returned invalid snapshot JSON: {e}")
                    })?;
                    serde_yaml::to_string(&value)
                        .map_err(|e| format!("encode Windows snapshot YAML: {e}"))?
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

#[derive(Serialize, Deserialize)]
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
    let json = serde_json::to_string(&payload).unwrap_or_else(|e| {
        json!({"ok":false,"result":null,"error":format!("serialize plugin response: {e}")})
            .to_string()
    });
    CString::new(json)
        .expect("JSON cannot contain NUL")
        .into_raw()
}
pub async fn boot() -> String {
    boot_surface("windows", 10).await
}
#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_api_version() -> u32 {
    ALLWRIGHT_PLUGIN_API_VERSION
}
#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_id() -> *const c_char {
    c"desktop-windows".as_ptr()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allwright_plugin_invoke(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return plugin_response(Err("plugin request pointer is null".to_string()));
    }
    let request = match unsafe { CStr::from_ptr(request_json) }.to_str() {
        Ok(v) => v,
        Err(e) => return plugin_response(Err(format!("plugin request is not valid UTF-8: {e}"))),
    };
    let command = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return plugin_response(Err(format!("parse desktop plugin request: {e}"))),
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
    async fn boots_windows_runtime() {
        assert_eq!(boot().await, "windows ready");
    }
    #[test]
    fn exposes_runtime_plugin_descriptor() {
        assert_eq!(descriptor().id, "desktop-windows");
    }
    #[test]
    fn parses_agent_endpoint() {
        assert_eq!(
            parse_http_endpoint(DEFAULT_AGENT_ENDPOINT).unwrap(),
            ("127.0.0.1".to_string(), 8300, "/v1/command".to_string())
        );
    }
}
