use crate::plugins::package;
use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, BrowserKind, BrowserLaunchInfo, BrowserSessionHandle,
    ChromeLaunchInfo, ChromeTabInfo, ChromiumBidiMapperInfo, ClickInfo, ElementCountInfo, FillInfo,
    FocusInfo, HighlightElementsInfo, HoverInfo, PageInfo, PageSessionHandle, PluginCommand,
    PluginEnvelope, PluginResult, PressKeyInfo, TabNavigationInfo, TextInfo, WaitForSelectorInfo,
};
use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;

type PluginApiVersionFn = unsafe extern "C" fn() -> u32;
type PluginIdFn = unsafe extern "C" fn() -> *const c_char;
type PluginInvokeFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut c_char);

fn plugin_home() -> Result<PathBuf, String> {
    if let Ok(home) = env::var("ALLWRIGHT_HOME") {
        return Ok(PathBuf::from(home));
    }

    let home =
        env::var("HOME").map_err(|_| "HOME is not set and ALLWRIGHT_HOME was not provided")?;
    Ok(PathBuf::from(home).join(".allwright"))
}

fn web_plugin_library_filename() -> &'static str {
    match env::consts::OS {
        "macos" => "liballwright_surface_web.dylib",
        "linux" => "liballwright_surface_web.so",
        "windows" => "allwright_surface_web.dll",
        _ => "allwright_surface_web.unknown",
    }
}

fn web_plugin_library_path() -> Result<PathBuf, String> {
    Ok(plugin_home()?
        .join("plugins")
        .join("web")
        .join("lib")
        .join(web_plugin_library_filename()))
}

fn invoke_web(command: PluginCommand) -> Result<PluginResult, String> {
    let library_path = web_plugin_library_path()?;
    if !library_path.exists() {
        return Err("web plugin is not installed".to_string());
    }

    let request_json = serde_json::to_string(&command)
        .map_err(|error| format!("failed to encode plugin request: {error}"))?;
    let request_cstr = CString::new(request_json)
        .map_err(|error| format!("plugin request contains NUL: {error}"))?;

    let library = unsafe { Library::new(&library_path) }.map_err(|error| {
        format!(
            "failed to load web plugin library {:?}: {error}",
            library_path
        )
    })?;

    unsafe {
        let api_version: Symbol<'_, PluginApiVersionFn> = library
            .get(b"allwright_plugin_api_version")
            .map_err(|error| format!("failed to load web plugin api version symbol: {error}"))?;
        if api_version() != ALLWRIGHT_PLUGIN_API_VERSION {
            return Err(format!(
                "web plugin ABI version mismatch: expected {}, got {}",
                ALLWRIGHT_PLUGIN_API_VERSION,
                api_version()
            ));
        }

        let plugin_id: Symbol<'_, PluginIdFn> = library
            .get(b"allwright_plugin_id")
            .map_err(|error| format!("failed to load web plugin id symbol: {error}"))?;
        let raw_id = plugin_id();
        if raw_id.is_null() {
            return Err("web plugin returned a null plugin id".to_string());
        }
        let plugin_id = CStr::from_ptr(raw_id)
            .to_str()
            .map_err(|error| format!("web plugin id is not valid UTF-8: {error}"))?;
        if plugin_id != "web" {
            return Err(format!(
                "unexpected plugin id `{plugin_id}` loaded for web surface"
            ));
        }

        let invoke: Symbol<'_, PluginInvokeFn> = library
            .get(b"allwright_plugin_invoke")
            .map_err(|error| format!("failed to load web plugin invoke symbol: {error}"))?;
        let free_string: Symbol<'_, PluginFreeStringFn> = library
            .get(b"allwright_plugin_free_string")
            .map_err(|error| format!("failed to load web plugin free-string symbol: {error}"))?;

        let response_ptr = invoke(request_cstr.as_ptr());
        if response_ptr.is_null() {
            return Err("web plugin returned a null response".to_string());
        }

        let response_json = CStr::from_ptr(response_ptr)
            .to_str()
            .map_err(|error| format!("web plugin response is not valid UTF-8: {error}"))?
            .to_string();
        free_string(response_ptr);

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
}

fn plugin_required_error(plugin_id: &str, command_name: &str) -> String {
    let package_name = package(plugin_id)
        .map(|package| package.package_name)
        .unwrap_or("plugin");
    format!(
        "{command_name} requires the `{plugin_id}` surface plugin. Install it with `allwright plugin install {plugin_id}` to download `{package_name}`."
    )
}

async fn invoke_web_expected(
    command_name: &str,
    command: PluginCommand,
) -> Result<PluginResult, String> {
    tokio::task::block_in_place(move || match invoke_web(command) {
        Ok(result) => Ok(result),
        Err(error) if error == "web plugin is not installed" => {
            Err(plugin_required_error("web", command_name))
        }
        Err(error) => Err(error),
    })
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

pub async fn open_page(browser_session: &BrowserSessionHandle) -> Result<PageInfo, String> {
    match invoke_web_expected(
        "OpenTabCommand",
        PluginCommand::OpenPage {
            browser_session: browser_session.clone(),
        },
    )
    .await?
    {
        PluginResult::OpenPage(result) => Ok(result),
        _ => Err("web plugin returned an unexpected response for OpenPage".to_string()),
    }
}

pub async fn close_page(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
) -> Result<(), String> {
    match invoke_web_expected(
        "CloseTabSessionCommand",
        PluginCommand::ClosePage {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    match invoke_web_expected(
        "NavigateTabCommand",
        PluginCommand::NavigatePage {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ClickInfo, String> {
    match invoke_web_expected(
        "ClickElementCommand",
        PluginCommand::ClickElement {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    match invoke_web_expected(
        "CountElementsCommand",
        PluginCommand::CountElements {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    duration_ms: u64,
) -> Result<HighlightElementsInfo, String> {
    match invoke_web_expected(
        "HighlightElementsCommand",
        PluginCommand::HighlightElements {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    match invoke_web_expected(
        "FocusElementCommand",
        PluginCommand::FocusElement {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    match invoke_web_expected(
        "FillElementCommand",
        PluginCommand::FillElement {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    match invoke_web_expected(
        "HoverElementCommand",
        PluginCommand::HoverElement {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    match invoke_web_expected(
        "PressKeyCommand",
        PluginCommand::PressKey {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetTextContentCommand",
        PluginCommand::GetTextContent {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match invoke_web_expected(
        "GetInnerTextCommand",
        PluginCommand::GetInnerText {
            browser_session: browser_session.clone(),
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
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    match invoke_web_expected(
        "WaitForSelectorCommand",
        PluginCommand::WaitForSelector {
            browser_session: browser_session.clone(),
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
        let command = PluginCommand::CloseBrowserProcess { process_id };
        match invoke_web(command) {
            Ok(result) => Ok(result),
            Err(error) if error == "web plugin is not installed" => {
                Err(plugin_required_error("web", "CloseBrowserSessionCommand"))
            }
            Err(error) => Err(error),
        }
    })? {
        PluginResult::CloseBrowserProcess => Ok(()),
        _ => Err("web plugin returned an unexpected response for CloseBrowserProcess".to_string()),
    }
}

pub async fn close_chrome_tab(cdp_websocket_url: &str, target_id: &str) -> Result<(), String> {
    match invoke_web_expected(
        "CloseTabSessionCommand",
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
        "NavigateTabCommand",
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
        "NavigateTabCommand",
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
        "NavigateTabCommand",
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
