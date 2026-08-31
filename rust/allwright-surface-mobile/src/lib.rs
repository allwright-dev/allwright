use allwright_plugin_sdk::SurfaceFamily;
use allwright_plugin_sdk::SurfacePluginDescriptor;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

pub const SURFACE_ID: &str = "mobile";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileAutomationBackend {
    UiAutomator2,
    Espresso,
    WebViewBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileAppKind {
    Native,
    Hybrid,
    BrowserWrapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMaturity {
    Planned,
    Scaffolding,
    RuntimeReady,
    Installable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileCapabilitySet {
    pub supports_native_views: bool,
    pub supports_webviews: bool,
    pub supports_deep_links: bool,
    pub supports_shell_commands: bool,
    pub supports_device_logs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileSurfaceProfile {
    pub plugin_id: &'static str,
    pub display_name: &'static str,
    pub family: SurfaceFamily,
    pub backends: &'static [MobileAutomationBackend],
    pub default_backend: MobileAutomationBackend,
    pub supported_app_kinds: &'static [MobileAppKind],
    pub capabilities: MobileCapabilitySet,
    pub bootstrap_hint: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileRuntimeReadiness {
    pub maturity: RuntimeMaturity,
    pub missing_runtime_artifacts: &'static [&'static str],
    pub next_milestones: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MobilePlatform {
    Android,
    Ios,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConnectionKind {
    Usb,
    Emulator,
    RemoteAdb,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceTarget {
    pub platform: MobilePlatform,
    pub device_id: String,
    pub connection_kind: DeviceConnectionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectOptions {
    pub platform: MobilePlatform,
    pub device: Option<String>,
    pub adb_endpoint: Option<String>,
    pub preserve_app_state: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchOptions {
    pub apk_path: Option<String>,
    pub app_id: Option<String>,
    pub launch_activity: Option<String>,
    pub stop_before_launch: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileAutomationSessionInfo {
    pub backend: String,
    pub session_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileBrowserSessionHandle {
    pub platform: MobilePlatform,
    pub automation: MobileAutomationSessionInfo,
    pub device: DeviceTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobilePageSessionHandle {
    pub page_id: String,
    pub package_name: Option<String>,
    pub activity_name: Option<String>,
    pub webview_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobilePageInfo {
    pub note: String,
    pub page_session: MobilePageSessionHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileConnectInfo {
    pub browser: String,
    pub note: String,
    pub browser_session: MobileBrowserSessionHandle,
    pub initial_page: MobilePageInfo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectorFlavor {
    Css,
    XPath,
    UiAutomator,
}

impl SelectorFlavor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::XPath => "xpath",
            Self::UiAutomator => "uia",
        }
    }
}

const UIAUTOMATOR_SELECTOR_KEYS: &[&str] = &[
    "text",
    "textcontains",
    "textmatches",
    "textstartswith",
    "classname",
    "classnamematches",
    "description",
    "desc",
    "descriptioncontains",
    "desccontains",
    "descriptionmatches",
    "descmatches",
    "descriptionstartswith",
    "descstartswith",
    "checkable",
    "checked",
    "clickable",
    "longclickable",
    "scrollable",
    "enabled",
    "focusable",
    "focused",
    "selected",
    "packagename",
    "package",
    "packagenamematches",
    "resourceid",
    "resourceidmatches",
    "index",
    "instance",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileLocator {
    pub selector: String,
}

impl MobileLocator {
    pub fn normalize(selector: &str) -> Self {
        Self {
            selector: normalize_selector_for_transport(selector),
        }
    }

    pub fn chain(&self, child_selector: &str) -> Self {
        Self {
            selector: chain_selector_for_transport(&self.selector, child_selector),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileClickInfo {
    pub selector: String,
    pub note: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileElementCountInfo {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileFillInfo {
    pub selector: String,
    pub value: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileTextInfo {
    pub selector: String,
    pub text: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileWaitForSelectorInfo {
    pub selector: String,
    pub visible: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileScreenshotInfo {
    pub png_data: Vec<u8>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum MobileCommand {
    Connect(ConnectOptions),
    LaunchApp {
        browser_session: MobileBrowserSessionHandle,
        options: LaunchOptions,
    },
    OpenPage {
        browser_session: MobileBrowserSessionHandle,
    },
    ClosePage {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
    },
    ClickElement {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    CountElements {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    FillElement {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        selector: String,
        value: String,
        timeout_ms: Option<u32>,
    },
    GetText {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    WaitForSelector {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        selector: String,
        visible: bool,
        timeout_ms: Option<u32>,
    },
    Screenshot {
        browser_session: MobileBrowserSessionHandle,
        page_session: MobilePageSessionHandle,
        timeout_ms: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum MobileCommandResult {
    Connect(MobileConnectInfo),
    LaunchApp(MobilePageInfo),
    OpenPage(MobilePageInfo),
    ClosePage,
    ClickElement(MobileClickInfo),
    CountElements(MobileElementCountInfo),
    FillElement(MobileFillInfo),
    GetText(MobileTextInfo),
    WaitForSelector(MobileWaitForSelectorInfo),
    Screenshot(MobileScreenshotInfo),
}

pub fn shared_descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: SURFACE_ID,
        family: SurfaceFamily::Mobile,
        version: env!("CARGO_PKG_VERSION"),
        description: "Shared mobile surface abstractions for Android and iOS plugins.",
    }
}

pub async fn boot_surface(label: &str, delay_ms: u64) -> String {
    sleep(Duration::from_millis(delay_ms)).await;
    format!("{label} ready")
}

pub async fn boot() -> String {
    boot_surface("mobile", 25).await
}

fn parse_explicit_selector_prefix(selector: &str) -> Option<(SelectorFlavor, usize)> {
    let lowered = selector.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return Some((SelectorFlavor::XPath, 6));
    }
    if lowered.starts_with("uia=") || lowered.starts_with("uia:") {
        return Some((SelectorFlavor::UiAutomator, 4));
    }
    if let Some(prefix_len) = uiautomator_selector_prefix_len(&lowered) {
        return Some((SelectorFlavor::UiAutomator, prefix_len));
    }
    if lowered.starts_with("text=") || lowered.starts_with("text:") {
        return Some((SelectorFlavor::UiAutomator, 5));
    }
    if lowered.starts_with("id=") || lowered.starts_with("id:") {
        return Some((SelectorFlavor::Css, 3));
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return Some((SelectorFlavor::Css, 4));
    }
    None
}

fn uiautomator_selector_prefix_len(lowered: &str) -> Option<usize> {
    UIAUTOMATOR_SELECTOR_KEYS.iter().find_map(|key| {
        if lowered.starts_with(key) {
            let separator = lowered.as_bytes().get(key.len()).copied()?;
            if separator == b'=' || separator == b':' {
                return Some(key.len() + 1);
            }
        }
        None
    })
}

fn find_json_string_end(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.first().copied()? != b'"' {
        return None;
    }

    let mut index = 1usize;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'"' => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn is_normalized_transport_selector(selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut index = 0usize;
    while index < trimmed.len() {
        let Some((_, prefix_len)) = parse_explicit_selector_prefix(&trimmed[index..]) else {
            return false;
        };

        index += prefix_len;
        let remainder = &trimmed[index..];
        if !remainder.starts_with('"') {
            return false;
        }

        let Some(json_end) = find_json_string_end(remainder) else {
            return false;
        };
        index += json_end;

        if index == trimmed.len() {
            return true;
        }

        let whitespace_len = trimmed[index..]
            .chars()
            .take_while(|char| char.is_ascii_whitespace())
            .count();
        if whitespace_len == 0 {
            return false;
        }
        index += whitespace_len;

        if parse_explicit_selector_prefix(&trimmed[index..]).is_none() {
            return false;
        }
    }

    true
}

fn decode_selector_body(body: &str) -> String {
    let candidate = body.trim();
    if candidate.len() >= 2 && candidate.starts_with('"') && candidate.ends_with('"') {
        if let Ok(decoded) = serde_json::from_str::<String>(candidate) {
            return unescape_shell_escaped_selector(&decoded);
        }
    }
    unescape_shell_escaped_selector(candidate)
}

fn unescape_shell_escaped_selector(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('_' | ' ' | '#' | ':' | '[' | ']' | '(' | ')' | '"' | '\'') => {
                    result.push(chars.next().expect("peeked char should exist"));
                    continue;
                }
                _ => {}
            }
        }
        result.push(ch);
    }
    result
}

pub fn parse_selector_for_transport(selector: &str) -> (SelectorFlavor, String) {
    let trimmed = selector.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return (SelectorFlavor::XPath, decode_selector_body(&trimmed[6..]));
    }
    if lowered.starts_with("uia=") || lowered.starts_with("uia:") {
        return (
            SelectorFlavor::UiAutomator,
            decode_selector_body(&trimmed[4..]),
        );
    }
    if let Some(prefix_len) = uiautomator_selector_prefix_len(&lowered) {
        return (
            SelectorFlavor::UiAutomator,
            trimmed[..prefix_len - 1].to_string()
                + "="
                + &decode_selector_body(&trimmed[prefix_len..]),
        );
    }
    if lowered.starts_with("text=") || lowered.starts_with("text:") {
        let body = decode_selector_body(&trimmed[5..]);
        return (SelectorFlavor::UiAutomator, format!("text={body}"));
    }
    if lowered.starts_with("id=") || lowered.starts_with("id:") {
        let body = decode_selector_body(&trimmed[3..]);
        let normalized = if body.starts_with('#') {
            body
        } else {
            format!("#{body}")
        };
        return (SelectorFlavor::Css, normalized);
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return (SelectorFlavor::Css, decode_selector_body(&trimmed[4..]));
    }
    if trimmed.starts_with("//")
        || trimmed.starts_with(".//")
        || trimmed.starts_with("../")
        || trimmed.starts_with('/')
        || trimmed.starts_with('(')
    {
        return (SelectorFlavor::XPath, trimmed.to_string());
    }
    (SelectorFlavor::Css, trimmed.to_string())
}

pub fn normalize_selector_for_transport(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if is_normalized_transport_selector(trimmed) {
        return trimmed.to_string();
    }
    let (flavor, body) = parse_selector_for_transport(selector);
    format!(
        "{}={}",
        flavor.as_str(),
        serde_json::to_string(&body).unwrap_or_else(|_| format!("{body:?}"))
    )
}

pub fn chain_selector_for_transport(parent: &str, child: &str) -> String {
    let parent_selector = if parent.trim().is_empty() {
        String::new()
    } else {
        normalize_selector_for_transport(parent)
    };
    let child_selector = if child.trim().is_empty() {
        String::new()
    } else {
        normalize_selector_for_transport(child)
    };
    if parent_selector.is_empty() {
        return child_selector;
    }
    if child_selector.is_empty() {
        return parent_selector;
    }
    format!("{parent_selector} {child_selector}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_mobile_runtime() {
        assert_eq!(boot().await, "mobile ready");
    }

    #[tokio::test]
    async fn boots_named_mobile_surface() {
        assert_eq!(boot_surface("android", 1).await, "android ready");
    }

    #[test]
    fn normalizes_xpath_and_css_like_web_clients() {
        assert_eq!(
            normalize_selector_for_transport("xpath=//android.widget.TextView"),
            "xpath=\"//android.widget.TextView\""
        );
        assert_eq!(normalize_selector_for_transport("#login"), "css=\"#login\"");
        assert_eq!(
            normalize_selector_for_transport("Id=bottom_nav_account"),
            "css=\"#bottom_nav_account\""
        );
        assert_eq!(
            normalize_selector_for_transport(r"Id=bottom\_nav\_account"),
            "css=\"#bottom_nav_account\""
        );
        assert_eq!(
            normalize_selector_for_transport("text=Account"),
            "uia=\"text=Account\""
        );
        assert_eq!(
            normalize_selector_for_transport("textContains=Account"),
            "uia=\"textContains=Account\""
        );
        assert_eq!(
            normalize_selector_for_transport("resourceId=com.example:id/login"),
            "uia=\"resourceId=com.example:id/login\""
        );
        assert_eq!(
            normalize_selector_for_transport("descriptionContains=Account"),
            "uia=\"descriptionContains=Account\""
        );
        assert_eq!(
            normalize_selector_for_transport("selected=true"),
            "uia=\"selected=true\""
        );
        assert_eq!(
            normalize_selector_for_transport("classNameMatches=android\\.widget\\..*"),
            "uia=\"classNameMatches=android\\\\.widget\\\\..*\""
        );
    }

    #[test]
    fn chains_mobile_locators_like_web_locators() {
        let parent = MobileLocator::normalize("xpath=//android.view.ViewGroup");
        let child = parent.chain("css=.cta");
        assert_eq!(
            child.selector,
            "xpath=\"//android.view.ViewGroup\" css=\".cta\""
        );
    }

    #[test]
    fn mobile_connect_command_returns_web_like_session_shape() {
        let command = MobileCommand::Connect(ConnectOptions {
            platform: MobilePlatform::Android,
            device: Some("emulator-5554".to_string()),
            adb_endpoint: None,
            preserve_app_state: true,
            timeout_ms: Some(5_000),
        });

        match command {
            MobileCommand::Connect(options) => {
                assert_eq!(options.platform, MobilePlatform::Android);
                assert_eq!(options.device.as_deref(), Some("emulator-5554"));
            }
            _ => panic!("expected connect command"),
        }
    }

    #[test]
    fn mobile_launch_command_keeps_launch_shape_separate() {
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

        let command = MobileCommand::LaunchApp {
            browser_session,
            options: LaunchOptions {
                apk_path: Some("/tmp/app.apk".to_string()),
                app_id: Some("dev.allwright.sample".to_string()),
                launch_activity: Some(".MainActivity".to_string()),
                stop_before_launch: true,
                timeout_ms: Some(15_000),
            },
        };

        match command {
            MobileCommand::LaunchApp { options, .. } => {
                assert_eq!(options.apk_path.as_deref(), Some("/tmp/app.apk"));
                assert!(options.stop_before_launch);
            }
            _ => panic!("expected launch command"),
        }
    }
}
