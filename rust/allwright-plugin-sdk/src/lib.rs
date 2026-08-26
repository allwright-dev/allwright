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
    pub browsing_context_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChromiumBidiMapperInfo {
    pub package_version: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClickInfo {
    pub css_selector: String,
    pub note: String,
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
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PluginCommand {
    LaunchBrowser {
        browser_kind: BrowserKind,
        browser_binary: Option<String>,
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
    ClickElementViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    CountElementsViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    HighlightElementsViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
        duration_ms: u64,
    },
    FocusElementViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    FillElementViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
        value: String,
    },
    HoverElementViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    PressKeyViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
        key: String,
        text: Option<String>,
    },
    GetTextContentViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    GetInnerTextViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
    },
    WaitForSelectorViaCdp {
        cdp_websocket_url: String,
        target_id: String,
        css_selector: String,
        visible: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum PluginResult {
    LaunchBrowser(BrowserLaunchInfo),
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
    ClickElementViaCdp(ClickInfo),
    CountElementsViaCdp(ElementCountInfo),
    HighlightElementsViaCdp(HighlightElementsInfo),
    FocusElementViaCdp(FocusInfo),
    FillElementViaCdp(FillInfo),
    HoverElementViaCdp(HoverInfo),
    PressKeyViaCdp(PressKeyInfo),
    GetTextContentViaCdp(TextInfo),
    GetInnerTextViaCdp(TextInfo),
    WaitForSelectorViaCdp(WaitForSelectorInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginEnvelope {
    pub ok: bool,
    pub result: Option<PluginResult>,
    pub error: Option<String>,
}
