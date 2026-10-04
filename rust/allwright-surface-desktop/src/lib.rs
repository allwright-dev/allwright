use allwright_plugin_sdk::{AccessibilitySnapshotInfo, SurfaceFamily, SurfacePluginDescriptor};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

pub const SURFACE_ID: &str = "desktop";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPlatform {
    Mac,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectOptions {
    pub platform: DesktopPlatform,
    pub agent_endpoint: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchOptions {
    pub app_id: String,
    pub terminate_running: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSessionHandle {
    pub platform: DesktopPlatform,
    pub endpoint: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopAppSessionHandle {
    pub app_session_id: String,
    pub app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopAppInfo {
    pub note: String,
    pub app_session: DesktopAppSessionHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopConnectInfo {
    pub host_name: String,
    pub note: String,
    pub desktop_session: DesktopSessionHandle,
    pub initial_app: DesktopAppInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopActionInfo {
    pub selector: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopElementCountInfo {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopTextInfo {
    pub selector: String,
    pub text: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopWaitInfo {
    pub selector: String,
    pub visible: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopScreenshotInfo {
    pub png_data: Vec<u8>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum DesktopCommand {
    Connect(ConnectOptions),
    LaunchApp {
        desktop_session: DesktopSessionHandle,
        options: LaunchOptions,
    },
    CloseApp {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
    },
    ClickElement {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    CountElements {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    FocusElement {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    FillElement {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        value: String,
        timeout_ms: Option<u32>,
    },
    PressKey {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        key: String,
        text: Option<String>,
        timeout_ms: Option<u32>,
    },
    GetText {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    GetInnerText {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        timeout_ms: Option<u32>,
    },
    WaitForSelector {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        selector: String,
        visible: bool,
        timeout_ms: Option<u32>,
    },
    Screenshot {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        timeout_ms: Option<u32>,
    },
    AccessibilitySnapshot {
        desktop_session: DesktopSessionHandle,
        app_session: DesktopAppSessionHandle,
        format: String,
        mode: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum DesktopCommandResult {
    Connect(DesktopConnectInfo),
    LaunchApp(DesktopAppInfo),
    CloseApp,
    ClickElement(DesktopActionInfo),
    CountElements(DesktopElementCountInfo),
    FocusElement(DesktopActionInfo),
    FillElement(DesktopActionInfo),
    PressKey(DesktopActionInfo),
    GetText(DesktopTextInfo),
    GetInnerText(DesktopTextInfo),
    WaitForSelector(DesktopWaitInfo),
    Screenshot(DesktopScreenshotInfo),
    AccessibilitySnapshot(AccessibilitySnapshotInfo),
}

pub fn shared_descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: SURFACE_ID,
        family: SurfaceFamily::Desktop,
        version: env!("CARGO_PKG_VERSION"),
        description: "Shared desktop surface abstractions for macOS, Windows, and Linux plugins.",
    }
}

pub async fn boot_surface(label: &str, delay_ms: u64) -> String {
    sleep(Duration::from_millis(delay_ms)).await;
    format!("{label} ready")
}

pub async fn boot() -> String {
    boot_surface("desktop", 20).await
}

pub fn normalize_selector_for_transport(selector: &str) -> String {
    let trimmed = selector.trim();
    for prefix in ["css=", "xpath=", "aw=", "uia="] {
        if let Some(value) = trimmed.strip_prefix(prefix) {
            if serde_json::from_str::<String>(value).is_ok() {
                return trimmed.to_string();
            }
            return format!("{prefix}{}", serde_json::to_string(value).unwrap());
        }
    }
    if let Some(value) = trimmed.strip_prefix("id=") {
        return format!(
            "css={}",
            serde_json::to_string(&format!("#{value}")).unwrap()
        );
    }
    format!("css={}", serde_json::to_string(trimmed).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_desktop_runtime() {
        assert_eq!(boot().await, "desktop ready");
    }

    #[test]
    fn command_round_trips() {
        let command = DesktopCommand::Connect(ConnectOptions {
            platform: DesktopPlatform::Mac,
            agent_endpoint: None,
            timeout_ms: Some(1_000),
        });
        let json = serde_json::to_string(&command).unwrap();
        assert_eq!(
            serde_json::from_str::<DesktopCommand>(&json).unwrap(),
            command
        );
    }
}
