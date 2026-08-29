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
                "selector": selector,
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
                "selector": selector,
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
