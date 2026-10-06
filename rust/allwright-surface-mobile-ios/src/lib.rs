use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AccessibilitySnapshotInfo, CaptureInfo, SurfaceFamily,
    SurfacePlugin, SurfacePluginDescriptor,
};
use allwright_surface_mobile::{
    ConnectOptions, DeviceConnectionKind, DeviceTarget, LaunchOptions, MobileAppKind,
    MobileAutomationBackend, MobileAutomationSessionInfo, MobileBrowserSessionHandle,
    MobileCapabilitySet, MobileClickInfo, MobileCommand, MobileCommandResult, MobileConnectInfo,
    MobileDownloadInfo, MobileDownloadSavedInfo, MobileElementCountInfo, MobileElementInfo,
    MobileFileChooserFilesSetInfo, MobileFileChooserInfo, MobileFillInfo, MobileHookRegistration,
    MobileHookResult, MobileHookType, MobileNavigationInfo, MobilePageInfo,
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
use std::net::{Shutdown, TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_AGENT_ENDPOINT: &str = "http://127.0.0.1:8100";
static AGENT_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static IOS_HOOKS: OnceLock<Mutex<std::collections::HashMap<String, IosHookState>>> =
    OnceLock::new();
static IOS_CHOOSERS: OnceLock<Mutex<std::collections::HashMap<String, IosChooserState>>> =
    OnceLock::new();
static IOS_DOWNLOADS: OnceLock<Mutex<std::collections::HashMap<String, IosDownloadState>>> =
    OnceLock::new();
static IOS_CURRENT_PAGES: OnceLock<
    Mutex<std::collections::HashMap<String, MobilePageSessionHandle>>,
> = OnceLock::new();
const IOS_BACKENDS: &[MobileAutomationBackend] = &[MobileAutomationBackend::XcuiTest];
const IOS_APP_KINDS: &[MobileAppKind] = &[
    MobileAppKind::Native,
    MobileAppKind::Hybrid,
    MobileAppKind::BrowserWrapped,
];
const IOS_MISSING_RUNTIME_ARTIFACTS: &[&str] = &[];
const IOS_NEXT_MILESTONES: &[&str] =
    &["add a WebKit inspector backend for arbitrary WebView DOM and JavaScript sessions"];

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
            supports_webviews: true,
            supports_deep_links: true,
            supports_shell_commands: false,
            supports_device_logs: false,
        },
        bootstrap_hint: "The installed plugin starts its bundled XCUITest runner automatically. Simulator runners are ready to use; physical-device runners are re-signed from local Apple development credentials and forwarded through usbmuxd.",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    full_page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<&'a str>>,
}

impl<'a> AgentRequest<'a> {
    fn new(command: &'a str) -> Self {
        Self {
            command,
            session_id: None,
            bundle_id: None,
            selector: None,
            kind: None,
            attribute_name: None,
            value: None,
            key: None,
            text: None,
            visible: None,
            timeout_ms: None,
            terminate_running: None,
            full_page: None,
            files: None,
        }
    }
}

#[derive(Debug, Clone)]
enum IosHookState {
    FileChooser {
        device_id: String,
        page: MobilePageSessionHandle,
    },
    Download {
        device_id: String,
        page: MobilePageSessionHandle,
        existing_files: std::collections::HashMap<String, IosFileInfo>,
    },
}

#[derive(Debug, Clone)]
struct IosChooserState {
    device_id: String,
    page: MobilePageSessionHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IosFileInfo {
    path: String,
    size: u64,
    signature: String,
    domain_bundle_id: String,
}

#[derive(Debug, Clone)]
struct IosDownloadState {
    device_id: String,
    file: IosFileInfo,
    observed_size: Option<u64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IosTargetKind {
    Simulator,
    Device,
}

impl IosTargetKind {
    fn package_directory(self) -> &'static str {
        match self {
            Self::Simulator => "simulator",
            Self::Device => "device",
        }
    }

    fn products_directory(self) -> &'static str {
        match self {
            Self::Simulator => "Release-iphonesimulator",
            Self::Device => "Release-iphoneos",
        }
    }
}

fn bundled_agent_root(kind: IosTargetKind) -> Result<PathBuf, String> {
    Ok(plugin_install_root()?
        .join("agent")
        .join(kind.package_directory()))
}

fn bundled_agent_xctestrun(kind: IosTargetKind) -> Result<PathBuf, String> {
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

    let agent_dir = bundled_agent_root(kind)?;
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

fn bundled_agent_runner(kind: IosTargetKind) -> Result<PathBuf, String> {
    let path = bundled_agent_root(kind)?
        .join(kind.products_directory())
        .join("AllwrightAgentUITests-Runner.app");
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!(
            "the mobile-ios plugin archive is missing its {} runner at {}",
            kind.package_directory(),
            path.display()
        ))
    }
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

fn stop_owned_agent() -> Result<(), String> {
    let process = AGENT_PROCESS.get_or_init(|| Mutex::new(None));
    let mut process = process
        .lock()
        .map_err(|_| "iOS agent process lock is poisoned".to_string())?;
    if let Some(mut child) = process.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

fn store_agent_process(child: Child) -> Result<(), String> {
    let process = AGENT_PROCESS.get_or_init(|| Mutex::new(None));
    let mut process = process
        .lock()
        .map_err(|_| "iOS agent process lock is poisoned".to_string())?;
    *process = Some(child);
    Ok(())
}

fn start_simulator_agent(
    requested_device: Option<&str>,
) -> Result<(String, String, String), String> {
    let xctestrun = bundled_agent_xctestrun(IosTargetKind::Simulator)?;
    let _ = bundled_agent_runner(IosTargetKind::Simulator)?;
    let (simulator_id, simulator_name) = available_simulator(requested_device)?;
    stop_owned_agent()?;
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
    store_agent_process(child)?;
    Ok((
        simulator_id,
        simulator_name,
        DEFAULT_AGENT_ENDPOINT.to_string(),
    ))
}

#[derive(Debug, Clone)]
struct PhysicalDevice {
    identifier: String,
    udid: String,
    name: String,
}

fn physical_device(requested: &str) -> Result<PhysicalDevice, String> {
    let output_path = env::temp_dir().join(format!(
        "allwright-devicectl-{}-{}.json",
        std::process::id(),
        unique_id("devices")
    ));
    let output = Command::new("xcrun")
        .args(["devicectl", "list", "devices", "--json-output"])
        .arg(&output_path)
        .output()
        .map_err(|error| format!("could not run `xcrun devicectl`: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(format!(
            "`xcrun devicectl list devices` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let bytes = fs::read(&output_path).map_err(|error| {
        format!(
            "could not read devicectl JSON {}: {error}",
            output_path.display()
        )
    })?;
    let _ = fs::remove_file(&output_path);
    let document: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("devicectl returned invalid JSON: {error}"))?;
    let devices = document
        .pointer("/result/devices")
        .and_then(Value::as_array)
        .ok_or_else(|| "devicectl JSON did not contain result.devices".to_string())?;
    devices
        .iter()
        .filter_map(|device| {
            let identifier = device.get("identifier")?.as_str()?.to_string();
            let udid = device
                .pointer("/hardwareProperties/udid")
                .and_then(Value::as_str)
                .unwrap_or(&identifier)
                .to_string();
            let name = device
                .pointer("/deviceProperties/name")
                .or_else(|| device.get("name"))?
                .as_str()?
                .to_string();
            let platform = device
                .pointer("/hardwareProperties/platform")
                .and_then(Value::as_str)
                .unwrap_or("iOS");
            (platform.eq_ignore_ascii_case("iOS")
                && (requested == identifier
                    || requested == udid
                    || requested.eq_ignore_ascii_case(&name)))
            .then_some(PhysicalDevice {
                identifier,
                udid,
                name,
            })
        })
        .next()
        .ok_or_else(|| format!("no connected iOS physical device matches `{requested}`"))
}

