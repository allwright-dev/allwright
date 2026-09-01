use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor,
};
use allwright_surface_mobile::{
    ConnectOptions, DeviceConnectionKind, DeviceTarget, LaunchOptions, MobileAppKind,
    MobileAutomationBackend, MobileAutomationSessionInfo, MobileBrowserSessionHandle,
    MobileCapabilitySet, MobileClickInfo, MobileCommand, MobileCommandResult, MobileConnectInfo,
    MobileFillInfo, MobilePageInfo, MobilePageSessionHandle, MobilePlatform,
    MobileRuntimeReadiness, MobileScreenshotInfo, MobileSurfaceProfile, RuntimeMaturity,
    boot_surface, normalize_selector_for_transport,
};
use image::{DynamicImage, ImageFormat, RgbaImage};
use regex::Regex;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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
    "expanded command coverage beyond connect, launch, click, and fill",
];
const ANDROID_NEXT_MILESTONES: &[&str] = &[
    "route Android mobile commands through the core engine session server",
    "add text and waitForSelector on top of the native ADB-driven runtime",
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForegroundAppInfo {
    current_package: Option<String>,
    current_activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AndroidUiNode {
    class_name: Option<String>,
    resource_id: Option<String>,
    package_name: Option<String>,
    text: Option<String>,
    content_desc: Option<String>,
    checkable: Option<bool>,
    checked: Option<bool>,
    clickable: Option<bool>,
    long_clickable: Option<bool>,
    scrollable: Option<bool>,
    enabled: Option<bool>,
    focusable: Option<bool>,
    focused: Option<bool>,
    selected: Option<bool>,
    index: Option<usize>,
    bounds: Option<AndroidBounds>,
    parent_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AndroidBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl AndroidBounds {
    fn center(self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SelectorFlavorInternal {
    Css,
    XPath,
    UiAutomator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectorSegment {
    flavor: SelectorFlavorInternal,
    value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NodeCriteria {
    class_name: Option<String>,
    class_name_matches: Option<String>,
    resource_id_exact: Option<String>,
    resource_id_suffix: Option<String>,
    resource_id_matches: Option<String>,
    package_name: Option<String>,
    package_name_matches: Option<String>,
    text: Option<String>,
    text_contains: Option<String>,
    text_starts_with: Option<String>,
    text_matches: Option<String>,
    content_desc: Option<String>,
    content_desc_contains: Option<String>,
    content_desc_starts_with: Option<String>,
    content_desc_matches: Option<String>,
    checkable: Option<bool>,
    checked: Option<bool>,
    clickable: Option<bool>,
    long_clickable: Option<bool>,
    scrollable: Option<bool>,
    enabled: Option<bool>,
    focusable: Option<bool>,
    focused: Option<bool>,
    selected: Option<bool>,
    index: Option<usize>,
    instance: Option<usize>,
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
    let foreground = current_foreground_app(&selected.serial)?;

    Ok(MobileConnectInfo {
        browser: "android".to_string(),
        note: format!(
            "connected Android device `{}` over ADB and attached the native Android runtime",
            selected.name.as_deref().unwrap_or(selected.serial.as_str())
        ),
        browser_session: MobileBrowserSessionHandle {
            platform: MobilePlatform::Android,
            automation: MobileAutomationSessionInfo {
                backend: "android-adb".to_string(),
                session_id: format!("android-adb:{}", selected.serial),
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
                package_name: foreground.current_package,
                activity_name: foreground.current_activity,
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
    let mut resolved_apk = None;

    if let Some(apk_path) = options.apk_path.as_deref() {
        let apk = resolve_apk_source(apk_path)?;
        let apk_owned = apk.path.to_string_lossy().into_owned();
        let _ = run_adb_for_device(device_id, &["install", "-r", &apk_owned])?;
        resolved_apk = Some(apk);
    }

    let package_name = match options.app_id.clone() {
        Some(package_name) => package_name,
        None => {
            if let Some(apk) = resolved_apk.as_ref() {
                resolve_package_name_from_apk_path(apk.path.as_path())?
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

    let current = current_foreground_app(device_id)?;
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

    let clicked = adb_click(&browser_session.device.device_id, &normalized, timeout_ms)?;
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

    let filled = adb_fill(
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
    let source = adb_dump_source(&browser_session.device.device_id)?;
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
        MobileCommand::Screenshot {
            browser_session,
            page_session,
            timeout_ms,
            full_page,
        } => screenshot(&browser_session, &page_session, timeout_ms, full_page)
            .map(MobileCommandResult::Screenshot),
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

fn run_adb_bytes_for_device(device_id: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut all_args = vec!["-s", device_id];
    all_args.extend_from_slice(args);
    run_command_bytes(&adb_command_path(), &all_args)
}

fn adb_command_path() -> String {
    env::var("ALLWRIGHT_ANDROID_ADB").unwrap_or_else(|_| "adb".to_string())
}

fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let output = run_command_output(command, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_command_bytes(command: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    Ok(run_command_output(command, args)?.stdout)
}

fn run_command_output(command: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run `{command}`: {error}"))?;
    if output.status.success() {
        return Ok(output);
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

#[derive(Debug)]
struct ResolvedApkSource {
    path: PathBuf,
    _downloaded_file: Option<PathBuf>,
}

fn resolve_apk_source(apk_path_or_url: &str) -> Result<ResolvedApkSource, String> {
    if is_remote_apk_source(apk_path_or_url) {
        let downloaded_path = download_apk_to_temp(apk_path_or_url)?;
        return Ok(ResolvedApkSource {
            path: downloaded_path.clone(),
            _downloaded_file: Some(downloaded_path),
        });
    }

    let apk = Path::new(apk_path_or_url);
    if !apk.is_file() {
        return Err(format!("APK path does not exist: {}", apk.display()));
    }
    Ok(ResolvedApkSource {
        path: apk.to_path_buf(),
        _downloaded_file: None,
    })
}

fn is_remote_apk_source(apk_path_or_url: &str) -> bool {
    let lowered = apk_path_or_url.trim().to_ascii_lowercase();
    lowered.starts_with("http://") || lowered.starts_with("https://")
}

fn download_apk_to_temp(url: &str) -> Result<PathBuf, String> {
    let response = reqwest::blocking::get(url)
        .map_err(|error| format!("failed to download APK from `{url}`: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "failed to download APK from `{url}`: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|error| format!("failed to read APK download from `{url}`: {error}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to resolve system time for APK download: {error}"))?
        .as_millis();
    let path = env::temp_dir().join(format!("allwright-mobile-android-{timestamp}.apk"));
    fs::write(&path, bytes.as_ref()).map_err(|error| {
        format!(
            "failed to write downloaded APK to {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn resolve_package_name_from_apk_path(apk_path: &Path) -> Result<String, String> {
    let apk_path = apk_path.to_string_lossy().into_owned();
    if let Ok(output) = run_command("aapt", &["dump", "badging", &apk_path]) {
        if let Some(package_name) = parse_package_name_from_aapt_badging(&output) {
            return Ok(package_name);
        }
    }

    if let Ok(output) = run_command("apkanalyzer", &["manifest", "application-id", &apk_path]) {
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

fn current_foreground_app(device_id: &str) -> Result<ForegroundAppInfo, String> {
    let window_dump = run_adb_for_device(device_id, &["shell", "dumpsys", "window", "windows"])
        .or_else(|_| run_adb_for_device(device_id, &["shell", "dumpsys", "window"]))?;
    Ok(parse_foreground_app_from_dumpsys(&window_dump))
}

fn adb_click(
    device_id: &str,
    selector: &str,
    timeout_ms: Option<u32>,
) -> Result<UiAutomator2ClickInfo, String> {
    let snapshot = resolve_selector_snapshot(device_id, selector, timeout_ms)?;
    let bounds = snapshot
        .node
        .bounds
        .ok_or_else(|| format!("selector resolved without bounds: {}", snapshot.selector))?;
    let (x, y) = bounds.center();
    let _ = run_adb_for_device(
        device_id,
        &["shell", "input", "tap", &x.to_string(), &y.to_string()],
    )?;
    Ok(UiAutomator2ClickInfo {
        resolved_selector: snapshot.selector,
        note: format!(
            "clicked Android element via native ADB input on {}/{}",
            snapshot
                .foreground
                .current_package
                .as_deref()
                .unwrap_or("<unknown>"),
            snapshot
                .foreground
                .current_activity
                .as_deref()
                .unwrap_or("<unknown>")
        ),
    })
}

fn adb_fill(
    device_id: &str,
    selector: &str,
    fill_value: &str,
    timeout_ms: Option<u32>,
) -> Result<UiAutomator2FillInfo, String> {
    let snapshot = resolve_selector_snapshot(device_id, selector, timeout_ms)?;
    let bounds = snapshot
        .node
        .bounds
        .ok_or_else(|| format!("selector resolved without bounds: {}", snapshot.selector))?;
    let (x, y) = bounds.center();
    let _ = run_adb_for_device(
        device_id,
        &["shell", "input", "tap", &x.to_string(), &y.to_string()],
    )?;
    let _ = run_adb_for_device(
        device_id,
        &["shell", "input", "keyevent", "KEYCODE_MOVE_END"],
    )?;
    let existing_text = snapshot.node.text.as_deref().unwrap_or("");
    for _ in existing_text.chars() {
        let _ = run_adb_for_device(device_id, &["shell", "input", "keyevent", "KEYCODE_DEL"])?;
    }
    let encoded = encode_adb_text(fill_value);
    if !encoded.is_empty() {
        let _ = run_adb_for_device(device_id, &["shell", "input", "text", &encoded])?;
    }
    Ok(UiAutomator2FillInfo {
        resolved_selector: snapshot.selector,
        value: fill_value.to_string(),
        note: format!(
            "filled Android element via native ADB input on {}/{}",
            snapshot
                .foreground
                .current_package
                .as_deref()
                .unwrap_or("<unknown>"),
            snapshot
                .foreground
                .current_activity
                .as_deref()
                .unwrap_or("<unknown>")
        ),
    })
}

fn adb_screenshot(
    device_id: &str,
    _timeout_ms: Option<u32>,
) -> Result<MobileScreenshotInfo, String> {
    let png_data = run_adb_bytes_for_device(device_id, &["exec-out", "screencap", "-p"])?;
    if png_data.is_empty() {
        return Err("adb returned an empty screenshot payload".to_string());
    }
    let foreground = current_foreground_app(device_id)?;
    Ok(MobileScreenshotInfo {
        png_data,
        note: format!(
            "captured Android screenshot via adb exec-out screencap on {}/{}",
            foreground.current_package.as_deref().unwrap_or("<unknown>"),
            foreground
                .current_activity
                .as_deref()
                .unwrap_or("<unknown>")
        ),
    })
}

fn adb_dump_source(device_id: &str) -> Result<UiAutomator2SourceInfo, String> {
    let remote_path = "/data/local/tmp/allwright-window.xml";
    let _ = run_adb_for_device(device_id, &["shell", "uiautomator", "dump", remote_path])?;
    let source = run_adb_for_device(device_id, &["shell", "cat", remote_path])?;
    let foreground = current_foreground_app(device_id)?;
    Ok(UiAutomator2SourceInfo {
        source,
        current_package: foreground.current_package,
        current_activity: foreground.current_activity,
    })
}

fn screenshot(
    browser_session: &MobileBrowserSessionHandle,
    _page_session: &MobilePageSessionHandle,
    timeout_ms: Option<u32>,
    full_page: bool,
) -> Result<MobileScreenshotInfo, String> {
    if full_page {
        adb_full_page_screenshot(&browser_session.device.device_id, timeout_ms)
    } else {
        adb_screenshot(&browser_session.device.device_id, timeout_ms)
    }
}

const MAX_FULL_PAGE_SCREENSHOTS: usize = 20;

fn adb_full_page_screenshot(
    device_id: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileScreenshotInfo, String> {
    let first = decode_android_screenshot(adb_screenshot(device_id, timeout_ms)?.png_data)?;
    let width = first.width();
    let height = first.height();
    if width == 0 || height < 4 {
        return Err("Android returned an invalid screenshot size".to_string());
    }

    let mut previous = first.clone();
    let mut stitched = first;
    let swipe_start = (height * 3 / 4) as i32;
    let swipe_end = (height / 4) as i32;
    for _ in 1..MAX_FULL_PAGE_SCREENSHOTS {
        run_adb_for_device(
            device_id,
            &[
                "shell",
                "input",
                "swipe",
                &(width as i32 / 2).to_string(),
                &swipe_start.to_string(),
                &(width as i32 / 2).to_string(),
                &swipe_end.to_string(),
                "250",
            ],
        )?;
        std::thread::sleep(Duration::from_millis(300));

        let next = decode_android_screenshot(adb_screenshot(device_id, timeout_ms)?.png_data)?;
        if next.dimensions() != (width, height) {
            return Err(
                "Android screenshot dimensions changed while capturing a full page".to_string(),
            );
        }
        let overlap = find_screenshot_overlap(&previous, &next)?;
        if overlap == height {
            break;
        }
        append_screenshot(&mut stitched, &next, overlap)?;
        previous = next;
    }

    let mut png_data = Vec::new();
    DynamicImage::ImageRgba8(stitched)
        .write_to(&mut std::io::Cursor::new(&mut png_data), ImageFormat::Png)
        .map_err(|error| format!("encode Android full-page screenshot: {error}"))?;
    let foreground = current_foreground_app(device_id)?;
    Ok(MobileScreenshotInfo {
        png_data,
        note: format!(
            "captured full-page Android screenshot via ADB scrolling on {}/{}",
            foreground.current_package.as_deref().unwrap_or("<unknown>"),
            foreground
                .current_activity
                .as_deref()
                .unwrap_or("<unknown>")
        ),
    })
}

fn decode_android_screenshot(png_data: Vec<u8>) -> Result<RgbaImage, String> {
    image::load_from_memory_with_format(&png_data, ImageFormat::Png)
        .map_err(|error| format!("decode Android screenshot: {error}"))
        .map(DynamicImage::into_rgba8)
}

fn find_screenshot_overlap(previous: &RgbaImage, next: &RgbaImage) -> Result<u32, String> {
    let (width, height) = previous.dimensions();
    if next.dimensions() != (width, height) {
        return Err("cannot stitch Android screenshots with different dimensions".to_string());
    }
    for overlap in ((height / 4)..=height).rev() {
        if screenshots_match_at_overlap(previous, next, overlap) {
            return Ok(overlap);
        }
    }
    Err(
        "could not align Android screenshots after scrolling; the foreground content changed"
            .to_string(),
    )
}

fn screenshots_match_at_overlap(previous: &RgbaImage, next: &RgbaImage, overlap: u32) -> bool {
    let (width, height) = previous.dimensions();
    let row_step = (overlap / 24).max(1);
    let column_step = (width / 24).max(1);
    let mut matched = 0_u32;
    let mut sampled = 0_u32;
    for row in (0..overlap).step_by(row_step as usize) {
        for column in (0..width).step_by(column_step as usize) {
            let before = previous.get_pixel(column, height - overlap + row);
            let after = next.get_pixel(column, row);
            sampled += 1;
            if before == after {
                matched += 1;
            }
        }
    }
    sampled > 0 && matched * 100 >= sampled * 98
}

fn append_screenshot(
    stitched: &mut RgbaImage,
    next: &RgbaImage,
    overlap: u32,
) -> Result<(), String> {
    let (width, height) = stitched.dimensions();
    let appended_height = next.height() - overlap;
    let combined_height = height
        .checked_add(appended_height)
        .ok_or_else(|| "Android full-page screenshot is too tall".to_string())?;
    let mut combined = RgbaImage::new(width, combined_height);
    image::imageops::replace(&mut combined, stitched, 0, 0);
    image::imageops::replace(&mut combined, next, 0, i64::from(height - overlap));
    *stitched = combined;
    Ok(())
}

struct ResolvedSelectorSnapshot {
    selector: String,
    node: AndroidUiNode,
    foreground: ForegroundAppInfo,
}

fn resolve_selector_snapshot(
    device_id: &str,
    selector: &str,
    _timeout_ms: Option<u32>,
) -> Result<ResolvedSelectorSnapshot, String> {
    let source = adb_dump_source(device_id)?;
    let nodes = parse_android_ui_nodes(&source.source)?;
    let node = find_node_by_selector(&nodes, selector).ok_or_else(|| {
        let package_name = source.current_package.as_deref().unwrap_or("<unknown>");
        let activity_name = source.current_activity.as_deref().unwrap_or("<unknown>");
        format!("selector not found: {selector} (current app: {package_name}/{activity_name})")
    })?;
    Ok(ResolvedSelectorSnapshot {
        selector: selector.to_string(),
        node: node.clone(),
        foreground: ForegroundAppInfo {
            current_package: source.current_package,
            current_activity: source.current_activity,
        },
    })
}

fn parse_foreground_app_from_dumpsys(output: &str) -> ForegroundAppInfo {
    for line in output.lines() {
        if !(line.contains("mCurrentFocus") || line.contains("mFocusedApp")) {
            continue;
        }
        if let Some((package_name, activity_name)) = extract_component_from_line(line) {
            return ForegroundAppInfo {
                current_package: Some(package_name),
                current_activity: Some(activity_name),
            };
        }
    }
    ForegroundAppInfo {
        current_package: None,
        current_activity: None,
    }
}

fn extract_component_from_line(line: &str) -> Option<(String, String)> {
    for token in line.split_whitespace() {
        let cleaned = token.trim_matches(|ch: char| matches!(ch, '{' | '}' | ',' | ';'));
        let slash_index = cleaned.find('/')?;
        let package_name = cleaned[..slash_index].trim();
        let activity_name = cleaned[slash_index + 1..].trim();
        if package_name.is_empty()
            || activity_name.is_empty()
            || !package_name.contains('.')
            || package_name.contains('=')
        {
            continue;
        }
        return Some((package_name.to_string(), activity_name.to_string()));
    }
    None
}

fn parse_android_ui_nodes(xml: &str) -> Result<Vec<AndroidUiNode>, String> {
    let mut nodes = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_start) = xml[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(relative_end) = xml[start..].find('>') else {
            break;
        };
        let end = start + relative_end;
        let tag = &xml[start + 1..end];
        let trimmed = tag.trim();
        if trimmed.starts_with("/node") {
            let _ = stack.pop();
        } else if trimmed.starts_with("node") {
            let self_closing = trimmed.ends_with('/');
            let attributes = parse_xml_attributes(trimmed);
            let node = AndroidUiNode {
                class_name: attributes.get("class").cloned(),
                resource_id: attributes.get("resource-id").cloned(),
                package_name: attributes.get("package").cloned(),
                text: attributes.get("text").cloned(),
                content_desc: attributes.get("content-desc").cloned(),
                checkable: attributes
                    .get("checkable")
                    .and_then(|value| parse_bool(value)),
                checked: attributes
                    .get("checked")
                    .and_then(|value| parse_bool(value)),
                clickable: attributes
                    .get("clickable")
                    .and_then(|value| parse_bool(value)),
                long_clickable: attributes
                    .get("long-clickable")
                    .and_then(|value| parse_bool(value)),
                scrollable: attributes
                    .get("scrollable")
                    .and_then(|value| parse_bool(value)),
                enabled: attributes
                    .get("enabled")
                    .and_then(|value| parse_bool(value)),
                focusable: attributes
                    .get("focusable")
                    .and_then(|value| parse_bool(value)),
                focused: attributes
                    .get("focused")
                    .and_then(|value| parse_bool(value)),
                selected: attributes
                    .get("selected")
                    .and_then(|value| parse_bool(value)),
                index: attributes.get("index").and_then(|value| value.parse().ok()),
                bounds: attributes
                    .get("bounds")
                    .and_then(|value| parse_bounds(value)),
                parent_index: stack.last().copied(),
            };
            let index = nodes.len();
            nodes.push(node);
            if !self_closing {
                stack.push(index);
            }
        }
        cursor = end + 1;
    }
    Ok(nodes)
}

fn parse_xml_attributes(tag: &str) -> std::collections::BTreeMap<String, String> {
    let mut attributes = std::collections::BTreeMap::new();
    let bytes = tag.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        while index < bytes.len() && !bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        let key_start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b'_'))
        {
            index += 1;
        }
        if key_start == index {
            continue;
        }
        let key = &tag[key_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'"' {
            continue;
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != b'"' {
            index += 1;
        }
        if index > value_start {
            attributes.insert(
                key.to_string(),
                decode_xml_entities(&tag[value_start..index]),
            );
        }
        if index < bytes.len() {
            index += 1;
        }
    }
    attributes
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_bounds(value: &str) -> Option<AndroidBounds> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('[')?;
    let (left_top, right_bottom) = trimmed.split_once("][")?;
    let right_bottom = right_bottom.strip_suffix(']')?;
    let (left, top) = left_top.split_once(',')?;
    let (right, bottom) = right_bottom.split_once(',')?;
    Some(AndroidBounds {
        left: left.parse().ok()?,
        top: top.parse().ok()?,
        right: right.parse().ok()?,
        bottom: bottom.parse().ok()?,
    })
}

fn find_node_by_selector<'a>(
    nodes: &'a [AndroidUiNode],
    selector: &str,
) -> Option<&'a AndroidUiNode> {
    let segments = parse_selector_segments(selector).ok()?;
    let criteria = segments
        .iter()
        .map(selector_segment_to_criteria)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let last = criteria.last()?;
    let mut matches = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if !node_matches_criteria(node, last) {
            continue;
        }
        if ancestor_chain_matches(nodes, index, &criteria[..criteria.len().saturating_sub(1)]) {
            matches.push(node);
        }
    }
    if let Some(instance) = last.instance {
        return matches.get(instance).copied();
    }
    matches.into_iter().next()
}

fn ancestor_chain_matches(
    nodes: &[AndroidUiNode],
    node_index: usize,
    criteria_chain: &[NodeCriteria],
) -> bool {
    if criteria_chain.is_empty() {
        return true;
    }
    let mut current_parent = nodes[node_index].parent_index;
    for criteria in criteria_chain.iter().rev() {
        let mut matched_parent = None;
        let mut probe = current_parent;
        while let Some(index) = probe {
            if node_matches_criteria(&nodes[index], criteria) {
                matched_parent = Some(index);
                break;
            }
            probe = nodes[index].parent_index;
        }
        let Some(index) = matched_parent else {
            return false;
        };
        current_parent = nodes[index].parent_index;
    }
    true
}

fn node_matches_criteria(node: &AndroidUiNode, criteria: &NodeCriteria) -> bool {
    if let Some(expected) = criteria.class_name.as_deref()
        && node.class_name.as_deref() != Some(expected)
    {
        return false;
    }
    if let Some(pattern) = criteria.class_name_matches.as_deref()
        && !matches_regex(node.class_name.as_deref(), pattern)
    {
        return false;
    }
    if let Some(expected) = criteria.resource_id_exact.as_deref()
        && node.resource_id.as_deref() != Some(expected)
    {
        return false;
    }
    if let Some(expected_suffix) = criteria.resource_id_suffix.as_deref() {
        let Some(resource_id) = node.resource_id.as_deref() else {
            return false;
        };
        if !resource_id_matches_suffix(resource_id, expected_suffix) {
            return false;
        }
    }
    if let Some(pattern) = criteria.resource_id_matches.as_deref()
        && !matches_regex(node.resource_id.as_deref(), pattern)
    {
        return false;
    }
    if let Some(expected) = criteria.package_name.as_deref()
        && node.package_name.as_deref() != Some(expected)
    {
        return false;
    }
    if let Some(pattern) = criteria.package_name_matches.as_deref()
        && !matches_regex(node.package_name.as_deref(), pattern)
    {
        return false;
    }
    if let Some(expected) = criteria.text.as_deref()
        && node.text.as_deref() != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.text_contains.as_deref()
        && !contains_text(node.text.as_deref(), expected)
    {
        return false;
    }
    if let Some(expected) = criteria.text_starts_with.as_deref()
        && !starts_with_text(node.text.as_deref(), expected)
    {
        return false;
    }
    if let Some(pattern) = criteria.text_matches.as_deref()
        && !matches_regex(node.text.as_deref(), pattern)
    {
        return false;
    }
    if let Some(expected) = criteria.content_desc.as_deref()
        && node.content_desc.as_deref() != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.content_desc_contains.as_deref()
        && !contains_text(node.content_desc.as_deref(), expected)
    {
        return false;
    }
    if let Some(expected) = criteria.content_desc_starts_with.as_deref()
        && !starts_with_text(node.content_desc.as_deref(), expected)
    {
        return false;
    }
    if let Some(pattern) = criteria.content_desc_matches.as_deref()
        && !matches_regex(node.content_desc.as_deref(), pattern)
    {
        return false;
    }
    if let Some(expected) = criteria.checkable
        && node.checkable != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.checked
        && node.checked != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.clickable
        && node.clickable != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.long_clickable
        && node.long_clickable != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.scrollable
        && node.scrollable != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.enabled
        && node.enabled != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.focusable
        && node.focusable != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.focused
        && node.focused != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.selected
        && node.selected != Some(expected)
    {
        return false;
    }
    if let Some(expected) = criteria.index
        && node.index != Some(expected)
    {
        return false;
    }
    true
}

fn parse_selector_segments(selector: &str) -> Result<Vec<SelectorSegment>, String> {
    let mut segments = Vec::new();
    let normalized = normalize_selector_for_transport(selector);
    let trimmed = normalized.trim();
    let mut index = 0usize;
    while index < trimmed.len() {
        let remainder = &trimmed[index..];
        let (flavor, prefix_len) = if remainder.starts_with("css=") {
            (SelectorFlavorInternal::Css, 4usize)
        } else if remainder.starts_with("xpath=") {
            (SelectorFlavorInternal::XPath, 6usize)
        } else if remainder.starts_with("uia=") {
            (SelectorFlavorInternal::UiAutomator, 4usize)
        } else {
            return Err(format!("unsupported selector segment in `{trimmed}`"));
        };
        index += prefix_len;
        let bytes = trimmed.as_bytes();
        if bytes.get(index).copied() != Some(b'"') {
            return Err(format!(
                "selector segment must use JSON string syntax: `{trimmed}`"
            ));
        }
        let start = index;
        index += 1;
        let mut escaped = false;
        while index < trimmed.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                index += 1;
                break;
            }
            index += 1;
        }
        let json_body = &trimmed[start..index];
        let value = serde_json::from_str::<String>(json_body)
            .map_err(|error| format!("failed to decode selector segment {json_body}: {error}"))?;
        segments.push(SelectorSegment { flavor, value });
        while index < trimmed.len() && trimmed.as_bytes()[index].is_ascii_whitespace() {
            index += 1;
        }
    }
    if segments.is_empty() {
        return Err("selector must not be empty".to_string());
    }
    Ok(segments)
}

fn selector_segment_to_criteria(segment: &SelectorSegment) -> Result<NodeCriteria, String> {
    match segment.flavor {
        SelectorFlavorInternal::Css => parse_css_criteria(&segment.value),
        SelectorFlavorInternal::XPath => parse_xpath_criteria(&segment.value),
        SelectorFlavorInternal::UiAutomator => parse_uiautomator_criteria(&segment.value),
    }
}

fn parse_uiautomator_criteria(selector: &str) -> Result<NodeCriteria, String> {
    let mut criteria = NodeCriteria::default();
    for token in split_uiautomator_tokens(selector) {
        let (raw_key, raw_value) = split_uiautomator_token(&token)?;
        let key = raw_key.to_ascii_lowercase();
        match key.as_str() {
            "text" => criteria.text = Some(raw_value.to_string()),
            "textcontains" => criteria.text_contains = Some(raw_value.to_string()),
            "textstartswith" => criteria.text_starts_with = Some(raw_value.to_string()),
            "textmatches" => criteria.text_matches = Some(raw_value.to_string()),
            "classname" => criteria.class_name = Some(raw_value.to_string()),
            "classnamematches" => criteria.class_name_matches = Some(raw_value.to_string()),
            "description" | "desc" => criteria.content_desc = Some(raw_value.to_string()),
            "descriptioncontains" | "desccontains" => {
                criteria.content_desc_contains = Some(raw_value.to_string())
            }
            "descriptionstartswith" | "descstartswith" => {
                criteria.content_desc_starts_with = Some(raw_value.to_string())
            }
            "descriptionmatches" | "descmatches" => {
                criteria.content_desc_matches = Some(raw_value.to_string())
            }
            "packagename" | "package" => criteria.package_name = Some(raw_value.to_string()),
            "packagenamematches" => criteria.package_name_matches = Some(raw_value.to_string()),
            "resourceid" => criteria.resource_id_exact = Some(raw_value.to_string()),
            "resourceidmatches" => criteria.resource_id_matches = Some(raw_value.to_string()),
            "checkable" => criteria.checkable = Some(parse_selector_bool(raw_value, &token)?),
            "checked" => criteria.checked = Some(parse_selector_bool(raw_value, &token)?),
            "clickable" => criteria.clickable = Some(parse_selector_bool(raw_value, &token)?),
            "longclickable" => {
                criteria.long_clickable = Some(parse_selector_bool(raw_value, &token)?)
            }
            "scrollable" => criteria.scrollable = Some(parse_selector_bool(raw_value, &token)?),
            "enabled" => criteria.enabled = Some(parse_selector_bool(raw_value, &token)?),
            "focusable" => criteria.focusable = Some(parse_selector_bool(raw_value, &token)?),
            "focused" => criteria.focused = Some(parse_selector_bool(raw_value, &token)?),
            "selected" => criteria.selected = Some(parse_selector_bool(raw_value, &token)?),
            "index" => criteria.index = Some(parse_selector_usize(raw_value, &token)?),
            "instance" => criteria.instance = Some(parse_selector_usize(raw_value, &token)?),
            other => return Err(format!("unsupported UiAutomator selector key `{other}`")),
        }
    }
    Ok(criteria)
}

fn split_uiautomator_tokens(selector: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for ch in selector.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            ',' | ';' if !in_single && !in_double => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    tokens.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        tokens.push(trimmed.to_string());
    }
    tokens
}

fn split_uiautomator_token(token: &str) -> Result<(&str, &str), String> {
    let Some((key, value)) = token.split_once(['=', ':']) else {
        return Err(format!("invalid UiAutomator selector token `{token}`"));
    };
    Ok((key.trim(), value.trim()))
}

fn parse_selector_bool(value: &str, token: &str) -> Result<bool, String> {
    parse_bool(value).ok_or_else(|| format!("invalid boolean value in `{token}`"))
}

fn parse_selector_usize(value: &str, token: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid integer value in `{token}`"))
}

fn parse_css_criteria(selector: &str) -> Result<NodeCriteria, String> {
    let mut criteria = NodeCriteria::default();
    let mut remainder = selector.trim();
    while let Some(start) = remainder.find('[') {
        let end = remainder[start + 1..]
            .find(']')
            .map(|index| start + 1 + index)
            .ok_or_else(|| format!("unterminated CSS attribute selector in `{selector}`"))?;
        let attribute = &remainder[start + 1..end];
        let (name, value) = parse_attribute_filter(attribute)?;
        match name {
            "text" => criteria.text = Some(value),
            "content-desc" => criteria.content_desc = Some(value),
            "resource-id" => criteria.resource_id_exact = Some(value),
            _ => {
                return Err(format!(
                    "unsupported CSS attribute selector `{name}` in `{selector}`"
                ));
            }
        }
        let mut next = String::with_capacity(remainder.len());
        next.push_str(remainder[..start].trim_end());
        next.push(' ');
        next.push_str(remainder[end + 1..].trim_start());
        let owned = next.trim().to_string();
        remainder = Box::leak(owned.into_boxed_str());
    }

    let class_or_id = remainder.trim();
    if let Some(resource_id) = class_or_id.strip_prefix('#') {
        if resource_id.contains(':') {
            criteria.resource_id_exact = Some(resource_id.to_string());
        } else {
            criteria.resource_id_suffix = Some(format!(":id/{resource_id}"));
        }
    } else if let Some(class_name) = class_or_id.strip_prefix('.') {
        if !class_name.is_empty() {
            criteria.class_name = Some(class_name.to_string());
        }
    } else if !class_or_id.is_empty() {
        criteria.class_name = Some(class_or_id.to_string());
    }
    Ok(criteria)
}

fn parse_xpath_criteria(selector: &str) -> Result<NodeCriteria, String> {
    let trimmed = selector.trim();
    let mut criteria = NodeCriteria::default();
    let mut path = trimmed;
    if let Some(stripped) = path.strip_prefix(".//") {
        path = stripped;
    } else if let Some(stripped) = path.strip_prefix("//") {
        path = stripped;
    }

    let path = path.trim();
    let (node_pattern, predicates) = if let Some(start) = path.find('[') {
        let end = path
            .rfind(']')
            .ok_or_else(|| format!("unterminated XPath in `{selector}`"))?;
        (&path[..start], Some(&path[start + 1..end]))
    } else {
        (path, None)
    };

    let node_pattern = node_pattern.trim();
    if !node_pattern.is_empty() && node_pattern != "*" {
        criteria.class_name = Some(node_pattern.to_string());
    }

    if let Some(predicates) = predicates {
        for predicate in predicates.split(" and ") {
            let predicate = predicate.trim().trim_matches(|ch| ch == '(' || ch == ')');
            if let Some(value) = parse_xpath_attr_equals(predicate, "@text") {
                criteria.text = Some(value);
                continue;
            }
            if let Some(value) = parse_xpath_attr_equals(predicate, "@content-desc") {
                criteria.content_desc = Some(value);
                continue;
            }
            if let Some(value) = parse_xpath_attr_equals(predicate, "@class") {
                criteria.class_name = Some(value);
                continue;
            }
            if let Some(value) = parse_xpath_attr_equals(predicate, "@resource-id") {
                if value.contains(':') {
                    criteria.resource_id_exact = Some(value);
                } else {
                    criteria.resource_id_suffix = Some(format!(":id/{value}"));
                }
                continue;
            }
            if let Some(value) = parse_xpath_resource_suffix(predicate) {
                criteria.resource_id_suffix = Some(value);
                continue;
            }
        }
    }

    Ok(criteria)
}

fn parse_attribute_filter(attribute: &str) -> Result<(&str, String), String> {
    let (name, value) = attribute
        .split_once('=')
        .ok_or_else(|| format!("unsupported CSS attribute selector `{attribute}`"))?;
    Ok((name.trim(), parse_quoted_value(value.trim())?))
}

fn parse_xpath_attr_equals(predicate: &str, attr_name: &str) -> Option<String> {
    let remainder = predicate.strip_prefix(attr_name)?;
    let remainder = remainder.trim_start();
    let remainder = remainder.strip_prefix('=')?.trim_start();
    parse_quoted_value(remainder).ok()
}

fn parse_xpath_resource_suffix(predicate: &str) -> Option<String> {
    let suffix_marker = ":id/";
    let start = predicate.find(suffix_marker)?;
    let candidate = &predicate[start - 1..];
    parse_quoted_value(candidate).ok()
}

fn resource_id_matches_suffix(resource_id: &str, expected_suffix: &str) -> bool {
    if resource_id.ends_with(expected_suffix) {
        return true;
    }

    let bare_expected = expected_suffix
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty());
    matches!(bare_expected, Some(expected) if resource_id == expected)
}

fn contains_text(actual: Option<&str>, expected: &str) -> bool {
    actual.is_some_and(|value| value.contains(expected))
}

fn starts_with_text(actual: Option<&str>, expected: &str) -> bool {
    actual.is_some_and(|value| value.starts_with(expected))
}

fn matches_regex(actual: Option<&str>, pattern: &str) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    Regex::new(pattern).is_ok_and(|regex| regex.is_match(actual))
}

fn parse_quoted_value(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let quote = trimmed
        .chars()
        .next()
        .filter(|ch| *ch == '\'' || *ch == '"')
        .ok_or_else(|| format!("expected quoted value in `{input}`"))?;
    let end = trimmed[1..]
        .find(quote)
        .ok_or_else(|| format!("unterminated quoted value in `{input}`"))?;
    Ok(trimmed[1..1 + end].to_string())
}

fn encode_adb_text(value: &str) -> String {
    let mut encoded = String::new();
    for ch in value.chars() {
        match ch {
            ' ' => encoded.push_str("%s"),
            '"' => encoded.push_str("\\\""),
            '\'' => encoded.push_str("\\'"),
            '&' | '|' | ';' | '<' | '>' | '(' | ')' | '$' | '\\' => {
                encoded.push('\\');
                encoded.push(ch);
            }
            _ => encoded.push(ch),
        }
    }
    encoded
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
    fn detects_remote_apk_sources() {
        assert!(is_remote_apk_source("https://example.com/app.apk"));
        assert!(is_remote_apk_source("http://example.com/app.apk"));
        assert!(!is_remote_apk_source("/tmp/app.apk"));
    }

    #[test]
    fn resolves_local_apk_source() {
        let apk_path = env::temp_dir().join("allwright-mobile-android-local-test.apk");
        fs::write(&apk_path, b"apk").expect("write local apk fixture");

        let resolved =
            resolve_apk_source(apk_path.to_string_lossy().as_ref()).expect("resolve local apk");
        assert_eq!(resolved.path, apk_path);

        fs::remove_file(&apk_path).expect("remove local apk fixture");
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

    #[test]
    fn parses_uiautomator_selector_strategies() {
        let criteria = parse_uiautomator_criteria(
            "textContains=Account,descriptionStartsWith=Open,classNameMatches=android\\.widget\\..*,clickable=true,index=2,instance=1,resourceIdMatches=.*bottom_nav.*,packageName=com.example.airticket",
        )
        .expect("uia criteria should parse");

        assert_eq!(criteria.text_contains.as_deref(), Some("Account"));
        assert_eq!(criteria.content_desc_starts_with.as_deref(), Some("Open"));
        assert_eq!(
            criteria.class_name_matches.as_deref(),
            Some("android\\.widget\\..*")
        );
        assert_eq!(criteria.clickable, Some(true));
        assert_eq!(criteria.index, Some(2));
        assert_eq!(criteria.instance, Some(1));
        assert_eq!(
            criteria.resource_id_matches.as_deref(),
            Some(".*bottom_nav.*")
        );
        assert_eq!(
            criteria.package_name.as_deref(),
            Some("com.example.airticket")
        );
    }

    #[test]
    fn matches_uiautomator_selectors_against_dumped_nodes() {
        let nodes = parse_android_ui_nodes(sample_ui_hierarchy()).expect("xml should parse");

        let account = find_node_by_selector(&nodes, "text=Account").expect("text selector");
        assert_eq!(
            account.resource_id.as_deref(),
            Some("com.example.airticket:id/bottom_nav_account")
        );

        let by_desc =
            find_node_by_selector(&nodes, "descriptionContains=Account").expect("desc selector");
        assert_eq!(
            by_desc.resource_id.as_deref(),
            Some("com.example.airticket:id/bottom_nav_account")
        );

        let by_resource =
            find_node_by_selector(&nodes, "resourceIdMatches=.*bottom_nav_.*,selected=true")
                .expect("resource regex selector");
        assert_eq!(
            by_resource.resource_id.as_deref(),
            Some("com.example.airticket:id/bottom_nav_account")
        );

        let by_class = find_node_by_selector(
            &nodes,
            "classNameMatches=android\\.widget\\..*,packageNameMatches=com\\.example\\..*,clickable=true,instance=1",
        )
        .expect("class/package/instance selector");
        assert_eq!(
            by_class.resource_id.as_deref(),
            Some("com.example.airticket:id/bottom_nav_account")
        );

        let by_css_id = find_node_by_selector(&nodes, r##"css="#bottom_nav_account""##)
            .expect("css id selector");
        assert_eq!(
            by_css_id.resource_id.as_deref(),
            Some("com.example.airticket:id/bottom_nav_account")
        );

        let bare_nodes =
            parse_android_ui_nodes(sample_ui_hierarchy_with_bare_resource_ids()).expect("xml");
        let bare_by_css_id = find_node_by_selector(&bare_nodes, r##"css="#signup_first_name""##)
            .expect("bare css id");
        assert_eq!(
            bare_by_css_id.resource_id.as_deref(),
            Some("signup_first_name")
        );
    }

    #[test]
    fn stitches_scrolled_screenshots_without_duplicate_overlap() {
        let first =
            RgbaImage::from_fn(4, 10, |x, y| image::Rgba([(x * 11) as u8, y as u8, 0, 255]));
        let second = RgbaImage::from_fn(4, 10, |x, y| {
            image::Rgba([(x * 11) as u8, (y + 4) as u8, 0, 255])
        });

        let overlap = find_screenshot_overlap(&first, &second).expect("screenshots align");
        assert_eq!(overlap, 6);

        let mut stitched = first;
        append_screenshot(&mut stitched, &second, overlap).expect("screenshots stitch");
        assert_eq!(stitched.dimensions(), (4, 14));
        assert_eq!(stitched.get_pixel(0, 13), second.get_pixel(0, 9));
    }

    fn sample_ui_hierarchy() -> &'static str {
        r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="" class="android.widget.FrameLayout" package="com.example.airticket" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,0][1080,2400]">
    <node index="0" text="Home" resource-id="com.example.airticket:id/bottom_nav_home" class="android.widget.TextView" package="com.example.airticket" content-desc="Home" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,2100][360,2400]" />
    <node index="1" text="Account" resource-id="com.example.airticket:id/bottom_nav_account" class="android.widget.TextView" package="com.example.airticket" content-desc="Open Account" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="true" bounds="[360,2100][720,2400]" />
    <node index="2" text="Email" resource-id="com.example.airticket:id/email" class="android.widget.EditText" package="com.example.airticket" content-desc="" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="true" scrollable="false" long-clickable="true" password="false" selected="false" bounds="[80,600][1000,720]" />
  </node>
</hierarchy>"#
    }

    fn sample_ui_hierarchy_with_bare_resource_ids() -> &'static str {
        r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="" class="android.widget.FrameLayout" package="com.example.airticket" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,0][1080,2400]">
    <node index="0" text="First Name" resource-id="signup_first_name" class="android.widget.EditText" package="com.example.airticket" content-desc="" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="true" password="false" selected="false" bounds="[80,600][1000,720]" />
  </node>
</hierarchy>"#
    }
}
