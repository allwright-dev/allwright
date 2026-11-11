use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AccessibilitySnapshotInfo, CaptureInfo, InspectorSnapshot,
    SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor,
};
use allwright_surface_desktop::{
    ConnectOptions, DesktopActionInfo, DesktopAppInfo, DesktopAppSessionHandle, DesktopCommand,
    DesktopCommandResult, DesktopConnectInfo, DesktopElementCountInfo, DesktopNavigationInfo,
    DesktopPlatform, DesktopScreenshotInfo, DesktopSessionHandle, DesktopTextInfo, DesktopWaitInfo,
    DesktopWindowInfo, LaunchOptions, boot_surface, normalize_selector_for_transport,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env,
    ffi::{CStr, CString, c_char},
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
const DEFAULT_AGENT_ENDPOINT: &str = "http://127.0.0.1:8300";
static AGENT_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static AGENT_ENDPOINT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static OWNED_DESKTOP_SESSIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

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
        description: "Windows desktop surface plugin backed by Allwright's bundled UIA3 agent.",
    }
}

#[derive(Debug, Serialize)]
struct AgentRequest<'a> {
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    window_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hook_id: Option<&'a str>,
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
    full_page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminate_running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u32>,
}
impl<'a> AgentRequest<'a> {
    fn new(command: &'a str) -> Self {
        Self {
            command,
            session_id: None,
            window_id: None,
            hook_id: None,
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
            full_page: None,
            terminate_running: None,
            x: None,
            y: None,
            to_x: None,
            to_y: None,
            delta_x: None,
            delta_y: None,
            duration_ms: None,
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
fn stop_owned_agent() -> Result<(), String> {
    let mut process = AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent process lock is poisoned".to_string())?;
    if let Some(mut child) = process.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *AGENT_ENDPOINT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent endpoint lock is poisoned".to_string())? = None;
    Ok(())
}
fn owned_agent_endpoint() -> Result<Option<String>, String> {
    let mut process = AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent process lock is poisoned".to_string())?;
    let Some(child) = process.as_mut() else {
        return Ok(None);
    };
    match child.try_wait() {
        Ok(None) => AGENT_ENDPOINT
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "Windows agent endpoint lock is poisoned".to_string())
            .map(|endpoint| endpoint.clone()),
        Ok(Some(_)) => {
            process.take();
            *AGENT_ENDPOINT
                .get_or_init(|| Mutex::new(None))
                .lock()
                .map_err(|_| "Windows agent endpoint lock is poisoned".to_string())? = None;
            Ok(None)
        }
        Err(error) => Err(format!(
            "could not inspect owned Windows agent process: {error}"
        )),
    }
}
fn available_agent_endpoint() -> Result<(String, u16), String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("could not reserve a Windows agent port: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("could not read reserved Windows agent port: {error}"))?
        .port();
    Ok((format!("http://127.0.0.1:{port}"), port))
}
fn start_bundled_agent() -> Result<String, String> {
    if env::consts::OS != "windows" {
        return Err("desktop-windows only supports Windows".to_string());
    }
    OWNED_DESKTOP_SESSIONS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map_err(|_| "Windows desktop session lock is poisoned".to_string())?
        .clear();
    stop_owned_agent()?;
    let (endpoint, port) = available_agent_endpoint()?;
    *AGENT_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent process lock is poisoned".to_string())? = Some(
        Command::new(bundled_agent_path()?)
            .arg("--port")
            .arg(port.to_string())
            .arg("--parent-pid")
            .arg(std::process::id().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not launch bundled Windows agent: {e}"))?,
    );
    *AGENT_ENDPOINT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent endpoint lock is poisoned".to_string())? =
        Some(endpoint.clone());
    Ok(endpoint)
}
fn connect(options: &ConnectOptions) -> Result<DesktopConnectInfo, String> {
    if options.platform != DesktopPlatform::Windows {
        return Err("desktop-windows only supports Windows".to_string());
    }
    let configured_endpoint = options
        .agent_endpoint
        .as_deref()
        .filter(|endpoint| !endpoint.trim().is_empty())
        .map(|endpoint| endpoint.trim_end_matches('/').to_string());
    let (endpoint, mut status, started_bundled_agent) = if let Some(endpoint) = configured_endpoint
    {
        let status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
        (endpoint, status, false)
    } else if let Some(endpoint) = owned_agent_endpoint()? {
        let status = invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000));
        if status.is_ok() {
            (endpoint, status, false)
        } else {
            let endpoint = start_bundled_agent()?;
            (endpoint, Err("Windows agent is starting".to_string()), true)
        }
    } else {
        let endpoint = start_bundled_agent()?;
        (endpoint, Err("Windows agent is starting".to_string()), true)
    };
    if started_bundled_agent {
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
    let status = match status {
        Ok(status) => status,
        Err(error) => {
            if started_bundled_agent {
                let _ = stop_owned_agent();
            }
            return Err(format!(
                "{error}; the UIA3 agent did not become ready in the interactive Windows session"
            ));
        }
    };
    let host_name = match result_string(&status, "host_name") {
        Ok(host_name) => host_name.to_string(),
        Err(error) => {
            if started_bundled_agent {
                let _ = stop_owned_agent();
            }
            return Err(error);
        }
    };
    let session_id = unique_id("windows-uia3");
    let owns_endpoint = AGENT_ENDPOINT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Windows agent endpoint lock is poisoned".to_string())?
        .as_deref()
        == Some(endpoint.as_str());
    if owns_endpoint {
        OWNED_DESKTOP_SESSIONS
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .map_err(|_| "Windows desktop session lock is poisoned".to_string())?
            .insert(session_id.clone());
    }
    Ok(DesktopConnectInfo {
        host_name,
        note: "connected to the native Allwright UIA3 agent".to_string(),
        desktop_session: DesktopSessionHandle {
            platform: DesktopPlatform::Windows,
            endpoint,
            session_id,
        },
        initial_app: DesktopAppInfo {
            note: "Windows desktop connected; launch an application before using the context"
                .to_string(),
            app_session: DesktopAppSessionHandle {
                app_session_id: unique_id("windows-pending"),
                app_id: None,
                window_id: None,
            },
        },
    })
}
fn disconnect(desktop: &DesktopSessionHandle) -> Result<(), String> {
    let should_stop = {
        let mut sessions = OWNED_DESKTOP_SESSIONS
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .map_err(|_| "Windows desktop session lock is poisoned".to_string())?;
        if !sessions.remove(&desktop.session_id) {
            return Ok(());
        }
        sessions.is_empty()
    };
    if should_stop {
        stop_owned_agent()?;
    }
    Ok(())
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
            window_id: None,
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
    request.window_id = app.window_id.as_deref();
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

fn inspector_snapshot(raw: &str) -> Result<InspectorSnapshot, String> {
    let mut snapshot: InspectorSnapshot = serde_json::from_str(raw)
        .map_err(|error| format!("Windows agent returned invalid inspector snapshot: {error}"))?;
    snapshot.backend = "UIA3".to_string();
    Ok(snapshot)
}

fn handle_plugin_command(command: DesktopCommand) -> Result<DesktopCommandResult, String> {
    match command {
        DesktopCommand::Connect(options) => connect(&options).map(DesktopCommandResult::Connect),
        DesktopCommand::Disconnect { desktop_session } => {
            disconnect(&desktop_session)?;
            Ok(DesktopCommandResult::Disconnect)
        }
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
            request.window_id = app_session.window_id.as_deref();
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
        DesktopCommand::ScrollElement {
            desktop_session,
            app_session,
            selector,
            direction,
            timeout_ms,
        } => {
            agent_action(
                "scroll",
                &desktop_session,
                &app_session,
                &selector,
                None,
                Some(&direction),
                None,
                None,
                timeout_ms,
            )?;
            Ok(DesktopCommandResult::ScrollElement(action_info(
                selector, "scrolled",
            )))
        }
        DesktopCommand::PointerInput {
            desktop_session,
            app_session,
            action,
            x,
            y,
            to_x,
            to_y,
            delta_x,
            delta_y,
            duration_ms,
        } => {
            let mut request = AgentRequest::new("pointer");
            request.session_id = Some(&app_session.app_session_id);
            request.window_id = app_session.window_id.as_deref();
            request.key = Some(&action);
            request.x = Some(x);
            request.y = Some(y);
            request.to_x = to_x;
            request.to_y = to_y;
            request.delta_x = delta_x;
            request.delta_y = delta_y;
            request.duration_ms = duration_ms;
            invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            Ok(DesktopCommandResult::PointerInput(action_info(
                format!("coordinate({x:.1}, {y:.1})"),
                &format!("performed live {action} at"),
            )))
        }
        DesktopCommand::KeyboardInput {
            desktop_session,
            app_session,
            key,
            text,
        } => {
            let mut request = AgentRequest::new("keyboard");
            request.session_id = Some(&app_session.app_session_id);
            request.window_id = app_session.window_id.as_deref();
            request.key = key.as_deref();
            request.text = text.as_deref();
            invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            Ok(DesktopCommandResult::KeyboardInput(action_info(
                "focused element".to_string(),
                "sent live keyboard input to",
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
            full_page,
        } => {
            let mut r = AgentRequest::new("screenshot");
            let agent_timeout_ms = timeout_ms.or(full_page.then_some(30_000));
            r.session_id = Some(&app_session.app_session_id);
            r.window_id = app_session.window_id.as_deref();
            r.timeout_ms = agent_timeout_ms;
            r.full_page = Some(full_page);
            let value = invoke_agent(&desktop_session.endpoint, &r, agent_timeout_ms)?;
            let png_data = base64::engine::general_purpose::STANDARD
                .decode(result_string(&value, "png_base64")?)
                .map_err(|e| format!("Windows agent returned invalid screenshot data: {e}"))?;
            Ok(DesktopCommandResult::Screenshot(DesktopScreenshotInfo {
                png_data,
                note: if full_page {
                    "captured full-page Windows app screenshot by scrolling through UIA3"
                } else {
                    "captured Windows application-window screenshot through UIA3"
                }
                .to_string(),
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
            r.window_id = app_session.window_id.as_deref();
            r.value = Some(&mode);
            r.timeout_ms = Some(8_000);
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
        DesktopCommand::InspectorSnapshot {
            desktop_session,
            app_session,
        } => {
            let mut request = AgentRequest::new("source");
            request.session_id = Some(&app_session.app_session_id);
            request.window_id = app_session.window_id.as_deref();
            request.value = Some("default");
            request.timeout_ms = Some(8_000);
            let value = invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            let raw = result_string(&value, "snapshot")?;
            Ok(DesktopCommandResult::InspectorSnapshot(inspector_snapshot(
                raw,
            )?))
        }
        DesktopCommand::ListWindows {
            desktop_session,
            app_session,
        } => {
            let mut request = AgentRequest::new("windows");
            request.session_id = Some(&app_session.app_session_id);
            let value = invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            let windows = serde_json::from_value::<Vec<DesktopWindowInfo>>(
                value
                    .get("windows")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
            )
            .map_err(|error| format!("Windows agent returned invalid window data: {error}"))?;
            Ok(DesktopCommandResult::ListWindows { windows })
        }
        DesktopCommand::CloseWindow {
            desktop_session,
            app_session,
        } => {
            let window_id = app_session
                .window_id
                .as_deref()
                .ok_or_else(|| "close_window requires a desktop window handle".to_string())?;
            let mut request = AgentRequest::new("close_window");
            request.session_id = Some(&app_session.app_session_id);
            request.window_id = Some(window_id);
            invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            Ok(DesktopCommandResult::CloseWindow)
        }
        DesktopCommand::RegisterWindowHook {
            desktop_session,
            app_session,
        } => {
            let mut request = AgentRequest::new("register_window_hook");
            request.session_id = Some(&app_session.app_session_id);
            let value = invoke_agent(&desktop_session.endpoint, &request, Some(10_000))?;
            Ok(DesktopCommandResult::RegisterWindowHook {
                hook_id: result_string(&value, "hook_id")?.to_string(),
            })
        }
        DesktopCommand::WaitForWindowHook {
            desktop_session,
            app_session,
            hook_id,
            timeout_ms,
        } => {
            let mut request = AgentRequest::new("wait_window_hook");
            request.session_id = Some(&app_session.app_session_id);
            request.hook_id = Some(&hook_id);
            request.timeout_ms = timeout_ms;
            let value = invoke_agent(&desktop_session.endpoint, &request, timeout_ms)?;
            let window = serde_json::from_value::<DesktopWindowInfo>(value)
                .map_err(|error| format!("Windows agent returned invalid window data: {error}"))?;
            Ok(DesktopCommandResult::WaitForWindowHook(window))
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
    #[test]
    fn parses_canonical_uia3_inspector_snapshot() {
        let raw = json!({
            "version": 1,
            "platform": "windows",
            "backend": "uia3",
            "root": {
                "role": "button",
                "name": "Save",
                "automationId": "save-button",
                "className": "Button",
                "enabled": true,
                "offscreen": false,
                "bounds": {"x": 10, "y": 20, "width": 80, "height": 30},
                "children": []
            }
        });
        let snapshot = inspector_snapshot(&raw.to_string()).unwrap();
        assert_eq!(snapshot.backend, "UIA3");
        assert_eq!(snapshot.root.automation_id, "save-button");
        assert_eq!(snapshot.root.bounds.width, 80.0);
    }
}