fn signing_identity() -> Result<String, String> {
    if let Ok(identity) = env::var("ALLWRIGHT_IOS_SIGNING_IDENTITY")
        && !identity.trim().is_empty()
    {
        return Ok(identity);
    }
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|error| format!("could not inspect code-signing identities: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`security find-identity` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("Apple Development") || line.contains("iPhone Developer"))
        .find_map(|line| line.split('"').nth(1).map(str::to_string))
        .ok_or_else(|| {
            "no Apple Development signing identity was found; sign in to Xcode or set ALLWRIGHT_IOS_SIGNING_IDENTITY"
                .to_string()
        })
}

fn installed_provisioning_profiles() -> Vec<PathBuf> {
    if let Ok(profile) = env::var("ALLWRIGHT_IOS_PROVISIONING_PROFILE")
        && !profile.trim().is_empty()
    {
        return vec![PathBuf::from(profile)];
    }
    let Some(home) = env::var_os("HOME") else {
        return Vec::new();
    };
    [
        PathBuf::from(&home).join("Library/Developer/Xcode/UserData/Provisioning Profiles"),
        PathBuf::from(&home).join("Library/MobileDevice/Provisioning Profiles"),
    ]
    .into_iter()
    .filter_map(|directory| fs::read_dir(directory).ok())
    .flatten()
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|path| path.is_file())
    .collect()
}

