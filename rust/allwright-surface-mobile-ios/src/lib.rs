use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AccessibilitySnapshotInfo, SurfaceFamily, SurfacePlugin,
    SurfacePluginDescriptor,
};
use allwright_surface_mobile::{
    ConnectOptions, DeviceConnectionKind, DeviceTarget, LaunchOptions, MobileAppKind,
    MobileAutomationBackend, MobileAutomationSessionInfo, MobileBrowserSessionHandle,
    MobileCapabilitySet, MobileClickInfo, MobileCommand, MobileCommandResult, MobileConnectInfo,
    MobileElementCountInfo, MobileElementInfo, MobileFillInfo, MobilePageInfo,
    MobilePageSessionHandle, MobilePlatform, MobilePressInfo, MobileRuntimeReadiness,
    MobileScreenshotInfo, MobileSurfaceProfile, MobileTextInfo, MobileWaitForSelectorInfo,
    RuntimeMaturity, boot_surface, normalize_selector_for_transport,
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

const DEFAULT_AGENT_ENDPOINT: &str = "http://127.0.0.1:8100";
static AGENT_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
const IOS_BACKENDS: &[MobileAutomationBackend] = &[MobileAutomationBackend::XcuiTest];
const IOS_APP_KINDS: &[MobileAppKind] = &[
    MobileAppKind::Native,
    MobileAppKind::Hybrid,
    MobileAppKind::BrowserWrapped,
];
const IOS_MISSING_RUNTIME_ARTIFACTS: &[&str] =
    &["automatic signed runner provisioning and usbmux forwarding for physical devices"];
const IOS_NEXT_MILESTONES: &[&str] = &[
    "package device-signed XCUITest runners and usbmux forwarding",
    "add file chooser and download hooks",
];

#[derive(Debug, Clone, Copy, Default)]
pub struct MobileIosPlugin;

impl SurfacePlugin for MobileIosPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        descriptor()
    }
}

pub fn descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: "mobile-ios",
        family: SurfaceFamily::Mobile,
        version: env!("CARGO_PKG_VERSION"),
        description: "iOS mobile surface plugin backed by the Allwright XCUITest agent.",
    }
}

pub fn profile() -> MobileSurfaceProfile {
    MobileSurfaceProfile {
        plugin_id: "mobile-ios",
        display_name: "iOS",
        family: SurfaceFamily::Mobile,
        backends: IOS_BACKENDS,
        default_backend: MobileAutomationBackend::XcuiTest,
        supported_app_kinds: IOS_APP_KINDS,
        capabilities: MobileCapabilitySet {
            supports_native_views: true,
            supports_webviews: false,
            supports_deep_links: false,
            supports_shell_commands: false,
            supports_device_logs: false,
        },
        bootstrap_hint: "The installed plugin starts its bundled XCUITest runner automatically for simulators. Physical devices currently require a signed runner and forwarded agent endpoint.",
    }
}

