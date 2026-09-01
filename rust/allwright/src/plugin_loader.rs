use crate::plugins::invoke_plugin;
use allwright_plugin_sdk::{
    BrowserKind, BrowserLaunchInfo, BrowserSessionHandle, ChromeLaunchInfo, ChromeTabInfo,
    ChromiumBidiMapperInfo, ClickInfo, ElementCountInfo, FillInfo, FocusInfo,
    HighlightElementsInfo, HoverInfo, PageInfo, PageSessionHandle, PluginCommand, PluginEnvelope,
    PluginResult, PressKeyInfo, ScreenshotInfo, TabNavigationInfo, TextInfo, WaitForSelectorInfo,
};
use allwright_surface_mobile::{
    ConnectOptions as MobileConnectOptions, MobileBrowserSessionHandle, MobileClickInfo,
    MobileCommand, MobileCommandResult, MobileConnectInfo, MobileFillInfo, MobilePageInfo,
    MobilePageSessionHandle, MobilePlatform, MobileScreenshotInfo,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MobilePluginEnvelope {
    ok: bool,
    result: Option<MobileCommandResult>,
    error: Option<String>,
}

fn invoke_web(command: PluginCommand) -> Result<PluginResult, String> {
    let request_json = serde_json::to_string(&command)
        .map_err(|error| format!("failed to encode plugin request: {error}"))?;
    let response_json = invoke_plugin("web", &request_json)?;
    let envelope: PluginEnvelope = serde_json::from_str(&response_json)
        .map_err(|error| format!("failed to decode plugin response: {error}"))?;

    if envelope.ok {
        envelope
            .result
            .ok_or_else(|| "web plugin returned success without a result payload".to_string())
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "web plugin returned an unknown error".to_string()))
    }
}

fn invoke_mobile(command: MobileCommand) -> Result<MobileCommandResult, String> {
    let request_json = serde_json::to_string(&command)
        .map_err(|error| format!("failed to encode mobile plugin request: {error}"))?;
    let response_json = invoke_plugin(mobile_plugin_id_for_command(&command)?, &request_json)?;
    let envelope: MobilePluginEnvelope = serde_json::from_str(&response_json)
        .map_err(|error| format!("failed to decode mobile plugin response: {error}"))?;

    if envelope.ok {
        envelope
            .result
            .ok_or_else(|| "mobile plugin returned success without a result payload".to_string())
    } else {
        Err(envelope
            .error
            .unwrap_or_else(|| "mobile plugin returned an unknown error".to_string()))
    }
}

async fn invoke_web_expected(
    _command_name: &str,
    command: PluginCommand,
) -> Result<PluginResult, String> {
    tokio::task::block_in_place(move || invoke_web(command))
}

async fn invoke_mobile_expected(command: MobileCommand) -> Result<MobileCommandResult, String> {
    tokio::task::block_in_place(move || invoke_mobile(command))
}

fn mobile_plugin_id_for_command(command: &MobileCommand) -> Result<&'static str, String> {
    let platform = match command {
        MobileCommand::Connect(options) => options.platform,
        MobileCommand::LaunchApp {
            browser_session, ..
        }
        | MobileCommand::OpenPage { browser_session }
        | MobileCommand::ClosePage {
            browser_session, ..
        }
        | MobileCommand::ClickElement {
            browser_session, ..
        }
        | MobileCommand::CountElements {
            browser_session, ..
        }
        | MobileCommand::FillElement {
            browser_session, ..
        }
        | MobileCommand::GetText {
            browser_session, ..
        }
        | MobileCommand::WaitForSelector {
            browser_session, ..
        }
        | MobileCommand::Screenshot {
            browser_session, ..
        } => browser_session.platform,
    };
    mobile_plugin_id(platform)
}

fn mobile_plugin_id(platform: MobilePlatform) -> Result<&'static str, String> {
    match platform {
        MobilePlatform::Android => Ok("mobile-android"),
        MobilePlatform::Ios => Err("mobile-ios runtime plugin is not available yet".to_string()),
    }
}