fn decode_provisioning_profile(path: &Path) -> Result<plist::Value, String> {
    let output = Command::new("security")
        .args(["cms", "-D", "-i"])
        .arg(path)
        .output()
        .map_err(|error| format!("could not decode {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "could not decode provisioning profile {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    plist::Value::from_reader(std::io::Cursor::new(output.stdout))
        .map_err(|error| format!("invalid provisioning profile {}: {error}", path.display()))
}

fn profile_supports_device(profile: &plist::Value, udid: &str) -> bool {
    profile
        .as_dictionary()
        .and_then(|dictionary| dictionary.get("ProvisionedDevices"))
        .and_then(plist::Value::as_array)
        .is_some_and(|devices| {
            devices
                .iter()
                .filter_map(plist::Value::as_string)
                .any(|candidate| candidate == udid)
        })
}

fn profile_is_usable_for_development(profile: &plist::Value, udid: &str) -> bool {
    let Some(dictionary) = profile.as_dictionary() else {
        return false;
    };
    let development_enabled = dictionary
        .get("Entitlements")
        .and_then(plist::Value::as_dictionary)
        .and_then(|entitlements| entitlements.get("get-task-allow"))
        .and_then(plist::Value::as_boolean)
        .unwrap_or(false);
    let unexpired = dictionary
        .get("ExpirationDate")
        .and_then(plist::Value::as_date)
        .is_some_and(|expiration| SystemTime::from(expiration) > SystemTime::now());
    development_enabled && unexpired && profile_supports_device(profile, udid)
}

fn profile_is_safe_for_agent(profile: &plist::Value) -> bool {
    profile
        .as_dictionary()
        .and_then(|dictionary| dictionary.get("Entitlements"))
        .and_then(plist::Value::as_dictionary)
        .and_then(|entitlements| entitlements.get("application-identifier"))
        .and_then(plist::Value::as_string)
        .is_some_and(|identifier| {
            identifier.ends_with('*') || identifier.to_ascii_lowercase().contains("allwright")
        })
}

fn provisioning_profile(udid: &str) -> Result<(PathBuf, plist::Value), String> {
    let explicit = env::var("ALLWRIGHT_IOS_PROVISIONING_PROFILE").is_ok();
    for path in installed_provisioning_profiles() {
        let Ok(profile) = decode_provisioning_profile(&path) else {
            continue;
        };
        if profile_is_usable_for_development(&profile, udid)
            && (explicit || profile_is_safe_for_agent(&profile))
        {
            return Ok((path, profile));
        }
        if explicit {
            return Err(format!(
                "provisioning profile {} is expired, is not a development profile, or does not include physical device `{udid}`",
                path.display()
            ));
        }
    }
    Err(format!(
        "no unexpired wildcard or Allwright-specific iOS development provisioning profile includes device `{udid}`; create a dedicated agent profile, or set ALLWRIGHT_IOS_PROVISIONING_PROFILE explicitly"
    ))
}

fn profile_entitlements(profile: &plist::Value) -> Result<plist::Dictionary, String> {
    profile
        .as_dictionary()
        .and_then(|dictionary| dictionary.get("Entitlements"))
        .and_then(plist::Value::as_dictionary)
        .cloned()
        .ok_or_else(|| "the provisioning profile has no Entitlements dictionary".to_string())
}

fn signed_runner_bundle_id(entitlements: &plist::Dictionary) -> Result<String, String> {
    let application_identifier = entitlements
        .get("application-identifier")
        .and_then(plist::Value::as_string)
        .ok_or_else(|| "the provisioning profile has no application-identifier".to_string())?;
    let (_, bundle_pattern) = application_identifier.split_once('.').ok_or_else(|| {
        "the provisioning profile application-identifier is malformed".to_string()
    })?;
    let bundle_id = if let Ok(bundle_id) = env::var("ALLWRIGHT_IOS_AGENT_BUNDLE_ID")
        && !bundle_id.trim().is_empty()
    {
        bundle_id
    } else if let Some(prefix) = bundle_pattern.strip_suffix('*') {
        format!("{prefix}allwright-agent.xctrunner")
    } else {
        bundle_pattern.to_string()
    };
    let permitted = bundle_pattern == bundle_id
        || bundle_pattern
            .strip_suffix('*')
            .is_some_and(|prefix| bundle_id.starts_with(prefix));
    if permitted {
        Ok(bundle_id)
    } else {
        Err(format!(
            "iOS agent bundle id `{bundle_id}` is not permitted by provisioning profile pattern `{bundle_pattern}`"
        ))
    }
}

fn materialize_profile_entitlements(
    entitlements: &mut plist::Dictionary,
    bundle_id: &str,
) -> Result<(), String> {
    let application_identifier = entitlements
        .get("application-identifier")
        .and_then(plist::Value::as_string)
        .ok_or_else(|| "the provisioning profile has no application-identifier".to_string())?;
    let (team_prefix, _) = application_identifier.split_once('.').ok_or_else(|| {
        "the provisioning profile application-identifier is malformed".to_string()
    })?;
    let concrete_application_identifier = format!("{team_prefix}.{bundle_id}");
    entitlements.insert(
        "application-identifier".to_string(),
        plist::Value::String(concrete_application_identifier.clone()),
    );
    if let Some(groups) = entitlements
        .get_mut("keychain-access-groups")
        .and_then(plist::Value::as_array_mut)
    {
        for group in groups {
            if group.as_string().is_some_and(|value| value.ends_with('*')) {
                *group = plist::Value::String(concrete_application_identifier.clone());
            }
        }
    }
    Ok(())
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

fn prepare_signed_device_agent(device: &PhysicalDevice) -> Result<PathBuf, String> {
    let source_root = bundled_agent_root(IosTargetKind::Device)?;
    let _ = bundled_agent_runner(IosTargetKind::Device)?;
    let source_xctestrun = bundled_agent_xctestrun(IosTargetKind::Device)?;
    let identity = signing_identity()?;
    let (profile_path, profile) = provisioning_profile(&device.udid)?;
    let mut entitlements = profile_entitlements(&profile)?;
    let bundle_id = signed_runner_bundle_id(&entitlements)?;
    materialize_profile_entitlements(&mut entitlements, &bundle_id)?;
    let signed_root = plugin_install_root()?
        .join("agent-signed")
        .join(&device.udid);
    if signed_root.exists() {
        fs::remove_dir_all(&signed_root)
            .map_err(|error| format!("failed to refresh {}: {error}", signed_root.display()))?;
    }
    fs::create_dir_all(&signed_root)
        .map_err(|error| format!("failed to create {}: {error}", signed_root.display()))?;
    run_checked(
        Command::new("ditto").arg(&source_root).arg(&signed_root),
        "copy the bundled physical-device runner",
    )?;
    let xctestrun_name = source_xctestrun
        .file_name()
        .ok_or_else(|| "bundled device xctestrun has no file name".to_string())?;
    let xctestrun = signed_root.join(xctestrun_name);
    let runner = signed_root
        .join(IosTargetKind::Device.products_directory())
        .join("AllwrightAgentUITests-Runner.app");
    let test_bundle = runner.join("PlugIns/AllwrightAgentUITests.xctest");
    fs::copy(&profile_path, runner.join("embedded.mobileprovision")).map_err(|error| {
        format!(
            "failed to embed provisioning profile {}: {error}",
            profile_path.display()
        )
    })?;
    run_checked(
        Command::new("plutil")
            .args(["-replace", "CFBundleIdentifier", "-string"])
            .arg(&bundle_id)
            .arg(runner.join("Info.plist")),
        "update the signed runner bundle identifier",
    )?;
    run_checked(
        Command::new("plutil")
            .args([
                "-replace",
                "TestConfigurations.0.TestTargets.0.TestHostBundleIdentifier",
                "-string",
            ])
            .arg(&bundle_id)
            .arg(&xctestrun),
        "update the device xctestrun bundle identifier",
    )?;
    run_checked(
        Command::new("plutil")
            .args([
                "-insert",
                "TestConfigurations.0.TestTargets.0.EnvironmentVariables.ALLWRIGHT_IOS_DEVICE_ID",
                "-string",
            ])
            .arg(&device.udid)
            .arg(&xctestrun),
        "record the physical device id in the device xctestrun",
    )?;
    let entitlements_path = signed_root.join("AllwrightAgent.entitlements.plist");
    plist::Value::Dictionary(entitlements)
        .to_file_xml(&entitlements_path)
        .map_err(|error| format!("failed to write signing entitlements: {error}"))?;
    run_checked(
        Command::new("codesign")
            .args(["--force", "--sign"])
            .arg(&identity)
            .args(["--timestamp=none"])
            .arg(&test_bundle),
        "sign the Allwright XCTest bundle",
    )?;
    run_checked(
        Command::new("codesign")
            .args(["--force", "--sign"])
            .arg(&identity)
            .args(["--timestamp=none", "--entitlements"])
            .arg(&entitlements_path)
            .arg(&runner),
        "sign the Allwright physical-device runner",
    )?;
    run_checked(
        Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&runner),
        "verify the signed Allwright physical-device runner",
    )?;
    Ok(xctestrun)
}

#[cfg(unix)]
fn usbmux_request(
    stream: &mut UnixStream,
    payload: &plist::Value,
    tag: u32,
) -> Result<plist::Value, String> {
    let mut body = Vec::new();
    payload
        .to_writer_xml(&mut body)
        .map_err(|error| format!("failed to encode usbmuxd request: {error}"))?;
    let length =
        u32::try_from(body.len() + 16).map_err(|_| "usbmuxd request is too large".to_string())?;
    for value in [length, 1, 8, tag] {
        stream
            .write_all(&value.to_le_bytes())
            .map_err(|error| format!("failed to write usbmuxd header: {error}"))?;
    }
    stream
        .write_all(&body)
        .map_err(|error| format!("failed to write usbmuxd request: {error}"))?;
    let mut header = [0_u8; 16];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("failed to read usbmuxd response header: {error}"))?;
    let response_length = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
    if response_length < 16 || response_length > 16 * 1024 * 1024 {
        return Err(format!(
            "usbmuxd returned invalid frame length {response_length}"
        ));
    }
    let mut response = vec![0_u8; response_length - 16];
    stream
        .read_exact(&mut response)
        .map_err(|error| format!("failed to read usbmuxd response: {error}"))?;
    plist::Value::from_reader(std::io::Cursor::new(response))
        .map_err(|error| format!("usbmuxd returned an invalid plist: {error}"))
}

#[cfg(unix)]
fn usbmux_dictionary(message_type: &str) -> plist::Dictionary {
    let mut dictionary = plist::Dictionary::new();
    dictionary.insert(
        "BundleID".to_string(),
        plist::Value::String("dev.allwright.mobile-ios".to_string()),
    );
    dictionary.insert(
        "ClientVersionString".to_string(),
        plist::Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    dictionary.insert(
        "MessageType".to_string(),
        plist::Value::String(message_type.to_string()),
    );
    dictionary.insert(
        "ProgName".to_string(),
        plist::Value::String("allwright".to_string()),
    );
    dictionary.insert(
        "kLibUSBMuxVersion".to_string(),
        plist::Value::Integer(3.into()),
    );
    dictionary
}

#[cfg(unix)]
fn usbmux_device_id(udid: &str) -> Result<u64, String> {
    let mut stream = UnixStream::connect("/var/run/usbmuxd")
        .map_err(|error| format!("could not connect to usbmuxd: {error}"))?;
    let response = usbmux_request(
        &mut stream,
        &plist::Value::Dictionary(usbmux_dictionary("ListDevices")),
        1,
    )?;
    response
        .as_dictionary()
        .and_then(|dictionary| dictionary.get("DeviceList"))
        .and_then(plist::Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|device| {
            let dictionary = device.as_dictionary()?;
            let serial = dictionary
                .get("Properties")?
                .as_dictionary()?
                .get("SerialNumber")?
                .as_string()?;
            (serial == udid)
                .then(|| dictionary.get("DeviceID")?.as_unsigned_integer())
                .flatten()
        })
        .ok_or_else(|| {
            format!(
                "usbmuxd cannot see physical device `{udid}` over USB or a paired network transport"
            )
        })
}

#[cfg(unix)]
fn usbmux_connect(device_id: u64, port: u16) -> Result<UnixStream, String> {
    let mut stream = UnixStream::connect("/var/run/usbmuxd")
        .map_err(|error| format!("could not connect to usbmuxd: {error}"))?;
    let mut request = usbmux_dictionary("Connect");
    request.insert(
        "DeviceID".to_string(),
        plist::Value::Integer(device_id.into()),
    );
    request.insert(
        "PortNumber".to_string(),
        plist::Value::Integer(u64::from(port.to_be()).into()),
    );
    let response = usbmux_request(&mut stream, &plist::Value::Dictionary(request), 2)?;
    let result = response
        .as_dictionary()
        .and_then(|dictionary| dictionary.get("Number"))
        .and_then(plist::Value::as_unsigned_integer)
        .unwrap_or(u64::MAX);
    if result == 0 {
        Ok(stream)
    } else {
        Err(format!(
            "usbmuxd rejected port forwarding with result {result}"
        ))
    }
}

#[cfg(unix)]
fn start_usbmux_forwarder(udid: &str, remote_port: u16) -> Result<String, String> {
    let device_id = usbmux_device_id(udid)?;
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("could not bind the local iOS agent port: {error}"))?;
    let local_port = listener
        .local_addr()
        .map_err(|error| format!("could not inspect the local iOS agent port: {error}"))?
        .port();
    thread::Builder::new()
        .name(format!("allwright-usbmux-{udid}"))
        .spawn(move || {
            for incoming in listener.incoming() {
                let Ok(mut local) = incoming else { continue };
                thread::spawn(move || {
                    let Ok(mut device) = usbmux_connect(device_id, remote_port) else {
                        return;
                    };
                    let Ok(mut local_read) = local.try_clone() else {
                        return;
                    };
                    let Ok(mut device_write) = device.try_clone() else {
                        return;
                    };
                    let upload = thread::spawn(move || {
                        let _ = std::io::copy(&mut local_read, &mut device_write);
                        let _ = device_write.shutdown(Shutdown::Write);
                    });
                    let _ = std::io::copy(&mut device, &mut local);
                    let _ = local.shutdown(Shutdown::Write);
                    let _ = upload.join();
                });
            }
        })
        .map_err(|error| format!("could not start usbmuxd forwarding: {error}"))?;
    Ok(format!("http://127.0.0.1:{local_port}"))
}

fn start_physical_device_agent(requested: &str) -> Result<(String, String, String), String> {
    let device = physical_device(requested)?;
    let xctestrun = prepare_signed_device_agent(&device)?;
    #[cfg(unix)]
    let endpoint = start_usbmux_forwarder(&device.udid, 8100)?;
    #[cfg(not(unix))]
    let endpoint = return Err("physical iOS devices require macOS usbmuxd".to_string());
    stop_owned_agent()?;
    let child = Command::new("xcodebuild")
        .arg("test-without-building")
        .arg("-xctestrun")
        .arg(&xctestrun)
        .arg("-destination")
        .arg(format!("platform=iOS,id={}", device.udid))
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
                "could not launch signed iOS agent {}: {error}",
                xctestrun.display()
            )
        })?;
    store_agent_process(child)?;
    Ok((device.identifier, device.name, endpoint))
}