pub fn runtime_readiness() -> MobileRuntimeReadiness {
    MobileRuntimeReadiness {
        maturity: RuntimeMaturity::RuntimeReady,
        missing_runtime_artifacts: IOS_MISSING_RUNTIME_ARTIFACTS,
        next_milestones: IOS_NEXT_MILESTONES,
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

fn endpoint_for_connect(options: &ConnectOptions) -> String {
    options
        .agent_endpoint
        .as_deref()
        .or(options.adb_endpoint.as_deref())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_AGENT_ENDPOINT)
        .trim_end_matches('/')
        .to_string()
}

fn endpoint_from_session(session: &MobileBrowserSessionHandle) -> Result<&str, String> {
    session
        .automation
        .note
        .strip_prefix("agent_endpoint=")
        .ok_or_else(|| "iOS session is missing its native agent endpoint".to_string())
}

fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16, String), String> {
    let endpoint = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "iOS agent endpoint must use http://".to_string())?;
    let (authority, base_path) = endpoint.split_once('/').unwrap_or((endpoint, ""));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>()
                .map_err(|_| format!("invalid iOS agent port in `{authority}`"))?,
        ),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err("iOS agent endpoint host cannot be empty".to_string());
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
    // XCTest waits consume the user deadline themselves. Leave a small framing
    // margin so the agent can serialize a success or useful timeout error.
    let timeout =
        Duration::from_millis(timeout_ms.unwrap_or(10_000).max(1).saturating_add(2_000) as u64);
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|error| format!("could not connect to iOS agent at {endpoint}: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("could not set iOS agent read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("could not set iOS agent write timeout: {error}"))?;

    let body = serde_json::to_vec(request)
        .map_err(|error| format!("failed to encode iOS agent request: {error}"))?;
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|error| format!("failed to write iOS agent request: {error}"))?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read iOS agent response: {error}"))?;
    let response = String::from_utf8(response)
        .map_err(|error| format!("iOS agent returned non-UTF-8 HTTP data: {error}"))?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "iOS agent returned a malformed HTTP response".to_string())?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(format!("iOS agent returned `{status}`: {body}"));
    }
    let envelope: AgentEnvelope = serde_json::from_str(body)
        .map_err(|error| format!("failed to decode iOS agent response: {error}"))?;
    if envelope.ok {
        Ok(envelope.result)
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "iOS agent returned an unknown error".to_string()))
    }
}

fn result_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("iOS agent response is missing `{key}`"))
}

fn plugin_install_root() -> Result<PathBuf, String> {
    if let Ok(home) = env::var("ALLWRIGHT_HOME") {
        return Ok(PathBuf::from(home).join("plugins/mobile-ios"));
    }
    let home = env::var("HOME").map_err(|_| {
        "HOME is not set; set ALLWRIGHT_HOME so mobile-ios can locate its bundled agent".to_string()
    })?;
    Ok(PathBuf::from(home).join(".allwright/plugins/mobile-ios"))
}

fn bundled_agent_xctestrun() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("ALLWRIGHT_IOS_AGENT_XCTESTRUN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "ALLWRIGHT_IOS_AGENT_XCTESTRUN does not point to a file: {}",
            path.display()
        ));
    }

    let agent_dir = plugin_install_root()?.join("agent");
    let entries = fs::read_dir(&agent_dir).map_err(|error| {
        format!(
            "the mobile-ios plugin has no bundled XCUITest agent at {}: {error}; reinstall it with `allwright plugin install mobile-ios`",
            agent_dir.display()
        )
    })?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "xctestrun")
        })
        .ok_or_else(|| {
            format!(
                "the mobile-ios plugin archive is missing an .xctestrun file under {}",
                agent_dir.display()
            )
        })
}

fn available_simulator(requested: Option<&str>) -> Result<(String, String), String> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available", "--json"])
        .output()
        .map_err(|error| format!("could not run `xcrun simctl`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`xcrun simctl list devices available --json` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let document: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("simctl returned invalid JSON: {error}"))?;
    let mut simulators = document
        .get("devices")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|runtimes| runtimes.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter(|device| {
            device
                .get("isAvailable")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        })
        .filter_map(|device| {
            Some((
                device.get("udid")?.as_str()?.to_string(),
                device.get("name")?.as_str()?.to_string(),
                device.get("state").and_then(Value::as_str) == Some("Booted"),
            ))
        })
        .collect::<Vec<_>>();

    if let Some(requested) = requested {
        return simulators
            .into_iter()
            .find(|(id, name, _)| id == requested || name.eq_ignore_ascii_case(requested))
            .map(|(id, name, _)| (id, name))
            .ok_or_else(|| format!("no available iOS Simulator matches `{requested}`"));
    }
    simulators.sort_by_key(|(_, name, booted)| (!*booted, !name.starts_with("iPhone")));
    simulators
        .into_iter()
        .next()
        .map(|(id, name, _)| (id, name))
        .ok_or_else(|| "no available iOS Simulator was found".to_string())
}

fn start_bundled_agent(requested_device: Option<&str>) -> Result<String, String> {
    let xctestrun = bundled_agent_xctestrun()?;
    let (simulator_id, simulator_name) = available_simulator(requested_device)?;
    let process = AGENT_PROCESS.get_or_init(|| Mutex::new(None));
    let mut process = process
        .lock()
        .map_err(|_| "iOS agent process lock is poisoned".to_string())?;
    if let Some(child) = process.as_mut() {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            return Ok(simulator_name);
        }
    }

    let child = Command::new("xcodebuild")
        .arg("test-without-building")
        .arg("-xctestrun")
        .arg(&xctestrun)
        .arg("-destination")
        .arg(format!("platform=iOS Simulator,id={simulator_id}"))
        .args([
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
                "could not launch bundled iOS agent {}: {error}",
                xctestrun.display()
            )
        })?;
    *process = Some(child);
    Ok(simulator_name)
}

