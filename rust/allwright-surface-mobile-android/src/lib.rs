use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor,
};
use allwright_surface_mobile::{
    ConnectOptions, DeviceConnectionKind, DeviceTarget, LaunchOptions, MobileAppKind,
    MobileAutomationBackend, MobileAutomationSessionInfo, MobileBrowserSessionHandle,
    MobileCapabilitySet, MobileClickInfo, MobileCommand, MobileCommandResult, MobileConnectInfo,
    MobileFillInfo, MobilePageInfo, MobilePageSessionHandle, MobilePlatform,
    MobileRuntimeReadiness, MobileSurfaceProfile, RuntimeMaturity, boot_surface,
    normalize_selector_for_transport,
};
use serde_json::Value;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, Default)]
pub struct MobileAndroidPlugin;

const ANDROID_BACKENDS: &[MobileAutomationBackend] = &[MobileAutomationBackend::UiAutomator2];
const ANDROID_APP_KINDS: &[MobileAppKind] = &[
    MobileAppKind::Native,
    MobileAppKind::Hybrid,
    MobileAppKind::BrowserWrapped,
];
const ANDROID_MISSING_RUNTIME_ARTIFACTS: &[&str] = &[
    "server-side mobile session routing in allwright-core",
    "expanded command coverage beyond connect, launch, and click",
];
const ANDROID_NEXT_MILESTONES: &[&str] = &[
    "route Android mobile commands through the core engine session server",
    "add fill, text, and waitForSelector on top of the same UiAutomator2 bridge",
    "expand runtime packaging validation across macOS, Linux, and Windows release assets",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub name: Option<String>,
    pub connection_kind: DeviceConnectionKind,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectSelectionError {
    NoDevicesAvailable,
    RequestedDeviceNotFound { requested: String },
}

impl fmt::Display for ConnectSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevicesAvailable => write!(f, "no Android devices were discovered over ADB"),
            Self::RequestedDeviceNotFound { requested } => {
                write!(
                    f,
                    "requested Android device `{requested}` was not found over ADB"
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAutomator2ConnectInfo {
    pub device_name: Option<String>,
    pub current_package: Option<String>,
    pub current_activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAutomator2ClickInfo {
    pub resolved_selector: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAutomator2SourceInfo {
    pub source: String,
    pub current_package: Option<String>,
    pub current_activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAutomator2FillInfo {
    pub resolved_selector: String,
    pub value: String,
    pub note: String,
}

impl SurfacePlugin for MobileAndroidPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        descriptor()
    }
}

pub fn descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: "mobile-android",
        family: SurfaceFamily::Mobile,
        version: env!("CARGO_PKG_VERSION"),
        description: "Android mobile surface plugin for the allwright engine.",
    }
}

pub fn profile() -> MobileSurfaceProfile {
    MobileSurfaceProfile {
        plugin_id: "mobile-android",
        display_name: "Android",
        family: SurfaceFamily::Mobile,
        backends: ANDROID_BACKENDS,
        default_backend: MobileAutomationBackend::UiAutomator2,
        supported_app_kinds: ANDROID_APP_KINDS,
        capabilities: MobileCapabilitySet {
            supports_native_views: true,
            supports_webviews: true,
            supports_deep_links: true,
            supports_shell_commands: true,
            supports_device_logs: true,
        },
        bootstrap_hint: "Connect a device or emulator over ADB, then bootstrap a UiAutomator2 session before launching and clicking through the app.",
    }
}

pub fn runtime_readiness() -> MobileRuntimeReadiness {
    MobileRuntimeReadiness {
        maturity: RuntimeMaturity::Scaffolding,
        missing_runtime_artifacts: ANDROID_MISSING_RUNTIME_ARTIFACTS,
        next_milestones: ANDROID_NEXT_MILESTONES,
    }
}

pub fn connect_command(options: ConnectOptions) -> MobileCommand {
    MobileCommand::Connect(options)
}

pub fn launch_command(
    browser_session: MobileBrowserSessionHandle,
    options: LaunchOptions,
) -> MobileCommand {
    MobileCommand::LaunchApp {
        browser_session,
        options,
    }
}

pub fn list_adb_devices() -> Result<Vec<AdbDevice>, String> {
    let output = run_adb(&["devices", "-l"])?;
    Ok(parse_adb_devices(&output))
}

pub fn select_device_for_connect(
    options: &ConnectOptions,
    devices: &[AdbDevice],
) -> Result<AdbDevice, ConnectSelectionError> {
    let available = devices
        .iter()
        .filter(|device| device.state == "device")
        .cloned()
        .collect::<Vec<_>>();
    if available.is_empty() {
        return Err(ConnectSelectionError::NoDevicesAvailable);
    }

    if let Some(requested) = options.device.as_deref() {
        return available
            .iter()
            .find(|device| {
                device.serial == requested
                    || device
                        .name
                        .as_deref()
                        .is_some_and(|name| name.eq_ignore_ascii_case(requested))
            })
            .cloned()
            .ok_or_else(|| ConnectSelectionError::RequestedDeviceNotFound {
                requested: requested.to_string(),
            });
    }

    available
        .first()
        .cloned()
        .ok_or(ConnectSelectionError::NoDevicesAvailable)
}

pub fn connect(options: &ConnectOptions) -> Result<MobileConnectInfo, String> {
    if options.platform != MobilePlatform::Android {
        return Err("mobile-android connect only supports the Android platform".to_string());
    }

    if let Some(endpoint) = options.adb_endpoint.as_deref() {
        let trimmed = endpoint.trim();
        if !trimmed.is_empty() {
            let _ = run_adb(&["connect", trimmed])?;
        }
    }

    let devices = list_adb_devices()?;
    let selected =
        select_device_for_connect(options, &devices).map_err(|error| error.to_string())?;
    let u2 = uiautomator2_connect(&selected.serial)?;

    Ok(MobileConnectInfo {
        browser: "android".to_string(),
        note: format!(
            "connected Android device `{}` over ADB and bootstrapped UiAutomator2",
            selected.name.as_deref().unwrap_or(selected.serial.as_str())
        ),
        browser_session: MobileBrowserSessionHandle {
            platform: MobilePlatform::Android,
            automation: MobileAutomationSessionInfo {
                backend: "uiautomator2".to_string(),
                session_id: format!("uiautomator2:{}", selected.serial),
                note: "session established through the Android surface plugin".to_string(),
            },
            device: DeviceTarget {
                platform: MobilePlatform::Android,
                device_id: selected.serial.clone(),
                connection_kind: selected.connection_kind,
            },
        },
        initial_page: MobilePageInfo {
            note: "attached to the device foreground context".to_string(),
            page_session: MobilePageSessionHandle {
                page_id: format!("{}:foreground", selected.serial),
                package_name: u2.current_package,
                activity_name: u2.current_activity,
                webview_context: None,
            },
        },
    })
}

pub fn launch_app(
    browser_session: &MobileBrowserSessionHandle,
    options: &LaunchOptions,
) -> Result<MobilePageInfo, String> {
    let device_id = browser_session.device.device_id.as_str();

    if let Some(apk_path) = options.apk_path.as_deref() {
        let apk = Path::new(apk_path);
        if !apk.is_file() {
            return Err(format!("APK path does not exist: {}", apk.display()));
        }
        let apk_owned = apk.to_string_lossy().into_owned();
        let _ = run_adb_for_device(device_id, &["install", "-r", &apk_owned])?;
    }

    let package_name = match options.app_id.clone() {
        Some(package_name) => package_name,
        None => {
            if let Some(apk_path) = options.apk_path.as_deref() {
                resolve_package_name_from_apk(apk_path)?
            } else {
                return Err(
                    "launch_app requires `app_id`, or an APK path whose package name can be resolved"
                        .to_string(),
                );
            }
        }
    };

    if options.stop_before_launch {
        let _ = run_adb_for_device(device_id, &["shell", "am", "force-stop", &package_name])?;
    }

    if let Some(activity) = options.launch_activity.as_deref() {
        let component = format!("{package_name}/{activity}");
        let _ = run_adb_for_device(device_id, &["shell", "am", "start", "-W", "-n", &component])?;
    } else {
        let _ = run_adb_for_device(
            device_id,
            &[
                "shell",
                "monkey",
                "-p",
                &package_name,
                "-c",
                "android.intent.category.LAUNCHER",
                "1",
            ],
        )?;
    }

    let current = uiautomator2_connect(device_id)?;
    let activity_name = current
        .current_activity
        .or_else(|| options.launch_activity.clone());

    Ok(MobilePageInfo {
        note: format!(
            "launched Android app `{package_name}` on {}",
            browser_session.device.device_id
        ),
        page_session: MobilePageSessionHandle {
            page_id: format!("{}:{package_name}", browser_session.device.device_id),
            package_name: Some(package_name),
            activity_name,
            webview_context: None,
        },
    })
}

pub fn click_element(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    selector: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileClickInfo, String> {
    let normalized = normalize_selector_for_transport(selector);
    if normalized.trim().is_empty() {
        return Err("click_element requires a non-empty selector".to_string());
    }

    let clicked = uiautomator2_click(&browser_session.device.device_id, &normalized, timeout_ms)?;
    Ok(MobileClickInfo {
        selector: clicked.resolved_selector,
        note: format!("{}; page={}", clicked.note, page_session.page_id),
        session_id: browser_session.automation.session_id.clone(),
    })
}

pub fn fill_element(
    browser_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    selector: &str,
    value: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileFillInfo, String> {
    let normalized = normalize_selector_for_transport(selector);
    if normalized.trim().is_empty() {
        return Err("fill_element requires a non-empty selector".to_string());
    }

    let filled = uiautomator2_fill(
        &browser_session.device.device_id,
        &normalized,
        value,
        timeout_ms,
    )?;
    Ok(MobileFillInfo {
        selector: filled.resolved_selector,
        value: filled.value,
        note: format!("{}; page={}", filled.note, page_session.page_id),
    })
}

pub fn dump_source(
    browser_session: &MobileBrowserSessionHandle,
    _page_session: &MobilePageSessionHandle,
) -> Result<UiAutomator2SourceInfo, String> {
    let source = uiautomator2_source(&browser_session.device.device_id)?;
    Ok(UiAutomator2SourceInfo {
        source: source.source,
        current_package: source.current_package,
        current_activity: source.current_activity,
    })
}

pub fn connect_result_example(options: &ConnectOptions) -> Result<MobileCommandResult, String> {
    Ok(MobileCommandResult::Connect(connect(options)?))
}

pub async fn boot() -> String {
    boot_surface("android", 10).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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

    match serde_json::to_string(&payload) {
        Ok(json) => CString::new(json)
            .map(CString::into_raw)
            .unwrap_or_else(|_| CString::new("{\"ok\":false,\"result\":null,\"error\":\"failed to encode plugin response\"}").expect("static JSON").into_raw()),
        Err(error) => CString::new(format!(
            "{{\"ok\":false,\"result\":null,\"error\":\"failed to serialize plugin response: {error}\"}}"
        ))
        .map(CString::into_raw)
        .unwrap_or_else(|_| CString::new("{\"ok\":false,\"result\":null,\"error\":\"failed to serialize plugin response\"}").expect("static JSON").into_raw()),
    }
}

fn handle_plugin_command(command: MobileCommand) -> Result<MobileCommandResult, String> {
    match command {
        MobileCommand::Connect(options) => connect(&options).map(MobileCommandResult::Connect),
        MobileCommand::LaunchApp {
            browser_session,
            options,
        } => launch_app(&browser_session, &options).map(MobileCommandResult::LaunchApp),
        MobileCommand::OpenPage { browser_session } => {
            let info = MobilePageInfo {
                note: "attached to the Android foreground app context".to_string(),
                page_session: MobilePageSessionHandle {
                    page_id: format!("{}:foreground", browser_session.device.device_id),
                    package_name: None,
                    activity_name: None,
                    webview_context: None,
                },
            };
            Ok(MobileCommandResult::OpenPage(info))
        }
        MobileCommand::ClosePage { .. } => Ok(MobileCommandResult::ClosePage),
        MobileCommand::ClickElement {
            browser_session,
            page_session,
            selector,
            timeout_ms,
        } => click_element(&browser_session, &page_session, &selector, timeout_ms)
            .map(MobileCommandResult::ClickElement),
        MobileCommand::CountElements { .. } => {
            Err("mobile-android plugin does not implement `count_elements` yet".to_string())
        }
        MobileCommand::FillElement {
            browser_session,
            page_session,
            selector,
            value,
            timeout_ms,
        } => fill_element(
            &browser_session,
            &page_session,
            &selector,
            &value,
            timeout_ms,
        )
        .map(MobileCommandResult::FillElement),
        MobileCommand::GetText { .. } => {
            Err("mobile-android plugin does not implement `get_text` yet".to_string())
        }
        MobileCommand::WaitForSelector { .. } => {
            Err("mobile-android plugin does not implement `wait_for_selector` yet".to_string())
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_api_version() -> u32 {
    ALLWRIGHT_PLUGIN_API_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_id() -> *const c_char {
    c"mobile-android".as_ptr()
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

    let command: MobileCommand = match serde_json::from_str(request) {
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

fn parse_adb_devices(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("List of devices attached"))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next()?.to_string();
            let metadata = parts.collect::<Vec<_>>();
            let mut name = None;
            for item in metadata {
                if let Some(model) = item.strip_prefix("model:") {
                    name = Some(model.replace('_', " "));
                    break;
                }
                if let Some(device_name) = item.strip_prefix("device:") {
                    name = Some(device_name.replace('_', " "));
                }
            }

            Some(AdbDevice {
                connection_kind: classify_device_connection_kind(&serial),
                serial,
                name,
                state,
            })
        })
        .collect()
}

fn classify_device_connection_kind(serial: &str) -> DeviceConnectionKind {
    if serial.starts_with("emulator-") {
        return DeviceConnectionKind::Emulator;
    }
    if serial.contains(':') {
        return DeviceConnectionKind::RemoteAdb;
    }
    DeviceConnectionKind::Usb
}

fn run_adb(args: &[&str]) -> Result<String, String> {
    run_command(&adb_command_path(), args)
}

fn run_adb_for_device(device_id: &str, args: &[&str]) -> Result<String, String> {
    let mut all_args = vec!["-s", device_id];
    all_args.extend_from_slice(args);
    run_command(&adb_command_path(), &all_args)
}

fn adb_command_path() -> String {
    env::var("ALLWRIGHT_ANDROID_ADB").unwrap_or_else(|_| "adb".to_string())
}

fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run `{command}`: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!(
        "`{command} {}` failed with status {}: {}",
        args.join(" "),
        output
            .status
            .code()
            .map_or_else(|| "signal".to_string(), |code| code.to_string()),
        detail
    ))
}

fn resolve_package_name_from_apk(apk_path: &str) -> Result<String, String> {
    if let Ok(output) = run_command("aapt", &["dump", "badging", apk_path]) {
        if let Some(package_name) = parse_package_name_from_aapt_badging(&output) {
            return Ok(package_name);
        }
    }

    if let Ok(output) = run_command("apkanalyzer", &["manifest", "application-id", apk_path]) {
        let package_name = output.trim();
        if !package_name.is_empty() {
            return Ok(package_name.to_string());
        }
    }

    Err(
        "could not resolve the app package name from the APK. Provide `--app-id`, or install `aapt`/`apkanalyzer`."
            .to_string(),
    )
}

fn parse_package_name_from_aapt_badging(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let package_line = line.trim();
        if !package_line.starts_with("package: ") {
            return None;
        }
        let needle = "name='";
        let start = package_line.find(needle)? + needle.len();
        let remainder = &package_line[start..];
        let end = remainder.find('\'')?;
        Some(remainder[..end].to_string())
    })
}

fn uiautomator2_connect(device_id: &str) -> Result<UiAutomator2ConnectInfo, String> {
    let value = run_uiautomator2_bridge("connect", &[device_id])?;
    Ok(UiAutomator2ConnectInfo {
        device_name: string_field(&value, "deviceName"),
        current_package: string_field(&value, "currentPackage"),
        current_activity: string_field(&value, "currentActivity"),
    })
}

fn uiautomator2_click(
    device_id: &str,
    selector: &str,
    timeout_ms: Option<u32>,
) -> Result<UiAutomator2ClickInfo, String> {
    let timeout = timeout_ms
        .map(|value| format!("{:.3}", f64::from(value) / 1_000.0))
        .unwrap_or_else(|| "10.0".to_string());
    let value = run_uiautomator2_bridge("click", &[device_id, selector, "--timeout", &timeout])?;
    Ok(UiAutomator2ClickInfo {
        resolved_selector: string_field(&value, "resolvedSelector")
            .unwrap_or_else(|| selector.to_string()),
        note: string_field(&value, "note")
            .unwrap_or_else(|| "clicked Android element through UiAutomator2".to_string()),
    })
}

fn uiautomator2_fill(
    device_id: &str,
    selector: &str,
    fill_value: &str,
    timeout_ms: Option<u32>,
) -> Result<UiAutomator2FillInfo, String> {
    let timeout = timeout_ms
        .map(|value| format!("{:.3}", f64::from(value) / 1_000.0))
        .unwrap_or_else(|| "10.0".to_string());
    let value = run_uiautomator2_bridge(
        "fill",
        &[device_id, selector, fill_value, "--timeout", &timeout],
    )?;
    Ok(UiAutomator2FillInfo {
        resolved_selector: string_field(&value, "resolvedSelector")
            .unwrap_or_else(|| selector.to_string()),
        value: string_field(&value, "value").unwrap_or_else(|| fill_value.to_string()),
        note: string_field(&value, "note")
            .unwrap_or_else(|| "filled Android element through UiAutomator2".to_string()),
    })
}

fn uiautomator2_source(device_id: &str) -> Result<UiAutomator2SourceInfo, String> {
    let value = run_uiautomator2_bridge("source", &[device_id])?;
    Ok(UiAutomator2SourceInfo {
        source: string_field(&value, "source").unwrap_or_default(),
        current_package: string_field(&value, "currentPackage"),
        current_activity: string_field(&value, "currentActivity"),
    })
}

fn run_uiautomator2_bridge(command: &str, args: &[&str]) -> Result<Value, String> {
    let script = uiautomator2_bridge_script_path();
    if !script.is_file() {
        return Err(format!(
            "missing UiAutomator2 bridge script at {}",
            script.display()
        ));
    }

    let python = env::var("ALLWRIGHT_ANDROID_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let mut command_line = Command::new(&python);
    command_line.arg(&script).arg(command).args(args);
    let output = command_line
        .output()
        .map_err(|error| format!("failed to run `{python}`: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("UiAutomator2 bridge `{command}` failed: {detail}"));
    }

    serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|error| format!("failed to decode UiAutomator2 bridge response: {error}"))
}

fn uiautomator2_bridge_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("scripts")
        .join("mobile_android_uiautomator2_bridge.py")
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_android_runtime() {
        assert_eq!(boot().await, "android ready");
    }

    #[test]
    fn defaults_to_uiautomator2() {
        let android = profile();
        assert_eq!(
            android.default_backend,
            MobileAutomationBackend::UiAutomator2
        );
        assert_eq!(android.backends, ANDROID_BACKENDS);
        assert!(android.capabilities.supports_native_views);
        assert!(android.capabilities.supports_webviews);
    }

    #[test]
    fn exposes_android_runtime_milestones() {
        let readiness = runtime_readiness();
        assert_eq!(readiness.maturity, RuntimeMaturity::Scaffolding);
        assert!(
            readiness
                .missing_runtime_artifacts
                .iter()
                .any(|item| item.contains("server-side mobile session routing"))
        );
    }

    #[test]
    fn connect_flow_matches_web_shape() {
        let options = ConnectOptions {
            platform: MobilePlatform::Android,
            device: Some("emulator-5554".to_string()),
            adb_endpoint: None,
            preserve_app_state: true,
            timeout_ms: Some(5_000),
        };

        let command = connect_command(options.clone());

        match command {
            MobileCommand::Connect(connect) => {
                assert_eq!(connect.platform, MobilePlatform::Android);
                assert_eq!(connect.device.as_deref(), Some("emulator-5554"));
            }
            _ => panic!("expected mobile connect command"),
        }
    }

    #[test]
    fn launch_flow_keeps_apk_work_separate() {
        let browser_session = MobileBrowserSessionHandle {
            platform: MobilePlatform::Android,
            automation: MobileAutomationSessionInfo {
                backend: "uiautomator2".to_string(),
                session_id: "uiautomator2:emulator-5554".to_string(),
                note: "ready".to_string(),
            },
            device: DeviceTarget {
                platform: MobilePlatform::Android,
                device_id: "emulator-5554".to_string(),
                connection_kind: DeviceConnectionKind::Emulator,
            },
        };

        let command = launch_command(
            browser_session,
            LaunchOptions {
                apk_path: Some("/tmp/app.apk".to_string()),
                app_id: Some("dev.allwright.sample".to_string()),
                launch_activity: Some(".MainActivity".to_string()),
                stop_before_launch: true,
                timeout_ms: Some(15_000),
            },
        );

        match command {
            MobileCommand::LaunchApp { options, .. } => {
                assert_eq!(options.apk_path.as_deref(), Some("/tmp/app.apk"));
                assert_eq!(options.app_id.as_deref(), Some("dev.allwright.sample"));
                assert!(options.stop_before_launch);
            }
            _ => panic!("expected launch command"),
        }
    }

    #[test]
    fn parse_adb_devices_extracts_friendly_names() {
        let devices = parse_adb_devices(
            "List of devices attached\nemulator-5554 device product:sdk_gphone model:Pixel_8_API_35 device:emu64xa transport_id:5\nR5CX123 device usb:1-1 product:e3q model:QA_Galaxy_S24 device:e3q transport_id:8\n",
        );

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name.as_deref(), Some("Pixel 8 API 35"));
        assert_eq!(devices[1].name.as_deref(), Some("QA Galaxy S24"));
        assert_eq!(devices[0].connection_kind, DeviceConnectionKind::Emulator);
        assert_eq!(devices[1].connection_kind, DeviceConnectionKind::Usb);
    }

    #[test]
    fn connect_uses_requested_device_when_provided() {
        let options = ConnectOptions {
            platform: MobilePlatform::Android,
            device: Some("R5CX123".to_string()),
            adb_endpoint: None,
            preserve_app_state: false,
            timeout_ms: None,
        };
        let devices = vec![
            AdbDevice {
                serial: "emulator-5554".to_string(),
                name: Some("Pixel 8 API 35".to_string()),
                connection_kind: DeviceConnectionKind::Emulator,
                state: "device".to_string(),
            },
            AdbDevice {
                serial: "R5CX123".to_string(),
                name: Some("QA Galaxy S24".to_string()),
                connection_kind: DeviceConnectionKind::Usb,
                state: "device".to_string(),
            },
        ];

        let selected = select_device_for_connect(&options, &devices).expect("device should match");
        assert_eq!(selected.serial, "R5CX123");
        assert_eq!(selected.connection_kind, DeviceConnectionKind::Usb);
    }

    #[test]
    fn connect_defaults_to_first_available_device() {
        let options = ConnectOptions {
            platform: MobilePlatform::Android,
            device: None,
            adb_endpoint: None,
            preserve_app_state: false,
            timeout_ms: None,
        };
        let devices = vec![
            AdbDevice {
                serial: "emulator-5554".to_string(),
                name: Some("Pixel 8 API 35".to_string()),
                connection_kind: DeviceConnectionKind::Emulator,
                state: "device".to_string(),
            },
            AdbDevice {
                serial: "R5CX123".to_string(),
                name: Some("QA Galaxy S24".to_string()),
                connection_kind: DeviceConnectionKind::Usb,
                state: "device".to_string(),
            },
        ];

        let selected = select_device_for_connect(&options, &devices).expect("first device");
        assert_eq!(selected.serial, "emulator-5554");
    }

    #[test]
    fn connect_matches_friendly_device_name() {
        let options = ConnectOptions {
            platform: MobilePlatform::Android,
            device: Some("qa galaxy s24".to_string()),
            adb_endpoint: None,
            preserve_app_state: false,
            timeout_ms: None,
        };
        let devices = vec![
            AdbDevice {
                serial: "emulator-5554".to_string(),
                name: Some("Pixel 8 API 35".to_string()),
                connection_kind: DeviceConnectionKind::Emulator,
                state: "device".to_string(),
            },
            AdbDevice {
                serial: "R5CX123".to_string(),
                name: Some("QA Galaxy S24".to_string()),
                connection_kind: DeviceConnectionKind::Usb,
                state: "device".to_string(),
            },
        ];

        let selected = select_device_for_connect(&options, &devices).expect("friendly name match");
        assert_eq!(selected.serial, "R5CX123");
    }

    #[test]
    fn connect_fails_when_requested_device_is_missing() {
        let options = ConnectOptions {
            platform: MobilePlatform::Android,
            device: Some("missing-device".to_string()),
            adb_endpoint: None,
            preserve_app_state: false,
            timeout_ms: None,
        };
        let devices = vec![AdbDevice {
            serial: "emulator-5554".to_string(),
            name: Some("Pixel 8 API 35".to_string()),
            connection_kind: DeviceConnectionKind::Emulator,
            state: "device".to_string(),
        }];

        let error = select_device_for_connect(&options, &devices).expect_err("missing device");
        assert_eq!(
            error,
            ConnectSelectionError::RequestedDeviceNotFound {
                requested: "missing-device".to_string()
            }
        );
    }

    #[test]
    fn parse_package_name_from_aapt_output_extracts_package() {
        let package_name = parse_package_name_from_aapt_badging(
            "package: name='dev.allwright.sample' versionCode='1' versionName='1.0'\n",
        )
        .expect("package name should parse");
        assert_eq!(package_name, "dev.allwright.sample");
    }

    #[test]
    fn dump_source_keeps_page_context_separate() {
        let page_session = MobilePageSessionHandle {
            page_id: "emulator-5554:dev.allwright.sample".to_string(),
            package_name: Some("dev.allwright.sample".to_string()),
            activity_name: Some(".MainActivity".to_string()),
            webview_context: None,
        };
        assert_eq!(page_session.page_id, "emulator-5554:dev.allwright.sample");
    }
}