fn status_matches_request(status: &Value, requested: Option<&str>) -> bool {
    let Some(requested) = requested else {
        return true;
    };
    status.get("device_id").and_then(Value::as_str) == Some(requested)
        || status
            .get("device_name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.eq_ignore_ascii_case(requested))
}

fn connect_agent(
    options: &ConnectOptions,
    configured_endpoint: &str,
) -> Result<(Value, String), String> {
    match invoke_agent(
        configured_endpoint,
        &AgentRequest::new("status"),
        options.timeout_ms,
    ) {
        Ok(status) if status_matches_request(&status, options.device.as_deref()) => {
            return Ok((status, configured_endpoint.to_string()));
        }
        Ok(_) if configured_endpoint != DEFAULT_AGENT_ENDPOINT => {
            return Err(format!(
                "iOS agent at `{configured_endpoint}` is not attached to requested device `{}`",
                options.device.as_deref().unwrap_or("<unspecified>")
            ));
        }
        Err(initial_error) if configured_endpoint != DEFAULT_AGENT_ENDPOINT => {
            return Err(initial_error);
        }
        Err(_) => {}
        Ok(_) => {}
    }

    let (_, device_name, endpoint) = match available_simulator(options.device.as_deref()) {
        Ok(_) => start_simulator_agent(options.device.as_deref())?,
        Err(simulator_error) if options.device.is_some() => {
            start_physical_device_agent(options.device.as_deref().unwrap()).map_err(|device_error| {
                format!(
                    "requested iOS target matched neither an available Simulator nor a connected physical device; Simulator: {simulator_error}; device: {device_error}"
                )
            })?
        }
        Err(error) => return Err(error),
    };
    let deadline =
        Instant::now() + Duration::from_millis(options.timeout_ms.unwrap_or(30_000).max(1) as u64);
    let mut last_error = String::new();
    while Instant::now() < deadline {
        match invoke_agent(&endpoint, &AgentRequest::new("status"), Some(1_000)) {
            Ok(status) if status_matches_request(&status, options.device.as_deref()) => {
                return Ok((status, endpoint));
            }
            Ok(_) => last_error = "agent reported a different device".to_string(),
            Err(error) => last_error = error,
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "bundled XCUITest agent did not become ready on `{device_name}`: {last_error}"
    ))
}

pub fn connect(options: &ConnectOptions) -> Result<MobileConnectInfo, String> {
    if options.platform != MobilePlatform::Ios {
        return Err("mobile-ios connect only supports the iOS platform".to_string());
    }
    let configured_endpoint = endpoint_for_connect(options);
    let (status, endpoint) = connect_agent(options, &configured_endpoint)?;
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
        match browser_session.device.connection_kind {
            DeviceConnectionKind::Emulator => {
                install_simulator_app(&browser_session.device.device_id, &app.path)?;
            }
            _ => install_device_app(&browser_session.device.device_id, &app.path)?,
        }
    }
    let endpoint = endpoint_from_session(browser_session)?;
    let mut request = AgentRequest::new("launch");
    request.bundle_id = Some(bundle_id);
    request.terminate_running = Some(options.stop_before_launch);
    request.timeout_ms = options.timeout_ms;
    let result = invoke_agent(endpoint, &request, options.timeout_ms)?;
    let agent_session_id = result_string(&result, "session_id")?.to_string();
    let page_session = MobilePageSessionHandle {
        page_id: agent_session_id,
        package_name: Some(bundle_id.to_string()),
        activity_name: None,
        webview_context: Some("xctest-accessibility".to_string()),
    };
    ios_current_pages()
        .lock()
        .map_err(|_| "iOS current app registry is unavailable".to_string())?
        .insert(
            browser_session.automation.session_id.clone(),
            page_session.clone(),
        );
    Ok(MobilePageInfo {
        note: format!("launched iOS app `{bundle_id}` through XCUITest"),
        page_session,
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

fn install_device_app(device_id: &str, app: &Path) -> Result<(), String> {
    let output = Command::new("xcrun")
        .args([
            "devicectl",
            "device",
            "install",
            "app",
            "--device",
            device_id,
        ])
        .arg(app)
        .output()
        .map_err(|error| format!("could not run `xcrun devicectl device install app`: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to install physical-device iOS app {}: {}. The app must contain an arm64 device binary signed for this device",
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

fn ios_hooks() -> &'static Mutex<std::collections::HashMap<String, IosHookState>> {
    IOS_HOOKS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn ios_choosers() -> &'static Mutex<std::collections::HashMap<String, IosChooserState>> {
    IOS_CHOOSERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn ios_downloads() -> &'static Mutex<std::collections::HashMap<String, IosDownloadState>> {
    IOS_DOWNLOADS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn ios_current_pages() -> &'static Mutex<std::collections::HashMap<String, MobilePageSessionHandle>>
{
    IOS_CURRENT_PAGES.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn simulator_app_container(device_id: &str, bundle_id: &str) -> Result<PathBuf, String> {
    let output = Command::new("xcrun")
        .args(["simctl", "get_app_container", device_id, bundle_id, "data"])
        .output()
        .map_err(|error| format!("could not resolve iOS Simulator app container: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not resolve iOS Simulator container for `{bundle_id}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

fn collect_host_files(
    root: &Path,
    domain_bundle_id: &str,
    files: &mut std::collections::HashMap<String, IosFileInfo>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                let metadata = fs::metadata(&path)
                    .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map(|value| value.as_nanos())
                    .unwrap_or_default();
                let key = path.to_string_lossy().to_string();
                files.insert(
                    key.clone(),
                    IosFileInfo {
                        path: key,
                        size: metadata.len(),
                        signature: format!("{}:{modified}", metadata.len()),
                        domain_bundle_id: domain_bundle_id.to_string(),
                    },
                );
            }
        }
    }
    Ok(())
}

fn collect_device_file_objects(
    value: &Value,
    bundle_id: &str,
    files: &mut std::collections::HashMap<String, IosFileInfo>,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_device_file_objects(value, bundle_id, files);
            }
        }
        Value::Object(object) => {
            let path = ["relativePath", "path", "filePath", "name"]
                .iter()
                .find_map(|key| object.get(*key).and_then(Value::as_str));
            let size = ["size", "fileSize"]
                .iter()
                .find_map(|key| object.get(*key).and_then(Value::as_u64));
            if let (Some(path), Some(size)) = (path, size)
                && !path.ends_with('/')
            {
                let modified = object
                    .get("modificationDate")
                    .or_else(|| object.get("modifiedDate"))
                    .map(Value::to_string)
                    .unwrap_or_default();
                files.insert(
                    format!("{bundle_id}:{path}"),
                    IosFileInfo {
                        path: path.to_string(),
                        size,
                        signature: format!("{size}:{modified}"),
                        domain_bundle_id: bundle_id.to_string(),
                    },
                );
            }
            for value in object.values() {
                collect_device_file_objects(value, bundle_id, files);
            }
        }
        _ => {}
    }
}

fn physical_device_files(
    device_id: &str,
    bundle_id: &str,
) -> Result<std::collections::HashMap<String, IosFileInfo>, String> {
    let output_path =
        env::temp_dir().join(format!("allwright-ios-files-{}.json", unique_id("list")));
    let output = Command::new("xcrun")
        .args([
            "devicectl",
            "device",
            "info",
            "files",
            "--device",
            device_id,
            "--domain-type",
            "appDataContainer",
            "--domain-identifier",
            bundle_id,
            "--subdirectory",
            "Documents",
            "--json-output",
        ])
        .arg(&output_path)
        .output()
        .map_err(|error| format!("could not list physical-device files: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(format!(
            "could not list files for `{bundle_id}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let bytes = fs::read(&output_path)
        .map_err(|error| format!("could not read devicectl file listing: {error}"))?;
    let _ = fs::remove_file(&output_path);
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not decode devicectl file listing: {error}"))?;
    let mut files = std::collections::HashMap::new();
    collect_device_file_objects(&value, bundle_id, &mut files);
    Ok(files)
}

fn ios_visible_files(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
) -> Result<std::collections::HashMap<String, IosFileInfo>, String> {
    let app_bundle = page_session
        .package_name
        .as_deref()
        .ok_or_else(|| "iOS app session has no bundle identifier".to_string())?;
    let bundles = [app_bundle, "com.apple.DocumentsApp"];
    let mut files = std::collections::HashMap::new();
    for bundle_id in bundles {
        match browser_session.device.connection_kind {
            DeviceConnectionKind::Emulator => {
                if let Ok(container) =
                    simulator_app_container(&browser_session.device.device_id, bundle_id)
                {
                    collect_host_files(&container.join("Documents"), bundle_id, &mut files)?;
                    collect_host_files(
                        &container.join("Library/Downloads"),
                        bundle_id,
                        &mut files,
                    )?;
                }
            }
            _ => {
                if let Ok(found) =
                    physical_device_files(&browser_session.device.device_id, bundle_id)
                {
                    files.extend(found);
                }
            }
        }
    }
    Ok(files)
}

fn register_ios_hook(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    hook_type: MobileHookType,
) -> Result<MobileHookRegistration, String> {
    validate_ios_session(browser_session)?;
    if page_session.package_name.is_none() {
        return Err("iOS hooks require a launched app".to_string());
    }
    let id = unique_id("ios-hook");
    let state = match hook_type {
        MobileHookType::FileChooser => IosHookState::FileChooser {
            device_id: browser_session.device.device_id.clone(),
            page: page_session.clone(),
        },
        MobileHookType::Download => IosHookState::Download {
            device_id: browser_session.device.device_id.clone(),
            page: page_session.clone(),
            existing_files: ios_visible_files(browser_session, page_session)?,
        },
    };
    ios_hooks()
        .lock()
        .map_err(|_| "iOS hook registry is unavailable".to_string())?
        .insert(id.clone(), state);
    Ok(MobileHookRegistration { opaque_state: id })
}

fn poll_ios_hook(
    browser_session: &MobileBrowserSessionHandle,
    registration: &MobileHookRegistration,
) -> Result<MobileHookResult, String> {
    let id = &registration.opaque_state;
    let state = ios_hooks()
        .lock()
        .map_err(|_| "iOS hook registry is unavailable".to_string())?
        .get(id)
        .cloned()
        .ok_or_else(|| "iOS hook registration is no longer available".to_string())?;
    match state {
        IosHookState::FileChooser { device_id, page } => {
            if device_id != browser_session.device.device_id {
                return Err("iOS hook belongs to a different device".to_string());
            }
            let mut request = AgentRequest::new("file_chooser_status");
            request.session_id = Some(&page.page_id);
            let result = invoke_agent(
                endpoint_from_session(browser_session)?,
                &request,
                Some(1_000),
            )?;
            if !result
                .get("opened")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Err(
                    "iOS file chooser hook is still waiting for a document picker".to_string(),
                );
            }
            let chooser_id = unique_id("ios-chooser");
            ios_choosers()
                .lock()
                .map_err(|_| "iOS chooser registry is unavailable".to_string())?
                .insert(chooser_id.clone(), IosChooserState { device_id, page });
            ios_hooks()
                .lock()
                .map_err(|_| "iOS hook registry is unavailable".to_string())?
                .remove(id);
            Ok(MobileHookResult::FileChooser(MobileFileChooserInfo {
                file_chooser_id: chooser_id,
                is_multiple: result
                    .get("multiple")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                note: "iOS document picker opened".to_string(),
            }))
        }
        IosHookState::Download {
            device_id,
            page,
            existing_files,
        } => {
            if device_id != browser_session.device.device_id {
                return Err("iOS hook belongs to a different device".to_string());
            }
            let current = ios_visible_files(browser_session, &page)?;
            let Some(file) = current
                .values()
                .find(|file| {
                    existing_files
                        .values()
                        .find(|before| {
                            before.path == file.path
                                && before.domain_bundle_id == file.domain_bundle_id
                        })
                        .map(|before| &before.signature)
                        != Some(&file.signature)
                })
                .cloned()
            else {
                return Err("iOS download hook is still waiting for a downloaded file".to_string());
            };
            let download_id = unique_id("ios-download");
            let suggested_filename = Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("download")
                .to_string();
            ios_downloads()
                .lock()
                .map_err(|_| "iOS download registry is unavailable".to_string())?
                .insert(
                    download_id.clone(),
                    IosDownloadState {
                        device_id,
                        file,
                        observed_size: None,
                    },
                );
            ios_hooks()
                .lock()
                .map_err(|_| "iOS hook registry is unavailable".to_string())?
                .remove(id);
            Ok(MobileHookResult::Download(MobileDownloadInfo {
                download_id,
                suggested_filename,
                note: "iOS app download appeared in the Files container".to_string(),
            }))
        }
    }
}

fn stage_ios_chooser_file(
    browser_session: &MobileBrowserSessionHandle,
    page: &MobilePageSessionHandle,
    source: &Path,
) -> Result<String, String> {
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "iOS chooser upload requires a valid file name".to_string())?;
    let app_bundle = page
        .package_name
        .as_deref()
        .ok_or_else(|| "iOS app session has no bundle identifier".to_string())?;
    let candidates = ["com.apple.DocumentsApp", app_bundle];
    match browser_session.device.connection_kind {
        DeviceConnectionKind::Emulator => {
            let mut last_error = None;
            for bundle_id in candidates {
                match simulator_app_container(&browser_session.device.device_id, bundle_id) {
                    Ok(container) => {
                        let directory = container.join("Documents/Allwright");
                        fs::create_dir_all(&directory).map_err(|error| {
                            format!("failed to create iOS chooser staging directory: {error}")
                        })?;
                        fs::copy(source, directory.join(filename)).map_err(|error| {
                            format!(
                                "failed to stage `{filename}` for the iOS document picker: {error}"
                            )
                        })?;
                        return Ok(filename.to_string());
                    }
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_error.unwrap_or_else(|| "no writable iOS Files container found".to_string()))
        }
        _ => {
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "copy",
                    "to",
                    "--device",
                    &browser_session.device.device_id,
                    "--source",
                ])
                .arg(source)
                .args([
                    "--destination",
                    "Documents/Allwright",
                    "--domain-type",
                    "appDataContainer",
                    "--domain-identifier",
                    app_bundle,
                ])
                .output()
                .map_err(|error| format!("could not stage file on physical iOS device: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "could not stage `{filename}` on the physical iOS device: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            Ok(filename.to_string())
        }
    }
}

fn set_ios_file_chooser_files(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    file_chooser_id: &str,
    files: &[String],
) -> Result<MobileFileChooserFilesSetInfo, String> {
    if files.is_empty() {
        return Err("iOS file chooser requires at least one file".to_string());
    }
    let chooser = ios_choosers()
        .lock()
        .map_err(|_| "iOS chooser registry is unavailable".to_string())?
        .get(file_chooser_id)
        .cloned()
        .ok_or_else(|| format!("unknown iOS file chooser {file_chooser_id}"))?;
    if chooser.device_id != browser_session.device.device_id
        || chooser.page.page_id != page_session.page_id
    {
        return Err("iOS file chooser belongs to a different app session".to_string());
    }
    let mut names = Vec::with_capacity(files.len());
    for file in files {
        let source = fs::canonicalize(file)
            .map_err(|error| format!("resolve staged upload {file}: {error}"))?;
        names.push(stage_ios_chooser_file(
            browser_session,
            page_session,
            &source,
        )?);
    }
    let mut request = AgentRequest::new("select_file");
    request.session_id = Some(&page_session.page_id);
    request.files = Some(names.iter().map(String::as_str).collect());
    request.timeout_ms = Some(10_000);
    invoke_agent(
        endpoint_from_session(browser_session)?,
        &request,
        request.timeout_ms,
    )?;
    ios_choosers()
        .lock()
        .map_err(|_| "iOS chooser registry is unavailable".to_string())?
        .remove(file_chooser_id);
    Ok(MobileFileChooserFilesSetInfo {
        file_chooser_id: file_chooser_id.to_string(),
        files: files.to_vec(),
        note: format!(
            "staged and selected {} file(s) in the iOS document picker",
            files.len()
        ),
    })
}

fn save_ios_download(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    download_id: &str,
    path: &str,
) -> Result<MobileDownloadSavedInfo, String> {
    if page_session.package_name.is_none() {
        return Err("iOS download requires a launched app".to_string());
    }
    let mut downloads = ios_downloads()
        .lock()
        .map_err(|_| "iOS download registry is unavailable".to_string())?;
    let download = downloads
        .get_mut(download_id)
        .ok_or_else(|| format!("unknown iOS download {download_id}"))?;
    if download.device_id != browser_session.device.device_id {
        return Err("iOS download belongs to a different device".to_string());
    }
    let current = ios_visible_files(browser_session, page_session)?
        .values()
        .find(|file| {
            file.path == download.file.path
                && file.domain_bundle_id == download.file.domain_bundle_id
        })
        .cloned()
        .ok_or_else(|| "iOS download disappeared before it could be saved".to_string())?;
    if download.observed_size != Some(current.size) {
        download.observed_size = Some(current.size);
        return Err("iOS download is still in progress".to_string());
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create iOS download destination: {error}"))?;
    }
    match browser_session.device.connection_kind {
        DeviceConnectionKind::Emulator => {
            fs::copy(&current.path, path)
                .map_err(|error| format!("copy iOS Simulator download: {error}"))?;
        }
        _ => {
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "copy",
                    "from",
                    "--device",
                    &browser_session.device.device_id,
                    "--source",
                    &current.path,
                    "--destination",
                    path,
                    "--domain-type",
                    "appDataContainer",
                    "--domain-identifier",
                    &current.domain_bundle_id,
                ])
                .output()
                .map_err(|error| format!("could not copy physical-device download: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "could not copy iOS download: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }
    }
    let suggested_filename = Path::new(&current.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download")
        .to_string();
    let result = MobileDownloadSavedInfo {
        download_id: download_id.to_string(),
        path: fs::canonicalize(path)
            .map_err(|error| format!("resolve saved iOS download: {error}"))?
            .to_string_lossy()
            .to_string(),
        suggested_filename,
        size: current.size,
        note: "copied completed iOS download to server staging".to_string(),
    };
    downloads.remove(download_id);
    Ok(result)
}

fn navigate_ios_app(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    url: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileNavigationInfo, String> {
    validate_ios_session(browser_session)?;
    if !url.contains(':') {
        return Err("iOS deep link must be an absolute URL with a scheme".to_string());
    }
    let mut command = Command::new("xcrun");
    match browser_session.device.connection_kind {
        DeviceConnectionKind::Emulator => {
            command.args(["simctl", "openurl", &browser_session.device.device_id, url]);
        }
        _ => {
            let bundle_id = page_session
                .package_name
                .as_deref()
                .ok_or_else(|| "iOS deep link requires a launched app bundle".to_string())?;
            command.args([
                "devicectl",
                "device",
                "process",
                "launch",
                "--device",
                &browser_session.device.device_id,
                "--payload-url",
                url,
                "--timeout",
                &(timeout_ms.unwrap_or(10_000) as f64 / 1_000.0).to_string(),
                bundle_id,
            ]);
        }
    }
    let output = command
        .output()
        .map_err(|error| format!("could not open iOS deep link `{url}`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not open iOS deep link `{url}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(MobileNavigationInfo {
        url: url.to_string(),
        note: "opened iOS deep link through the platform URL router".to_string(),
    })
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
        MobileCommand::RegisterHook {
            browser_session,
            page_session,
            hook_type,
        } => register_ios_hook(&browser_session, &page_session, hook_type)
            .map(MobileCommandResult::RegisterHook),
        MobileCommand::PollHook {
            browser_session,
            registration,
        } => poll_ios_hook(&browser_session, &registration).map(MobileCommandResult::PollHook),
        MobileCommand::SetFileChooserFiles {
            browser_session,
            page_session,
            file_chooser_id,
            files,
        } => set_ios_file_chooser_files(&browser_session, &page_session, &file_chooser_id, &files)
            .map(MobileCommandResult::SetFileChooserFiles),
        MobileCommand::SaveDownload {
            browser_session,
            page_session,
            download_id,
            path,
        } => save_ios_download(&browser_session, &page_session, &download_id, &path)
            .map(MobileCommandResult::SaveDownload),
        MobileCommand::Connect(options) => connect(&options).map(MobileCommandResult::Connect),
        MobileCommand::LaunchApp {
            browser_session,
            options,
        } => launch_app(&browser_session, &options).map(MobileCommandResult::LaunchApp),
        MobileCommand::OpenPage { browser_session } => {
            let page_session = ios_current_pages()
                .lock()
                .map_err(|_| "iOS current app registry is unavailable".to_string())?
                .get(&browser_session.automation.session_id)
                .cloned()
                .ok_or_else(|| {
                    "iOS has no launched app context; call launch_app first".to_string()
                })?;
            Ok(MobileCommandResult::OpenPage(MobilePageInfo {
                note: "attached to the current iOS app context".to_string(),
                page_session,
            }))
        }
        MobileCommand::ClosePage {
            browser_session,
            page_session,
        } => {
            validate_ios_session(&browser_session)?;
            let mut request = AgentRequest::new("close");
            request.session_id = Some(&page_session.page_id);
            invoke_agent(endpoint_from_session(&browser_session)?, &request, None)?;
            ios_current_pages()
                .lock()
                .map_err(|_| "iOS current app registry is unavailable".to_string())?
                .remove(&browser_session.automation.session_id);
            Ok(MobileCommandResult::ClosePage)
        }
        MobileCommand::NavigateApp {
            browser_session,
            page_session,
            url,
            timeout_ms,
        } => navigate_ios_app(&browser_session, &page_session, &url, timeout_ms)
            .map(MobileCommandResult::NavigateApp),
        MobileCommand::Capture {
            browser_session,
            page_session,
            kind,
            selector,
            attribute_name,
            timeout_ms,
        } => {
            let selector = normalized_selector(&selector)?;
            let mut request = action_request("capture", &page_session, &selector, timeout_ms);
            request.kind = Some(&kind);
            request.attribute_name = Some(&attribute_name);
            let result = invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                timeout_ms,
            )?;
            let capture: CaptureInfo = serde_json::from_value(result).map_err(|error| {
                format!("iOS agent returned an invalid capture result: {error}")
            })?;
            Ok(MobileCommandResult::Capture(capture))
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
            full_page,
        } => {
            let mut request = AgentRequest::new("screenshot");
            request.session_id = Some(&page_session.page_id);
            let screenshot_timeout = if full_page {
                Some(timeout_ms.unwrap_or(60_000))
            } else {
                timeout_ms
            };
            request.timeout_ms = screenshot_timeout;
            request.full_page = Some(full_page);
            let result = invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                screenshot_timeout,
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
            request.timeout_ms = Some(60_000);
            let result = invoke_agent(
                endpoint_from_session(&browser_session)?,
                &request,
                request.timeout_ms,
            )?;
            let json_snapshot = result_string(&result, "snapshot")?;
            let resolved_format = if format.is_empty() {
                "json"
            } else {
                format.as_str()
            };
            let snapshot = match resolved_format {
                "json" => json_snapshot.to_string(),
                "yaml" => {
                    let value: Value = serde_json::from_str(json_snapshot)
                        .map_err(|error| format!("invalid iOS accessibility snapshot: {error}"))?;
                    serde_yaml::to_string(&value)
                        .map_err(|error| format!("could not encode iOS YAML snapshot: {error}"))?
                }
                _ => return Err("accessibility snapshot format must be json or yaml".to_string()),
            };
            Ok(MobileCommandResult::AccessibilitySnapshot(
                AccessibilitySnapshotInfo {
                    snapshot,
                    format: resolved_format.to_string(),
                },
            ))
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

    #[test]
    fn materializes_wildcard_signing_entitlements() {
        let mut entitlements = plist::Dictionary::new();
        entitlements.insert(
            "application-identifier".to_string(),
            plist::Value::String("TEAM123.com.example.*".to_string()),
        );
        entitlements.insert(
            "keychain-access-groups".to_string(),
            plist::Value::Array(vec![plist::Value::String(
                "TEAM123.com.example.*".to_string(),
            )]),
        );

        materialize_profile_entitlements(
            &mut entitlements,
            "com.example.allwright-agent.xctrunner",
        )
        .unwrap();

        assert_eq!(
            entitlements
                .get("application-identifier")
                .and_then(plist::Value::as_string),
            Some("TEAM123.com.example.allwright-agent.xctrunner")
        );
        assert_eq!(
            entitlements
                .get("keychain-access-groups")
                .and_then(plist::Value::as_array)
                .and_then(|groups| groups.first())
                .and_then(plist::Value::as_string),
            Some("TEAM123.com.example.allwright-agent.xctrunner")
        );
    }

    #[test]
    fn matches_a_device_provisioning_profile_by_udid() {
        let mut profile = plist::Dictionary::new();
        profile.insert(
            "ProvisionedDevices".to_string(),
            plist::Value::Array(vec![plist::Value::String("device-123".to_string())]),
        );
        let profile = plist::Value::Dictionary(profile);

        assert!(profile_supports_device(&profile, "device-123"));
        assert!(!profile_supports_device(&profile, "device-456"));
    }

    #[test]
    fn automatic_profile_selection_avoids_unrelated_exact_app_ids() {
        let profile = |application_identifier: &str| {
            let mut entitlements = plist::Dictionary::new();
            entitlements.insert(
                "application-identifier".to_string(),
                plist::Value::String(application_identifier.to_string()),
            );
            let mut profile = plist::Dictionary::new();
            profile.insert(
                "Entitlements".to_string(),
                plist::Value::Dictionary(entitlements),
            );
            plist::Value::Dictionary(profile)
        };

        assert!(profile_is_safe_for_agent(&profile("TEAM123.com.example.*")));
        assert!(profile_is_safe_for_agent(&profile(
            "TEAM123.com.example.allwright-agent"
        )));
        assert!(!profile_is_safe_for_agent(&profile(
            "TEAM123.com.example.customer-app"
        )));
    }
}