fn connect_agent(options: &ConnectOptions, endpoint: &str) -> Result<Value, String> {
    match invoke_agent(endpoint, &AgentRequest::new("status"), options.timeout_ms) {
        Ok(status) => return Ok(status),
        Err(initial_error) if endpoint != DEFAULT_AGENT_ENDPOINT => return Err(initial_error),
        Err(_) => {}
    }

    let simulator_name = start_bundled_agent(options.device.as_deref())?;
    let deadline =
        Instant::now() + Duration::from_millis(options.timeout_ms.unwrap_or(30_000).max(1) as u64);
    let mut last_error = String::new();
    while Instant::now() < deadline {
        match invoke_agent(endpoint, &AgentRequest::new("status"), Some(1_000)) {
            Ok(status) => return Ok(status),
            Err(error) => last_error = error,
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "bundled XCUITest agent did not become ready on `{simulator_name}`: {last_error}"
    ))
}

pub fn connect(options: &ConnectOptions) -> Result<MobileConnectInfo, String> {
    if options.platform != MobilePlatform::Ios {
        return Err("mobile-ios connect only supports the iOS platform".to_string());
    }
    let endpoint = endpoint_for_connect(options);
    let status = connect_agent(options, &endpoint)?;
    let device_id = result_string(&status, "device_id")?.to_string();
    let device_name = result_string(&status, "device_name")?.to_string();
    if let Some(requested) = options.device.as_deref()
        && requested != device_id
        && !requested.eq_ignore_ascii_case(&device_name)
    {
        return Err(format!(
            "iOS agent is attached to `{device_name}` ({device_id}), not requested device `{requested}`"
        ));
    }
    let connection_kind = if status
        .get("simulator")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        DeviceConnectionKind::Emulator
    } else {
        DeviceConnectionKind::Usb
    };
    let session_id = unique_id("xctest");

    Ok(MobileConnectInfo {
        browser: device_name,
        note: "connected to the native Allwright XCUITest agent".to_string(),
        browser_session: MobileBrowserSessionHandle {
            platform: MobilePlatform::Ios,
            automation: MobileAutomationSessionInfo {
                backend: "xctest".to_string(),
                session_id,
                note: format!("agent_endpoint={endpoint}"),
            },
            device: DeviceTarget {
                platform: MobilePlatform::Ios,
                device_id,
                connection_kind,
            },
        },
        initial_page: MobilePageInfo {
            note: "agent connected; provision or launch an iOS app".to_string(),
            page_session: MobilePageSessionHandle {
                page_id: unique_id("ios-app"),
                package_name: None,
                activity_name: None,
                webview_context: None,
            },
        },
    })
}

