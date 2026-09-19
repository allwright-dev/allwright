pub mod accessibility_yaml;
use serde::{Deserialize, Serialize};

pub const ALLWRIGHT_PLUGIN_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceFamily {
    Web,
    Mobile,
    Desktop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chromium,
    Firefox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePluginDescriptor {
    pub id: &'static str,
    pub family: SurfaceFamily,
    pub version: &'static str,
    pub description: &'static str,
}

pub trait SurfacePlugin: Send + Sync {
    fn descriptor(&self) -> SurfacePluginDescriptor;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChromeLaunchInfo {
    pub browser: String,
    pub note: String,
    pub cdp_websocket_url: String,
    pub user_data_dir: String,
    pub process_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserLaunchInfo {
    pub browser_kind: BrowserKind,
    pub browser: String,
    pub note: String,
    pub user_data_dir: String,
    pub process_id: u32,
    pub browser_session: BrowserSessionHandle,
    pub initial_page: PageInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChromeTabInfo {
    pub note: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabNavigationInfo {
    pub url: String,
    pub note: String,
    pub page_session: PageSessionHandle,
    pub automation: AutomationSessionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChromiumBidiMapperInfo {
    pub package_version: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserSessionHandle {
    Chromium {
        cdp_websocket_url: String,
    },
    Firefox {
        connection_id: String,
        bidi_session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PageSessionHandle {
    Chromium {
        target_id: String,
        browsing_context_id: Option<String>,
        #[serde(default)]
        mapper_target_id: Option<String>,
    },
    Firefox {
        browsing_context_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageInfo {
    pub note: String,
    pub page_session: PageSessionHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookType {
    NewPage,
    FileChooser,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookRegistration {
    pub opaque_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "hook_type", content = "value", rename_all = "snake_case")]
pub enum HookResult {
    NewPage(PageInfo),
    FileChooser(FileChooserInfo),
    Download(DownloadInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChooserInfo {
    pub file_chooser_id: String,
    pub is_multiple: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChooserFilesSetInfo {
    pub file_chooser_id: String,
    pub files: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadInfo {
    pub download_id: String,
    pub url: String,
    pub suggested_filename: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadSavedInfo {
    pub download_id: String,
    pub path: String,
    pub suggested_filename: String,
    pub size: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutomationSessionInfo {
    pub bidi_session_id: String,
    pub note: String,
    pub mapper_target_id: Option<String>,
    pub mapper_session_id: Option<String>,
    pub package_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClickInfo {
    pub css_selector: String,
    pub note: String,
    pub bidi_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElementCountInfo {
    pub css_selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HighlightElementsInfo {
    pub css_selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FocusInfo {
    pub css_selector: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FillInfo {
    pub css_selector: String,
    pub value: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HoverInfo {
    pub css_selector: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PressKeyInfo {
    pub css_selector: String,
    pub key: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextInfo {
    pub css_selector: String,
    pub text: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaitForSelectorInfo {
    pub css_selector: String,
    pub visible: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessibilitySnapshotInfo {
    pub snapshot: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenshotInfo {
    pub png_data: Vec<u8>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PluginCommand {
    LaunchBrowser {
        browser_kind: BrowserKind,
        browser_binary: Option<String>,
    },
    OpenPage {
        browser_session: BrowserSessionHandle,
    },
    RegisterHook {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        hook_type: HookType,
    },
    PollHook {
        browser_session: BrowserSessionHandle,
        registration: HookRegistration,
    },
    SetFileChooserFiles {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        file_chooser_id: String,
        files: Vec<String>,
    },
    SaveDownload {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        download_id: String,
        path: String,
    },
    ClosePage {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
    },
    NavigatePage {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        url: String,
    },
    ClickElement {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    CountElements {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    HighlightElements {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
        duration_ms: u64,
    },
    FocusElement {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    FillElement {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
        value: String,
    },
    HoverElement {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    PressKey {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
        key: String,
        text: Option<String>,
    },
    GetTextContent {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    GetInnerText {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
    },
    WaitForSelector {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        css_selector: String,
        visible: bool,
    },
    AccessibilitySnapshot {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        format: String,
        #[serde(default)]
        mode: String,
    },
    Screenshot {
        browser_session: BrowserSessionHandle,
        page_session: PageSessionHandle,
        full_page: bool,
    },
    OpenChromeWindow {
        chrome_binary: Option<String>,
    },
    DiscoverInitialTab {
        cdp_websocket_url: String,
    },
    OpenChromeTab {
        cdp_websocket_url: String,
    },
    CloseBrowserProcess {
        process_id: u32,
    },
    CloseChromeTab {
        cdp_websocket_url: String,
        target_id: String,
    },
    NavigateChromeTab {
        cdp_websocket_url: String,
        target_id: String,
        url: String,
    },
    InjectChromiumBidiMapper {
        cdp_websocket_url: String,
    },
    ResolveBidiContextForTab {
        cdp_websocket_url: String,
        mapper_target_id: Option<String>,
        browsing_context_id: Option<String>,
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum PluginResult {
    LaunchBrowser(BrowserLaunchInfo),
    OpenPage(PageInfo),
    RegisterHook(HookRegistration),
    PollHook(HookResult),
    SetFileChooserFiles(FileChooserFilesSetInfo),
    SaveDownload(DownloadSavedInfo),
    ClosePage,
    NavigatePage(TabNavigationInfo),
    ClickElement(ClickInfo),
    CountElements(ElementCountInfo),
    HighlightElements(HighlightElementsInfo),
    FocusElement(FocusInfo),
    FillElement(FillInfo),
    HoverElement(HoverInfo),
    PressKey(PressKeyInfo),
    GetTextContent(TextInfo),
    GetInnerText(TextInfo),
    WaitForSelector(WaitForSelectorInfo),
    AccessibilitySnapshot(AccessibilitySnapshotInfo),
    Screenshot(ScreenshotInfo),
    OpenChromeWindow(ChromeLaunchInfo),
    DiscoverInitialTab(ChromeTabInfo),
    OpenChromeTab(ChromeTabInfo),
    CloseBrowserProcess,
    CloseChromeTab,
    NavigateChromeTab(TabNavigationInfo),
    InjectChromiumBidiMapper(ChromiumBidiMapperInfo),
    ResolveBidiContextForTab {
        browsing_context_id: String,
        mapper: ChromiumBidiMapperInfo,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginEnvelope {
    pub ok: bool,
    pub result: Option<PluginResult>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_hook_envelope_round_trips_without_duplicate_result_fields() {
        let envelope = PluginEnvelope {
            ok: true,
            result: Some(PluginResult::PollHook(HookResult::NewPage(PageInfo {
                note: "opened".to_string(),
                page_session: PageSessionHandle::Chromium {
                    target_id: "target-2".to_string(),
                    browsing_context_id: None,
                    mapper_target_id: None,
                },
            }))),
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            serde_json::from_str::<PluginEnvelope>(&json).unwrap(),
            envelope
        );
    }

    #[test]
    fn file_chooser_hook_envelope_round_trips() {
        let envelope = PluginEnvelope {
            ok: true,
            result: Some(PluginResult::PollHook(HookResult::FileChooser(
                FileChooserInfo {
                    file_chooser_id: "chooser-1".to_string(),
                    is_multiple: true,
                    note: "opened".to_string(),
                },
            ))),
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            serde_json::from_str::<PluginEnvelope>(&json).unwrap(),
            envelope
        );
    }

    #[test]
    fn download_hook_envelope_round_trips() {
        let envelope = PluginEnvelope {
            ok: true,
            result: Some(PluginResult::PollHook(HookResult::Download(DownloadInfo {
                download_id: "download-1".to_string(),
                url: "https://example.test/report.csv".to_string(),
                suggested_filename: "report.csv".to_string(),
                note: "started".to_string(),
            }))),
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            serde_json::from_str::<PluginEnvelope>(&json).unwrap(),
            envelope
        );
    }
}
