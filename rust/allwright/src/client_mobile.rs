use serde::{Deserialize, Serialize};

use super::bootstrap::{ensure_plugins_installed, invoke_plugin};
use super::types::{ClickResult, CommandOptions, Error, FillResult, Result};

#[derive(Debug, Clone, Default)]
pub struct MobileAndroidConnectOptions {
    pub device: Option<String>,
    pub adb_endpoint: Option<String>,
    pub preserve_app_state: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct MobileAndroidLaunchOptions {
    pub apk_path: Option<String>,
    pub app_id: Option<String>,
    pub launch_activity: Option<String>,
    pub stop_before_launch: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobilePluginEnvelope<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobileBrowserSessionHandle {
    platform: String,
    automation: MobileAutomationSessionInfo,
    device: MobileDeviceTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobileAutomationSessionInfo {
    session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobileDeviceTarget {
    device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobilePageSessionHandle {
    page_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobilePageInfo {
    page_session: MobilePageSessionHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobileConnectInfo {
    browser_session: MobileBrowserSessionHandle,
    initial_page: MobilePageInfo,
}

#[derive(Debug, Clone)]
pub struct AndroidLocator {
    page: AndroidPage,
    selector: String,
}

#[derive(Debug, Clone)]
pub struct AndroidPage {
    browser_session: MobileBrowserSessionHandle,
    page_session: MobilePageSessionHandle,
}

#[derive(Debug, Clone)]
pub struct AndroidDevice {
    connect_info: MobileConnectInfo,
    page: AndroidPage,
}

pub mod android {
    use super::*;

    pub fn connect(options: MobileAndroidConnectOptions) -> Result<AndroidDevice> {
        ensure_plugins_installed(&["mobile-android"])?;
        let request = serde_json::json!({
            "command": "connect",
            "platform": "android",
            "device": options.device,
            "adb_endpoint": options.adb_endpoint,
            "preserve_app_state": options.preserve_app_state,
            "timeout_ms": options.timeout_ms,
        });
        let connect_info: MobileConnectInfo = invoke_android("connect", request)?;
        Ok(AndroidDevice::new(connect_info))
    }
}

impl AndroidDevice {
    fn new(connect_info: MobileConnectInfo) -> Self {
        let page = AndroidPage {
            browser_session: connect_info.browser_session.clone(),
            page_session: connect_info.initial_page.page_session.clone(),
        };
        Self { connect_info, page }
    }

    pub fn session_id(&self) -> &str {
        &self.connect_info.browser_session.automation.session_id
    }

    pub fn page(&self) -> AndroidPage {
        self.page.clone()
    }

    pub fn initial_page(&self) -> AndroidPage {
        self.page()
    }

    pub fn launch(&mut self, options: MobileAndroidLaunchOptions) -> Result<AndroidPage> {
        let request = serde_json::json!({
            "command": "launch_app",
            "browser_session": self.connect_info.browser_session,
            "options": {
                "apk_path": options.apk_path,
                "app_id": options.app_id,
                "launch_activity": options.launch_activity,
                "stop_before_launch": options.stop_before_launch,
                "timeout_ms": options.timeout_ms,
            },
        });
        let page_info: MobilePageInfo = invoke_android("launch", request)?;
        self.page = AndroidPage {
            browser_session: self.connect_info.browser_session.clone(),
            page_session: page_info.page_session,
        };
        Ok(self.page.clone())
    }
}

impl AndroidPage {
    pub fn session_id(&self) -> &str {
        &self.page_session.page_id
    }

    pub fn locator(&self, selector: impl Into<String>) -> AndroidLocator {
        AndroidLocator {
            page: self.clone(),
            selector: normalize_mobile_selector_for_transport(&selector.into()),
        }
    }

    pub fn click(&self, selector: &str, options: CommandOptions) -> Result<ClickResult> {
        #[derive(Deserialize)]
        struct ClickInfo {
            selector: String,
            note: String,
            session_id: String,
        }
        let result: ClickInfo = invoke_android(
            "click",
            serde_json::json!({
                "command": "click_element",
                "browser_session": self.browser_session,
                "page_session": self.page_session,
                "selector": normalize_mobile_selector_for_transport(selector),
                "timeout_ms": options.timeout_ms,
            }),
        )?;
        Ok(ClickResult {
            selector: result.selector,
            note: result.note,
            bidi_session_id: result.session_id,
        })
    }

    pub fn fill(&self, selector: &str, value: &str, options: CommandOptions) -> Result<FillResult> {
        #[derive(Deserialize)]
        struct FillInfo {
            selector: String,
            value: String,
            note: String,
        }
        let result: FillInfo = invoke_android(
            "fill",
            serde_json::json!({
                "command": "fill_element",
                "browser_session": self.browser_session,
                "page_session": self.page_session,
                "selector": normalize_mobile_selector_for_transport(selector),
                "value": value,
                "timeout_ms": options.timeout_ms,
            }),
        )?;
        Ok(FillResult {
            selector: result.selector,
            value: result.value,
            note: result.note,
        })
    }
}

impl AndroidLocator {
    pub fn page(&self) -> &AndroidPage {
        &self.page
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn locator(&self, selector: impl Into<String>) -> AndroidLocator {
        AndroidLocator {
            page: self.page.clone(),
            selector: chain_mobile_selector_for_transport(&self.selector, &selector.into()),
        }
    }

    pub fn click(&self, options: CommandOptions) -> Result<ClickResult> {
        self.page.click(&self.selector, options)
    }

    pub fn fill(&self, value: &str, options: CommandOptions) -> Result<FillResult> {
        self.page.fill(&self.selector, value, options)
    }
}

fn invoke_android<T>(command_name: &str, request: serde_json::Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let payload = invoke_plugin("mobile-android", &request.to_string())?;
    let envelope: MobilePluginEnvelope<T> =
        serde_json::from_str(payload.trim()).map_err(|error| {
            Error::new(format!(
                "failed to decode mobile-android plugin response for {command_name}: {error}"
            ))
        })?;
    if !envelope.ok {
        return Err(Error::new(envelope.error.unwrap_or_else(|| {
            format!("mobile-android plugin {command_name} failed")
        })));
    }
    envelope.result.ok_or_else(|| {
        Error::new(format!(
            "mobile-android plugin {command_name} returned success without a result payload"
        ))
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MobileSelectorFlavor {
    Css,
    XPath,
    UiAutomator,
}

impl MobileSelectorFlavor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::XPath => "xpath",
            Self::UiAutomator => "uia",
        }
    }
}

fn parse_explicit_mobile_selector_prefix(selector: &str) -> Option<(MobileSelectorFlavor, usize)> {
    let lowered = selector.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return Some((MobileSelectorFlavor::XPath, 6));
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return Some((MobileSelectorFlavor::Css, 4));
    }
    if lowered.starts_with("uia=") || lowered.starts_with("uia:") {
        return Some((MobileSelectorFlavor::UiAutomator, 4));
    }
    None
}

fn parse_uiautomator_selector_prefix(selector: &str) -> Option<usize> {
    for (index, ch) in selector.char_indices() {
        if ch != '=' && ch != ':' {
            continue;
        }
        let key = selector[..index].trim().to_ascii_lowercase();
        if UIAUTOMATOR_SELECTOR_KEYS
            .iter()
            .any(|candidate| *candidate == key)
        {
            return Some(index + ch.len_utf8());
        }
        return None;
    }
    None
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

fn is_normalized_mobile_transport_selector(selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut index = 0usize;
    while index < trimmed.len() {
        let Some((_, prefix_len)) = parse_explicit_mobile_selector_prefix(&trimmed[index..]) else {
            return false;
        };
        index += prefix_len;

        let remainder = &trimmed[index..];
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
        if parse_explicit_mobile_selector_prefix(&trimmed[index..]).is_none() {
            return false;
        }
    }

    true
}

fn decode_selector_body(body: &str) -> String {
    let candidate = body.trim();
    if candidate.len() >= 2 && candidate.starts_with('"') && candidate.ends_with('"') {
        if let Ok(decoded) = serde_json::from_str::<String>(candidate) {
            return decoded;
        }
    }
    candidate.to_string()
}

fn parse_mobile_selector_for_transport(selector: &str) -> (MobileSelectorFlavor, String) {
    let trimmed = selector.trim();
    if let Some((flavor, prefix_len)) = parse_explicit_mobile_selector_prefix(trimmed) {
        return (flavor, decode_selector_body(&trimmed[prefix_len..]));
    }
    if let Some(prefix_len) = parse_uiautomator_selector_prefix(trimmed) {
        return (
            MobileSelectorFlavor::UiAutomator,
            format!("{}={}", &trimmed[..prefix_len - 1], &trimmed[prefix_len..]),
        );
    }
    if trimmed.starts_with("//")
        || trimmed.starts_with(".//")
        || trimmed.starts_with("../")
        || trimmed.starts_with('/')
        || trimmed.starts_with('(')
    {
        return (MobileSelectorFlavor::XPath, trimmed.to_string());
    }
    (MobileSelectorFlavor::Css, trimmed.to_string())
}

fn normalize_mobile_selector_for_transport(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if is_normalized_mobile_transport_selector(trimmed) {
        return trimmed.to_string();
    }
    let (flavor, body) = parse_mobile_selector_for_transport(selector);
    format!(
        "{}={}",
        flavor.as_str(),
        serde_json::to_string(&body).unwrap_or_else(|_| format!("{body:?}"))
    )
}

fn chain_mobile_selector_for_transport(parent: &str, child: &str) -> String {
    let parent_selector = if parent.trim().is_empty() {
        String::new()
    } else {
        normalize_mobile_selector_for_transport(parent)
    };
    let child_selector = if child.trim().is_empty() {
        String::new()
    } else {
        normalize_mobile_selector_for_transport(child)
    };
    if parent_selector.is_empty() {
        return child_selector;
    }
    if child_selector.is_empty() {
        return parent_selector;
    }
    format!("{parent_selector} {child_selector}")
}