pub fn launch_app(
    browser_session: &MobileBrowserSessionHandle,
    options: &LaunchOptions,
) -> Result<MobilePageInfo, String> {
    validate_ios_session(browser_session)?;
    let resolved_app = options
        .apk_path
        .as_deref()
        .map(|source| resolve_ios_app_source(source))
        .transpose()?;
    let discovered_bundle_id = resolved_app
        .as_ref()
        .map(|app| bundle_identifier(&app.path))
        .transpose()?;
    if let (Some(requested), Some(discovered)) =
        (options.app_id.as_deref(), discovered_bundle_id.as_deref())
        && requested != discovered
    {
        return Err(format!(
            "configured iOS app id `{requested}` does not match bundle `{discovered}` from the app binary"
        ));
    }
    let bundle_id = options
        .app_id
        .as_deref()
        .or(discovered_bundle_id.as_deref())
        .ok_or_else(|| {
            "iOS launch_app requires `app_id` or an app binary whose bundle id can be resolved"
                .to_string()
        })?;
    if let Some(app) = resolved_app.as_ref() {
        install_simulator_app(&browser_session.device.device_id, &app.path)?;
    }
    let endpoint = endpoint_from_session(browser_session)?;
    let mut request = AgentRequest::new("launch");
    request.bundle_id = Some(bundle_id);
    request.terminate_running = Some(options.stop_before_launch);
    request.timeout_ms = options.timeout_ms;
    let result = invoke_agent(endpoint, &request, options.timeout_ms)?;
    let agent_session_id = result_string(&result, "session_id")?.to_string();
    Ok(MobilePageInfo {
        note: format!("launched iOS app `{bundle_id}` through XCUITest"),
        page_session: MobilePageSessionHandle {
            page_id: agent_session_id,
            package_name: Some(bundle_id.to_string()),
            activity_name: None,
            webview_context: None,
        },
    })
}

#[derive(Debug)]
struct ResolvedIosApp {
    path: PathBuf,
    temporary_root: Option<PathBuf>,
}

impl Drop for ResolvedIosApp {
    fn drop(&mut self) {
        if let Some(root) = self.temporary_root.take() {
            let _ = fs::remove_dir_all(root);
        }
    }
}

fn resolve_ios_app_source(source: &str) -> Result<ResolvedIosApp, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return download_and_extract_ios_app(source);
    }
    let path = PathBuf::from(source);
    if path.is_dir() && path.extension().is_some_and(|extension| extension == "app") {
        return Ok(ResolvedIosApp {
            path,
            temporary_root: None,
        });
    }
    if path.is_file() {
        return extract_ios_app_archive(&path, None);
    }
    Err(format!(
        "iOS app binary does not exist or is not an .app bundle/archive: {}",
        path.display()
    ))
}

fn download_and_extract_ios_app(url: &str) -> Result<ResolvedIosApp, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("failed to create iOS app downloader: {error}"))?;
    let mut response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("failed to download iOS app from `{url}`: {error}"))?;
    let root = temporary_app_root()?;
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create {}: {error}", root.display()))?;
    let extension = url
        .split(['?', '#'])
        .next()
        .and_then(|path| Path::new(path).extension())
        .and_then(|value| value.to_str())
        .filter(|value| matches!(value.to_ascii_lowercase().as_str(), "zip" | "ipa"))
        .unwrap_or("zip");
    let archive = root.join(format!("download.{extension}"));
    let mut archive_file = fs::File::create(&archive).map_err(|error| {
        let _ = fs::remove_dir_all(&root);
        format!("failed to create {}: {error}", archive.display())
    })?;
    if let Err(error) = std::io::copy(&mut response, &mut archive_file) {
        let _ = fs::remove_dir_all(&root);
        return Err(format!(
            "failed to write iOS app download to {}: {error}",
            archive.display()
        ));
    }
    drop(archive_file);
    extract_ios_app_archive(&archive, Some(root))
}