pub async fn connect_mobile(options: MobileConnectOptions) -> Result<MobileConnectInfo, String> {
    match invoke_mobile_expected(MobileCommand::Connect(options)).await? {
        MobileCommandResult::Connect(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for ConnectMobile".to_string()),
    }
}

pub async fn launch_mobile_app(
    surface_session: &MobileBrowserSessionHandle,
    options: allwright_surface_mobile::LaunchOptions,
) -> Result<MobilePageInfo, String> {
    match invoke_mobile_expected(MobileCommand::LaunchApp {
        browser_session: surface_session.clone(),
        options,
    })
    .await?
    {
        MobileCommandResult::LaunchApp(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for LaunchApp".to_string()),
    }
}

pub async fn open_mobile_page(
    surface_session: &MobileBrowserSessionHandle,
) -> Result<MobilePageInfo, String> {
    match invoke_mobile_expected(MobileCommand::OpenPage {
        browser_session: surface_session.clone(),
    })
    .await?
    {
        MobileCommandResult::OpenPage(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for OpenPage".to_string()),
    }
}

pub async fn close_mobile_page(
    surface_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
) -> Result<(), String> {
    match invoke_mobile_expected(MobileCommand::ClosePage {
        browser_session: surface_session.clone(),
        page_session: page_session.clone(),
    })
    .await?
    {
        MobileCommandResult::ClosePage => Ok(()),
        _ => Err("mobile plugin returned an unexpected response for ClosePage".to_string()),
    }
}

pub async fn click_mobile_element(
    surface_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    selector: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileClickInfo, String> {
    match invoke_mobile_expected(MobileCommand::ClickElement {
        browser_session: surface_session.clone(),
        page_session: page_session.clone(),
        selector: selector.to_string(),
        timeout_ms,
    })
    .await?
    {
        MobileCommandResult::ClickElement(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for ClickElement".to_string()),
    }
}

pub async fn fill_mobile_element(
    surface_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    selector: &str,
    value: &str,
    timeout_ms: Option<u32>,
) -> Result<MobileFillInfo, String> {
    match invoke_mobile_expected(MobileCommand::FillElement {
        browser_session: surface_session.clone(),
        page_session: page_session.clone(),
        selector: selector.to_string(),
        value: value.to_string(),
        timeout_ms,
    })
    .await?
    {
        MobileCommandResult::FillElement(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for FillElement".to_string()),
    }
}

pub async fn screenshot_mobile(
    surface_session: &MobileBrowserSessionHandle,
    page_session: &MobilePageSessionHandle,
    timeout_ms: Option<u32>,
    full_page: bool,
) -> Result<MobileScreenshotInfo, String> {
    match invoke_mobile_expected(MobileCommand::Screenshot {
        browser_session: surface_session.clone(),
        page_session: page_session.clone(),
        timeout_ms,
        full_page,
    })
    .await?
    {
        MobileCommandResult::Screenshot(result) => Ok(result),
        _ => Err("mobile plugin returned an unexpected response for Screenshot".to_string()),
    }
}

pub async fn open_chrome_window(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    match invoke_web_expected(
        "LaunchChromeCommand",
        PluginCommand::OpenChromeWindow {
            chrome_binary: chrome_binary.map(str::to_string),
        },
    )
    .await?
    {
        PluginResult::OpenChromeWindow(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for OpenChromeWindow".to_string()),
    }
}

pub async fn launch_browser(
    browser_kind: BrowserKind,
    browser_binary: Option<&str>,
) -> Result<BrowserLaunchInfo, String> {
    match invoke_web_expected(
        "LaunchBrowserCommand",
        PluginCommand::LaunchBrowser {
            browser_kind,
            browser_binary: browser_binary.map(str::to_string),
        },
    )
    .await?
    {
        PluginResult::LaunchBrowser(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for LaunchBrowser".to_string()),
    }
}

pub async fn open_page(surface_session: &BrowserSessionHandle) -> Result<PageInfo, String> {
    match invoke_web_expected(
        "OpenTabCommand",
        PluginCommand::OpenPage {
            browser_session: surface_session.clone(),
        },
    )
    .await?
    {
        PluginResult::OpenPage(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for OpenPage".to_string()),
    }
}

pub async fn close_page(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
) -> Result<(), String> {
    match invoke_web_expected(
        "CloseContextSessionCommand",
        PluginCommand::ClosePage {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
        },
    )
    .await?
    {
        PluginResult::ClosePage => Ok(()),
        _ => Err("web plugin returned an unexpected response for ClosePage".to_string()),
    }
}

pub async fn navigate_page(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    match invoke_web_expected(
        "NavigatePageCommand",
        PluginCommand::NavigatePage {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            url: url.to_string(),
        },
    )
    .await?
    {
        PluginResult::NavigatePage(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for NavigatePage".to_string()),
    }
}

pub async fn click_element(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ClickInfo, String> {
    match invoke_web_expected(
        "ClickElementCommand",
        PluginCommand::ClickElement {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::ClickElement(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for ClickElement".to_string()),
    }
}

pub async fn count_elements(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    match invoke_web_expected(
        "CountElementsCommand",
        PluginCommand::CountElements {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::CountElements(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for CountElements".to_string()),
    }
}

pub async fn highlight_elements(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    duration_ms: u64,
) -> Result<HighlightElementsInfo, String> {
    match invoke_web_expected(
        "HighlightElementsCommand",
        PluginCommand::HighlightElements {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
            duration_ms,
        },
    )
    .await?
    {
        PluginResult::HighlightElements(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for HighlightElements".to_string()),
    }
}

pub async fn focus_element(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    match invoke_web_expected(
        "FocusElementCommand",
        PluginCommand::FocusElement {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::FocusElement(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for FocusElement".to_string()),
    }
}

pub async fn fill_element(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    match invoke_web_expected(
        "FillElementCommand",
        PluginCommand::FillElement {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
            value: value.to_string(),
        },
    )
    .await?
    {
        PluginResult::FillElement(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for FillElement".to_string()),
    }
}

pub async fn hover_element(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    match invoke_web_expected(
        "HoverElementCommand",
        PluginCommand::HoverElement {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::HoverElement(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for HoverElement".to_string()),
    }
}

pub async fn press_key(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    match invoke_web_expected(
        "PressKeyCommand",
        PluginCommand::PressKey {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
            key: key.to_string(),
            text: text.map(str::to_string),
        },
    )
    .await?
    {
        PluginResult::PressKey(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for PressKey".to_string()),
    }
}

pub async fn get_text_content(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetTextContentCommand",
        PluginCommand::GetTextContent {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::GetTextContent(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for GetTextContent".to_string()),
    }
}

pub async fn get_inner_text(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetInnerTextCommand",
        PluginCommand::GetInnerText {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::GetInnerText(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for GetInnerText".to_string()),
    }
}

pub async fn wait_for_selector(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    match invoke_web_expected(
        "WaitForSelectorCommand",
        PluginCommand::WaitForSelector {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            css_selector: css_selector.to_string(),
            visible,
        },
    )
    .await?
    {
        PluginResult::WaitForSelector(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for WaitForSelector".to_string()),
    }
}

pub async fn screenshot_page(
    surface_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    full_page: bool,
) -> Result<ScreenshotInfo, String> {
    match invoke_web_expected(
        "ScreenshotCommand",
        PluginCommand::Screenshot {
            browser_session: surface_session.clone(),
            page_session: page_session.clone(),
            full_page,
        },
    )
    .await?
    {
        PluginResult::Screenshot(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for Screenshot".to_string()),
    }
}

pub async fn discover_initial_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    match invoke_web_expected(
        "LaunchChromeCommand",
        PluginCommand::DiscoverInitialTab {
            cdp_websocket_url: cdp_websocket_url.to_string(),
        },
    )
    .await?
    {
        PluginResult::DiscoverInitialTab(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for DiscoverInitialTab".to_string()),
    }
}

pub async fn open_chrome_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    match invoke_web_expected(
        "OpenTabCommand",
        PluginCommand::OpenChromeTab {
            cdp_websocket_url: cdp_websocket_url.to_string(),
        },
    )
    .await?
    {
        PluginResult::OpenChromeTab(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for OpenChromeTab".to_string()),
    }
}

pub fn close_browser_process(process_id: u32) -> Result<(), String> {
    match tokio::task::block_in_place(|| {
        invoke_web(PluginCommand::CloseBrowserProcess { process_id })
    })? {
        PluginResult::CloseBrowserProcess => Ok(()),
        _ => Err("web plugin returned an unexpected response for CloseBrowserProcess".to_string()),
    }
}

pub async fn close_chrome_tab(cdp_websocket_url: &str, target_id: &str) -> Result<(), String> {
    match invoke_web_expected(
        "CloseContextSessionCommand",
        PluginCommand::CloseChromeTab {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
        },
    )
    .await?
    {
        PluginResult::CloseChromeTab => Ok(()),
        _ => Err("web plugin returned an unexpected response for CloseChromeTab".to_string()),
    }
}

pub async fn navigate_chrome_tab(
    cdp_websocket_url: &str,
    target_id: &str,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    match invoke_web_expected(
        "NavigatePageCommand",
        PluginCommand::NavigateChromeTab {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            url: url.to_string(),
        },
    )
    .await?
    {
        PluginResult::NavigateChromeTab(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for NavigateChromeTab".to_string()),
    }
}

pub async fn inject_chromium_bidi_mapper(
    cdp_websocket_url: &str,
) -> Result<ChromiumBidiMapperInfo, String> {
    match invoke_web_expected(
        "NavigatePageCommand",
        PluginCommand::InjectChromiumBidiMapper {
            cdp_websocket_url: cdp_websocket_url.to_string(),
        },
    )
    .await?
    {
        PluginResult::InjectChromiumBidiMapper(result) => Ok(result),
        _ => Err(
            "web plugin returned an unexpected response for InjectChromiumBidiMapper".to_string(),
        ),
    }
}

pub async fn resolve_bidi_context_for_tab(
    cdp_websocket_url: &str,
    mapper_target_id: Option<&str>,
    browsing_context_id: Option<&str>,
    url: Option<&str>,
) -> Result<(String, ChromiumBidiMapperInfo), String> {
    match invoke_web_expected(
        "NavigatePageCommand",
        PluginCommand::ResolveBidiContextForTab {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            mapper_target_id: mapper_target_id.map(str::to_string),
            browsing_context_id: browsing_context_id.map(str::to_string),
            url: url.map(str::to_string),
        },
    )
    .await?
    {
        PluginResult::ResolveBidiContextForTab {
            browsing_context_id,
            mapper,
        } => Ok((browsing_context_id, mapper)),
        _ => Err(
            "web plugin returned an unexpected response for ResolveBidiContextForTab".to_string(),
        ),
    }
}

pub async fn click_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<ClickInfo, String> {
    match invoke_web_expected(
        "ClickElementCommand",
        PluginCommand::ClickElementViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::ClickElementViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for ClickElementViaCdp".to_string()),
    }
}

pub async fn count_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    match invoke_web_expected(
        "CountElementsCommand",
        PluginCommand::CountElementsViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::CountElementsViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for CountElementsViaCdp".to_string()),
    }
}

pub async fn highlight_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    duration_ms: u64,
) -> Result<HighlightElementsInfo, String> {
    match invoke_web_expected(
        "HighlightElementsCommand",
        PluginCommand::HighlightElementsViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
            duration_ms,
        },
    )
    .await?
    {
        PluginResult::HighlightElementsViaCdp(result) => Ok(result),
        _ => Err(
            "web plugin returned an unexpected response for HighlightElementsViaCdp".to_string(),
        ),
    }
}

pub async fn focus_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    match invoke_web_expected(
        "FocusElementCommand",
        PluginCommand::FocusElementViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::FocusElementViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for FocusElementViaCdp".to_string()),
    }
}

pub async fn fill_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    match invoke_web_expected(
        "FillElementCommand",
        PluginCommand::FillElementViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
            value: value.to_string(),
        },
    )
    .await?
    {
        PluginResult::FillElementViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for FillElementViaCdp".to_string()),
    }
}

pub async fn hover_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    match invoke_web_expected(
        "HoverElementCommand",
        PluginCommand::HoverElementViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::HoverElementViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for HoverElementViaCdp".to_string()),
    }
}

pub async fn press_key_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    match invoke_web_expected(
        "PressKeyCommand",
        PluginCommand::PressKeyViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
            key: key.to_string(),
            text: text.map(str::to_string),
        },
    )
    .await?
    {
        PluginResult::PressKeyViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for PressKeyViaCdp".to_string()),
    }
}

pub async fn get_text_content_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetTextContentCommand",
        PluginCommand::GetTextContentViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::GetTextContentViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for GetTextContentViaCdp".to_string()),
    }
}

pub async fn get_inner_text_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetInnerTextCommand",
        PluginCommand::GetInnerTextViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
        },
    )
    .await?
    {
        PluginResult::GetInnerTextViaCdp(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for GetInnerTextViaCdp".to_string()),
    }
}

pub async fn wait_for_selector_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    match invoke_web_expected(
        "WaitForSelectorCommand",
        PluginCommand::WaitForSelectorViaCdp {
            cdp_websocket_url: cdp_websocket_url.to_string(),
            target_id: target_id.to_string(),
            css_selector: css_selector.to_string(),
            visible,
        },
    )
    .await?
    {
        PluginResult::WaitForSelectorViaCdp(result) => Ok(result),
        _ => {
            Err("web plugin returned an unexpected response for WaitForSelectorViaCdp".to_string())
        }
    }
}