fn extract_ios_app_archive(
    archive: &Path,
    existing_root: Option<PathBuf>,
) -> Result<ResolvedIosApp, String> {
    let root = existing_root.map_or_else(temporary_app_root, Ok)?;
    let expanded = root.join("expanded");
    fs::create_dir_all(&expanded)
        .map_err(|error| format!("failed to create {}: {error}", expanded.display()))?;
    let output = Command::new("ditto")
        .args(["-x", "-k"])
        .arg(archive)
        .arg(&expanded)
        .output()
        .map_err(|error| format!("could not extract iOS app archive with `ditto`: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_dir_all(&root);
        return Err(format!(
            "failed to extract iOS app archive {}: {}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let app = match find_primary_app_bundle(&expanded) {
        Ok(Some(app)) => app,
        Ok(None) => {
            let _ = fs::remove_dir_all(&root);
            return Err(format!(
                "iOS app archive {} does not contain an .app bundle",
                archive.display()
            ));
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&root);
            return Err(error);
        }
    };
    Ok(ResolvedIosApp {
        path: app,
        temporary_root: Some(root),
    })
}

fn temporary_app_root() -> Result<PathBuf, String> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to resolve system time: {error}"))?
        .as_nanos();
    Ok(env::temp_dir().join(format!(
        "allwright-mobile-ios-{}-{unique}",
        std::process::id()
    )))
}

fn find_primary_app_bundle(root: &Path) -> Result<Option<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut apps = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if !path.is_dir() {
                continue;
            }
            if path.extension().is_some_and(|extension| extension == "app") {
                apps.push(path);
            } else {
                pending.push(path);
            }
        }
    }
    apps.sort_by_key(|path| path.components().count());
    Ok(apps.into_iter().next())
}

fn bundle_identifier(app: &Path) -> Result<String, String> {
    let plist = app.join("Info.plist");
    let output = Command::new("plutil")
        .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
        .arg(&plist)
        .output()
        .map_err(|error| format!("could not read {} with plutil: {error}", plist.display()))?;
    if !output.status.success() {
        return Err(format!(
            "could not resolve CFBundleIdentifier from {}: {}",
            plist.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Err(format!(
            "{} has an empty CFBundleIdentifier",
            plist.display()
        ))
    } else {
        Ok(value)
    }
}

fn install_simulator_app(device_id: &str, app: &Path) -> Result<(), String> {
    let output = Command::new("xcrun")
        .args(["simctl", "install", device_id])
        .arg(app)
        .output()
        .map_err(|error| format!("could not run `xcrun simctl install`: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to install iOS Simulator app {}: {}. The bundle must be built for iOS Simulator, not a physical device",
            app.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn validate_ios_session(session: &MobileBrowserSessionHandle) -> Result<(), String> {
    if session.platform != MobilePlatform::Ios || session.device.platform != MobilePlatform::Ios {
        return Err("mobile-ios received a non-iOS session".to_string());
    }
    Ok(())
}

fn action_request<'a>(
    command: &'a str,
    page: &'a MobilePageSessionHandle,
    selector: &'a str,
    timeout_ms: Option<u32>,
) -> AgentRequest<'a> {
    let mut request = AgentRequest::new(command);
    request.session_id = Some(&page.page_id);
    request.selector = Some(selector);
    request.timeout_ms = timeout_ms;
    request
}

fn normalized_selector(selector: &str) -> Result<String, String> {
    let selector = normalize_selector_for_transport(selector);
    if selector.trim().is_empty() {
        Err("iOS element command requires a non-empty selector".to_string())
    } else {
        Ok(selector)
    }
}

fn read_text_command(
    browser_session: MobileBrowserSessionHandle,
    page_session: MobilePageSessionHandle,
    selector: String,
    timeout_ms: Option<u32>,
    inner: bool,
) -> Result<MobileCommandResult, String> {
    let selector = normalized_selector(&selector)?;
    let result = invoke_agent(
        endpoint_from_session(&browser_session)?,
        &action_request("text", &page_session, &selector, timeout_ms),
        timeout_ms,
    )?;
    let info = MobileTextInfo {
        selector,
        text: result_string(&result, "text")?.to_string(),
        note: "read iOS accessibility text through XCUITest".to_string(),
    };
    Ok(if inner {
        MobileCommandResult::GetInnerText(info)
    } else {
        MobileCommandResult::GetText(info)
    })
}

fn handle_plugin_command(command: MobileCommand) -> Result<MobileCommandResult, String> {
    match command {
        MobileCommand::Connect(options) => connect(&options).map(MobileCommandResult::Connect),
        MobileCommand::LaunchApp {
            browser_session,
            options,
        } => launch_app(&browser_session, &options).map(MobileCommandResult::LaunchApp),
        MobileCommand::OpenPage { .. } => Err(
            "iOS exposes one app context per launched bundle; call launch_app instead".to_string(),
        ),
        MobileCommand::ClosePage {
            browser_session,
            page_session,
        } => {
            validate_ios_session(&browser_session)?;
            let mut request = AgentRequest::new("close");
            request.session_id = Some(&page_session.page_id);
            invoke_agent(endpoint_from_session(&browser_session)?, &request, None)?;
            Ok(MobileCommandResult::ClosePage)
        }
        MobileCommand::ClickElement {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            invoke_agent(
                endpoint_from_session(&browser_session)?,
                &action_request("click", &page_session, &selector, timeout_ms),
                timeout_ms,
            )?;
            Ok(MobileCommandResult::ClickElement(MobileClickInfo {
                selector,
                note: "clicked iOS element through XCUITest".to_string(),
                session_id: browser_session.automation.session_id,
            }))
        }
        MobileCommand::CountElements {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            let result = invoke_agent(
                endpoint_from_session(&browser_session)?,
                &action_request("count", &page_session, &selector, timeout_ms),
                timeout_ms,
            )?;
            let count = result
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| "iOS agent count response is missing `count`".to_string())?;
            Ok(MobileCommandResult::CountElements(MobileElementCountInfo {
                selector,
                count: u32::try_from(count)
                    .map_err(|_| "iOS element count exceeds u32".to_string())?,
                note: "counted iOS elements through XCUITest".to_string(),
            }))
        }
        MobileCommand::FocusElement {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            invoke_agent(
                endpoint_from_session(&browser_session)?,
                &action_request("focus", &page_session, &selector, timeout_ms),
                timeout_ms,
            )?;
            Ok(MobileCommandResult::FocusElement(MobileElementInfo {
                selector,
                note: "focused iOS element through XCUITest".to_string(),
            }))
        }
        MobileCommand::FillElement {
            browser_session,
            page_session,
            selector,
            value,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            let mut request = action_request("fill", &page_session, &selector, timeout_ms);
            request.value = Some(&value);
            invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                timeout_ms,
            )?;
            Ok(MobileCommandResult::FillElement(MobileFillInfo {
                selector,
                value,
                note: "filled iOS element through XCUITest".to_string(),
            }))
        }
        MobileCommand::PressKey {
            browser_session,
            page_session,
            selector,
            key,
            text,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            let mut request = action_request("press", &page_session, &selector, timeout_ms);
            request.key = Some(&key);
            request.text = text.as_deref();
            invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                timeout_ms,
            )?;
            Ok(MobileCommandResult::PressKey(MobilePressInfo {
                selector,
                key,
                note: "sent key input through XCUITest".to_string(),
            }))
        }
        MobileCommand::GetText {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => read_text_command(browser_session, page_session, selector, timeout_ms, false),
        MobileCommand::GetInnerText {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => read_text_command(browser_session, page_session, selector, timeout_ms, true),
        MobileCommand::WaitForSelector {
            browser_session,
            page_session,
            selector,
            visible,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            let mut request = action_request("wait", &page_session, &selector, timeout_ms);
            request.visible = Some(visible);
            invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                timeout_ms,
            )?;
            Ok(MobileCommandResult::WaitForSelector(
                MobileWaitForSelectorInfo {
                    selector,
                    visible,
                    note: "waited for iOS element through XCUITest".to_string(),
                },
            ))
        }
        MobileCommand::Screenshot {
            browser_session,
            page_session,
            timeout_ms,
            ..
        } => {
            let mut request = AgentRequest::new("screenshot");
            request.session_id = Some(&page_session.page_id);
            request.timeout_ms = timeout_ms;
            let result = invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                timeout_ms,
            )?;
            let png_data = base64::engine::general_purpose::STANDARD
                .decode(result_string(&result, "png_base64")?)
                .map_err(|error| format!("iOS agent returned invalid screenshot data: {error}"))?;
            Ok(MobileCommandResult::Screenshot(MobileScreenshotInfo {
                png_data,
                note: "captured iOS screenshot through XCUITest".to_string(),
            }))
        }
        MobileCommand::AccessibilitySnapshot {
            browser_session,
            page_session,
            format,
            mode,
        } => {
            let mut request = AgentRequest::new("source");
            request.session_id = Some(&page_session.page_id);
            request.value = Some(&mode);
            let result = invoke_agent(endpoint_from_session(&browser_session)?, &request, None)?;
            let snapshot = result_string(&result, "snapshot")?.to_string();
            if format != "json" && !format.is_empty() {
                return Err(
                    "the iOS agent currently supports JSON accessibility snapshots only"
                        .to_string(),
                );
            }
            Ok(MobileCommandResult::AccessibilitySnapshot(
                AccessibilitySnapshotInfo {
                    snapshot,
                    format: "json".to_string(),
                },
            ))
        }
        MobileCommand::RegisterHook { .. }
        | MobileCommand::PollHook { .. }
        | MobileCommand::SetFileChooserFiles { .. }
        | MobileCommand::SaveDownload { .. } => {
            Err("mobile hooks are not yet supported by the iOS XCUITest agent".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobilePluginEnvelope {
    ok: bool,
    result: Option<MobileCommandResult>,
    error: Option<String>,
}

fn plugin_response(result: Result<MobileCommandResult, String>) -> *mut c_char {
    let payload = match result {
        Ok(result) => MobilePluginEnvelope {
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => MobilePluginEnvelope {
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
    boot_surface("ios", 10).await
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_api_version() -> u32 {
    ALLWRIGHT_PLUGIN_API_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_id() -> *const c_char {
    c"mobile-ios".as_ptr()
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
    let command = match serde_json::from_str::<MobileCommand>(request) {
        Ok(command) => command,
        Err(error) => {
            return plugin_response(Err(format!(
                "failed to parse mobile plugin request JSON: {error}"
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
    async fn boots_ios_runtime() {
        assert_eq!(boot().await, "ios ready");
    }

    #[test]
    fn exposes_runtime_plugin_descriptor() {
        assert_eq!(descriptor().id, "mobile-ios");
        assert_eq!(runtime_readiness().maturity, RuntimeMaturity::RuntimeReady);
    }

    #[test]
    fn parses_default_and_nested_agent_endpoints() {
        assert_eq!(
            parse_http_endpoint("http://127.0.0.1:8100").unwrap(),
            ("127.0.0.1".to_string(), 8100, "/v1/command".to_string())
        );
        assert_eq!(
            parse_http_endpoint("http://localhost:9000/agent")
                .unwrap()
                .2,
            "/agent/v1/command"
        );
    }

    #[test]
    fn finds_the_shallowest_app_bundle_in_an_archive_tree() {
        let root = temporary_app_root().unwrap();
        let primary = root.join("Payload/Flights.app");
        let nested = primary.join("PlugIns/Widget.appex/Embedded.app");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(find_primary_app_bundle(&root).unwrap(), Some(primary));

        fs::remove_dir_all(root).unwrap();
    }
}
