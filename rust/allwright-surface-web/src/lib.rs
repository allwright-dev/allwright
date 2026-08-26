use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, BrowserKind, BrowserLaunchInfo, ChromeLaunchInfo, ChromeTabInfo,
    ChromiumBidiMapperInfo, ClickInfo, ElementCountInfo, FillInfo, FocusInfo,
    HighlightElementsInfo, HoverInfo, PluginCommand, PluginEnvelope, PluginResult, PressKeyInfo,
    SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor, TabNavigationInfo, TextInfo,
    WaitForSelectorInfo,
};
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq)]
struct DiscoveredElements {
    count: u32,
    first_center: Option<ElementCenter>,
}

#[derive(Debug, Clone, PartialEq)]
struct ElementCenter {
    x: f64,
    y: f64,
}

const CHROMIUM_BIDI_NPM_VERSION: &str = "17.0.2";
const CHROMIUM_BIDI_MAPPER_BUNDLE: &str =
    include_str!("../third_party/chromium-bidi/17.0.2/mapperTab.js");

#[derive(Debug, Clone, Copy, Default)]
pub struct WebPlugin;

impl SurfacePlugin for WebPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "web",
            family: SurfaceFamily::Web,
            version: env!("CARGO_PKG_VERSION"),
            description: "Web surface plugin for the allwright engine.",
        }
    }
}

pub fn open_chrome_window(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    launch_chrome_for_platform(chrome_binary.filter(|value| !value.trim().is_empty()))
}

pub async fn discover_initial_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;

    for _ in 0..100 {
        let targets = cdp
            .send_command("Target.getTargets", json!({}), None)
            .await?;
        if let Some(page_target) = targets
            .get("targetInfos")
            .and_then(Value::as_array)
            .and_then(|target_infos| {
                target_infos.iter().find(|target_info| {
                    target_info.get("type").and_then(Value::as_str) == Some("page")
                })
            })
        {
            let target_id = required_string(page_target, "/targetId")?;
            return Ok(ChromeTabInfo {
                note: "tracked initial Chrome tab via CDP".to_string(),
                target_id,
            });
        }

        sleep(Duration::from_millis(50)).await;
    }

    Err("timed out waiting for the initial Chrome page target over CDP".to_string())
}

pub async fn open_chrome_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let create_target = cdp
        .send_command(
            "Target.createTarget",
            json!({
                "url": "about:blank",
                "background": false,
            }),
            None,
        )
        .await?;
    let target_id = required_string(&create_target, "/targetId")?;

    Ok(ChromeTabInfo {
        note: "opened Chrome tab via CDP".to_string(),
        target_id,
    })
}

pub fn close_browser_process(process_id: u32) -> Result<(), String> {
    close_browser_process_for_platform(process_id)
}

pub async fn close_chrome_tab(cdp_websocket_url: &str, target_id: &str) -> Result<(), String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let response = cdp
        .send_command(
            "Target.closeTarget",
            json!({
                "targetId": target_id,
            }),
            None,
        )
        .await?;
    let success = response
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        Err(format!(
            "CDP Target.closeTarget reported unsuccessful close for target {target_id}"
        ))
    }
}

pub async fn navigate_chrome_tab(
    cdp_websocket_url: &str,
    target_id: &str,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    if url.trim().is_empty() {
        return Err("navigate command requires a non-empty url".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let attach = cdp
        .send_command(
            "Target.attachToTarget",
            json!({
                "targetId": target_id,
                "flatten": true,
            }),
            None,
        )
        .await?;
    let session_id = required_string(&attach, "/sessionId")?;

    cdp.send_command("Page.enable", json!({}), Some(&session_id))
        .await?;
    cdp.navigate_and_wait_for_load(&session_id, url).await?;
    let frame_tree = cdp
        .send_command("Page.getFrameTree", json!({}), Some(&session_id))
        .await?;
    let browsing_context_id = required_string(&frame_tree, "/frameTree/frame/id")?;
    cdp.send_command(
        "Target.detachFromTarget",
        json!({
            "sessionId": session_id,
        }),
        None,
    )
    .await?;

    Ok(TabNavigationInfo {
        url: url.to_string(),
        note: "navigated Chrome tab via CDP and observed Page.loadEventFired".to_string(),
        browsing_context_id,
    })
}

pub async fn inject_chromium_bidi_mapper(
    cdp_websocket_url: &str,
) -> Result<ChromiumBidiMapperInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let mapper_target = cdp
        .send_command(
            "Target.createTarget",
            json!({
                "url": "about:blank#MAPPER_TARGET",
                "hidden": true,
                "background": true,
            }),
            None,
        )
        .await?;
    let mapper_target_id = required_string(&mapper_target, "/targetId")?;

    let attached = cdp
        .send_command(
            "Target.attachToTarget",
            json!({
                "targetId": mapper_target_id,
                "flatten": true,
            }),
            None,
        )
        .await?;
    let mapper_session_id = required_string(&attached, "/sessionId")?;

    cdp.send_command("Runtime.enable", json!({}), Some(&mapper_session_id))
        .await?;
    cdp.send_command(
        "Target.exposeDevToolsProtocol",
        json!({
            "bindingName": "cdp",
            "targetId": mapper_target_id,
            "inheritPermissions": true,
        }),
        None,
    )
    .await?;
    cdp.send_command(
        "Runtime.addBinding",
        json!({
            "name": "sendBidiResponse",
        }),
        Some(&mapper_session_id),
    )
    .await?;
    cdp.evaluate_expression(&mapper_session_id, CHROMIUM_BIDI_MAPPER_BUNDLE, false)
        .await?;
    cdp.evaluate_expression(
        &mapper_session_id,
        &format!(
            "window.runMapperInstance('{}')",
            js_single_quote(&mapper_target_id)
        ),
        true,
    )
    .await?;

    Ok(ChromiumBidiMapperInfo {
        package_version: CHROMIUM_BIDI_NPM_VERSION.to_string(),
        mapper_target_id,
        mapper_session_id,
        note: format!(
            "injected chromium-bidi mapper from pinned published chromium-bidi@{} artifact into hidden mapper target",
            CHROMIUM_BIDI_NPM_VERSION
        ),
    })
}

pub async fn click_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<ClickInfo, String> {
    if css_selector.trim().is_empty() {
        return Err("click_element command requires a non-empty css_selector".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered =
        discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, true).await?;
    let center = discovered.first_center.ok_or_else(|| {
        format!("element discovery did not return a clickable center for selector {css_selector}")
    })?;
    cdp.dispatch_mouse_click(&session_id, center.x, center.y)
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(ClickInfo {
        css_selector: css_selector.to_string(),
        note: format!("clicked element via CDP mouse events using css selector {css_selector}"),
    })
}

pub async fn count_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    if css_selector.trim().is_empty() {
        return Err("count_elements command requires a non-empty css_selector".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered =
        discover_elements_via_cdp(&mut cdp, &session_id, css_selector, false, false).await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(ElementCountInfo {
        css_selector: css_selector.to_string(),
        count: discovered.count,
        note: format!(
            "counted {} element(s) matching css selector {css_selector}",
            discovered.count
        ),
    })
}

pub async fn highlight_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    duration_ms: u32,
) -> Result<HighlightElementsInfo, String> {
    if css_selector.trim().is_empty() {
        return Err("highlight_elements command requires a non-empty css_selector".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered =
        discover_elements_via_cdp(&mut cdp, &session_id, css_selector, false, false).await?;

    let selector_literal = serde_json::to_string(css_selector)
        .map_err(|error| format!("failed to serialize css selector for highlight: {error}"))?;
    let duration_ms = duration_ms.max(1);
    cdp.evaluate_expression(
        &session_id,
        &format!(
            "(() => {{
                const selector = {selector_literal};
                const durationMs = {duration_ms};
                const elements = Array.from(document.querySelectorAll(selector));
                for (const element of elements) {{
                    if (!(element instanceof HTMLElement)) {{
                        continue;
                    }}
                    element.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
                    const priorOutline = element.style.outline;
                    const priorOutlineOffset = element.style.outlineOffset;
                    const priorBackgroundColor = element.style.backgroundColor;
                    element.style.outline = '3px solid #ff5a36';
                    element.style.outlineOffset = '2px';
                    element.style.backgroundColor = 'rgba(255, 235, 59, 0.35)';
                    window.setTimeout(() => {{
                        element.style.outline = priorOutline;
                        element.style.outlineOffset = priorOutlineOffset;
                        element.style.backgroundColor = priorBackgroundColor;
                    }}, durationMs);
                }}
                return elements.length;
            }})()"
        ),
        true,
    )
    .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(HighlightElementsInfo {
        css_selector: css_selector.to_string(),
        count: discovered.count,
        note: format!(
            "highlighted {} element(s) matching css selector {css_selector} for {duration_ms}ms",
            discovered.count
        ),
    })
}

pub async fn focus_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    require_non_empty_selector("focus_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, true).await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(FocusInfo {
        css_selector: css_selector.to_string(),
        note: format!("focused element matching css selector {css_selector}"),
    })
}

pub async fn fill_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    require_non_empty_selector("fill_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, true).await?;
    let selector_literal = json_string_literal(css_selector, "fill selector")?;
    let value_literal = json_string_literal(value, "fill value")?;
    cdp.evaluate_expression(
        &session_id,
        &format!(
            "(() => {{
                const selector = {selector_literal};
                const value = {value_literal};
                const element = document.querySelector(selector);
                if (!element) {{
                    throw new Error(`No element matches selector: ${{selector}}`);
                }}
                if (!(element instanceof HTMLElement)) {{
                    throw new Error(`Element is not focusable: ${{selector}}`);
                }}
                element.focus();
                if ('value' in element) {{
                    element.value = value;
                }} else if (element.isContentEditable) {{
                    element.textContent = value;
                }} else {{
                    throw new Error(`Element does not support fill: ${{selector}}`);
                }}
                element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                element.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"
        ),
        true,
    )
    .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(FillInfo {
        css_selector: css_selector.to_string(),
        value: value.to_string(),
        note: format!("filled element matching css selector {css_selector}"),
    })
}

pub async fn hover_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    require_non_empty_selector("hover_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered =
        discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, false).await?;
    let center = discovered.first_center.ok_or_else(|| {
        format!("element discovery did not return a hover center for selector {css_selector}")
    })?;
    cdp.dispatch_mouse_move(&session_id, center.x, center.y)
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(HoverInfo {
        css_selector: css_selector.to_string(),
        note: format!("hovered element matching css selector {css_selector}"),
    })
}

pub async fn press_key_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    require_non_empty_selector("press_key", css_selector)?;
    if key.trim().is_empty() {
        return Err("press_key command requires a non-empty key".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, true).await?;
    cdp.dispatch_key_press(&session_id, key, text).await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(PressKeyInfo {
        css_selector: css_selector.to_string(),
        key: key.to_string(),
        note: format!("pressed key {key} on element matching css selector {css_selector}"),
    })
}

pub async fn get_text_content_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<TextInfo, String> {
    get_text_via_cdp(cdp_websocket_url, target_id, css_selector, "textContent")
        .await
        .map(|mut info| {
            info.note = format!("resolved textContent for css selector {css_selector}");
            info
        })
}

pub async fn get_inner_text_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<TextInfo, String> {
    get_text_via_cdp(cdp_websocket_url, target_id, css_selector, "innerText")
        .await
        .map(|mut info| {
            info.note = format!("resolved innerText for css selector {css_selector}");
            info
        })
}

pub async fn wait_for_selector_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    require_non_empty_selector("wait_for_selector", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered =
        discover_elements_via_cdp(&mut cdp, &session_id, css_selector, false, false).await?;
    let success = if visible {
        discovered.first_center.is_some()
    } else {
        discovered.count > 0
    };
    cdp.detach_from_target(&session_id).await?;

    if !success {
        return Err(if visible {
            format!("no visible element matches css selector {css_selector}")
        } else {
            format!("no element matches css selector {css_selector}")
        });
    }

    Ok(WaitForSelectorInfo {
        css_selector: css_selector.to_string(),
        visible,
        note: if visible {
            format!("visible element matched css selector {css_selector}")
        } else {
            format!("element matched css selector {css_selector}")
        },
    })
}

async fn discover_elements_via_cdp(
    cdp: &mut CdpConnection,
    session_id: &str,
    css_selector: &str,
    require_match: bool,
    focus_first_match: bool,
) -> Result<DiscoveredElements, String> {
    let selector_literal = serde_json::to_string(css_selector)
        .map_err(|error| format!("failed to serialize css selector for discovery: {error}"))?;
    let expression = format!(
        "(() => {{
            const selector = {selector_literal};
            const elements = Array.from(document.querySelectorAll(selector));
            const first = elements[0] ?? null;
            if (first) {{
                first.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
                if ({focus_first_match} && first instanceof HTMLElement) {{
                    first.focus();
                }}
            }}
            if ({require_match} && !first) {{
                throw new Error(`No element matches selector: ${{selector}}`);
            }}
            if (first && !(first instanceof Element)) {{
                throw new Error(`Selector did not resolve to a DOM Element: ${{selector}}`);
            }}
            const rect = first ? first.getBoundingClientRect() : null;
            if (first && (!rect.width || !rect.height)) {{
                throw new Error(`Element matched by selector has zero size: ${{selector}}`);
            }}
            return {{
                count: elements.length,
                first: rect ? {{
                    x: rect.left + (rect.width / 2),
                    y: rect.top + (rect.height / 2)
                }} : null
            }};
        }})()"
    );
    let result = cdp
        .evaluate_expression(session_id, &expression, true)
        .await?;
    let count = json_u32(&result, "/result/value/count")?;
    let first_center = if result.pointer("/result/value/first").is_some()
        && !result
            .pointer("/result/value/first")
            .is_some_and(Value::is_null)
    {
        Some(ElementCenter {
            x: json_f64(&result, "/result/value/first/x")?,
            y: json_f64(&result, "/result/value/first/y")?,
        })
    } else {
        None
    };

    Ok(DiscoveredElements {
        count,
        first_center,
    })
}

async fn get_text_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    property: &str,
) -> Result<TextInfo, String> {
    require_non_empty_selector("get_text", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(&mut cdp, &session_id, css_selector, true, false).await?;
    let selector_literal = json_string_literal(css_selector, "text selector")?;
    let property_literal = json_string_literal(property, "text property")?;
    let result = cdp
        .evaluate_expression(
            &session_id,
            &format!(
                "(() => {{
                    const selector = {selector_literal};
                    const property = {property_literal};
                    const element = document.querySelector(selector);
                    if (!element) {{
                        throw new Error(`No element matches selector: ${{selector}}`);
                    }}
                    const value = element[property];
                    return typeof value === 'string' ? value : '';
                }})()"
            ),
            true,
        )
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(TextInfo {
        css_selector: css_selector.to_string(),
        text: json_string(&result, "/result/value")?,
        note: String::new(),
    })
}

pub async fn resolve_bidi_context_for_tab(
    cdp_websocket_url: &str,
    existing_mapper_target_id: Option<&str>,
    existing_context_id: Option<&str>,
    current_url: Option<&str>,
) -> Result<(String, ChromiumBidiMapperInfo), String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let mapper = cdp
        .ensure_chromium_bidi_mapper(existing_mapper_target_id)
        .await?;
    let context_id = cdp
        .resolve_bidi_context_id(&mapper.mapper_session_id, existing_context_id, current_url)
        .await?;

    Ok((
        context_id,
        ChromiumBidiMapperInfo {
            package_version: mapper.package_version,
            mapper_target_id: mapper.mapper_target_id,
            mapper_session_id: mapper.mapper_session_id,
            note: "resolved BiDi browsing context for tab".to_string(),
        },
    ))
}

#[cfg(target_os = "macos")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    let browser_binary = chrome_binary.map(ToOwned::to_owned).unwrap_or_else(|| {
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string()
    });
    launch_chrome_with_cdp(&browser_binary)
}

#[cfg(target_os = "linux")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    if let Some(chrome_binary) = chrome_binary {
        return launch_chrome_with_cdp(chrome_binary);
    }

    let candidates = ["google-chrome", "chromium", "chromium-browser"];

    for candidate in candidates {
        match launch_chrome_with_cdp(candidate) {
            Ok(launch_info) => return Ok(launch_info),
            Err(error) => {
                if error.contains("No such file or directory")
                    || error.contains("os error 2")
                    || error.contains("not found")
                {
                    continue;
                }
                return Err(format!("failed to launch {candidate}: {error}"));
            }
        }
    }

    Err("could not find a Chrome-compatible browser binary".to_string())
}

#[cfg(target_os = "windows")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    if let Some(chrome_binary) = chrome_binary {
        return launch_chrome_with_cdp(chrome_binary);
    }

    launch_chrome_with_cdp("chrome")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    match chrome_binary {
        Some(chrome_binary) => launch_chrome_with_cdp(chrome_binary),
        None => Err("Chrome launching is not supported on this platform yet".to_string()),
    }
}

fn launch_chrome_with_cdp(browser_binary: &str) -> Result<ChromeLaunchInfo, String> {
    let user_data_dir = create_chrome_user_data_dir()?;
    let user_data_dir_str = user_data_dir.to_string_lossy().to_string();

    let child = Command::new(browser_binary)
        .args([
            "--new-window",
            "--remote-debugging-port=0",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-sync",
            &format!("--user-data-dir={user_data_dir_str}"),
        ])
        .spawn()
        .map_err(|error| format!("failed to launch Chrome binary {browser_binary}: {error}"))?;
    let process_id = child.id();

    let cdp_websocket_url = wait_for_cdp_endpoint(&user_data_dir)?;

    Ok(ChromeLaunchInfo {
        browser: browser_binary.to_string(),
        note: "launched Chrome window with CDP enabled".to_string(),
        cdp_websocket_url,
        user_data_dir: user_data_dir_str,
        process_id,
    })
}

fn create_chrome_user_data_dir() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute timestamp: {error}"))?
        .as_millis();
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("allwright-chrome-{pid}-{timestamp}"));
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create Chrome user data dir {}: {error}",
            dir.display()
        )
    })?;
    Ok(dir)
}

fn wait_for_cdp_endpoint(user_data_dir: &std::path::Path) -> Result<String, String> {
    let active_port_file = user_data_dir.join("DevToolsActivePort");

    for _ in 0..100 {
        if active_port_file.exists() {
            let contents = fs::read_to_string(&active_port_file).map_err(|error| {
                format!(
                    "failed to read DevToolsActivePort file {}: {error}",
                    active_port_file.display()
                )
            })?;
            return parse_devtools_active_port(&contents);
        }
        thread::sleep(StdDuration::from_millis(50));
    }

    Err(format!(
        "timed out waiting for DevToolsActivePort in {}",
        user_data_dir.display()
    ))
}

fn parse_devtools_active_port(contents: &str) -> Result<String, String> {
    let mut lines = contents.lines();
    let port = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "DevToolsActivePort missing port line".to_string())?;
    let browser_path = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "DevToolsActivePort missing websocket path line".to_string())?;

    Ok(format!("ws://127.0.0.1:{port}{browser_path}"))
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(15)).await;
    "web ready".to_string()
}

#[cfg(unix)]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    let status = Command::new("kill")
        .args(["-TERM", &process_id.to_string()])
        .status()
        .map_err(|error| format!("failed to send TERM to process {process_id}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "kill -TERM returned non-zero exit status for process {process_id}: {status}"
        ))
    }
}

#[cfg(windows)]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("failed to terminate process {process_id}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "taskkill returned non-zero exit status for process {process_id}: {status}"
        ))
    }
}

#[cfg(not(any(unix, windows)))]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    Err(format!(
        "browser process termination is not implemented for process {process_id} on this platform"
    ))
}

type CdpSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct MapperConnectionInfo {
    package_version: String,
    mapper_target_id: String,
    mapper_session_id: String,
}

struct CdpConnection {
    socket: CdpSocket,
    next_id: u64,
}

impl CdpConnection {
    async fn connect(cdp_websocket_url: &str) -> Result<Self, String> {
        let (socket, _) = connect_async(cdp_websocket_url).await.map_err(|error| {
            format!("failed to connect to CDP websocket {cdp_websocket_url}: {error}")
        })?;
        Ok(Self { socket, next_id: 1 })
    }

    async fn send_command(
        &mut self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let mut payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(session_id) = session_id {
            payload["sessionId"] = Value::String(session_id.to_string());
        }

        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send CDP command {method}: {error}"))?;

        loop {
            let message = self.next_json_message().await?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error") {
                return Err(format!("CDP command {method} failed: {error}"));
            }

            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn evaluate_expression(
        &mut self,
        session_id: &str,
        expression: &str,
        await_promise: bool,
    ) -> Result<Value, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "awaitPromise": await_promise,
                    "returnByValue": true,
                }),
                Some(session_id),
            )
            .await?;
        if let Some(exception_details) = result.get("exceptionDetails") {
            return Err(format!(
                "Runtime.evaluate failed with exception details: {exception_details}"
            ));
        }
        Ok(result)
    }

    async fn attach_to_target(&mut self, target_id: &str) -> Result<String, String> {
        let attached = self
            .send_command(
                "Target.attachToTarget",
                json!({
                    "targetId": target_id,
                    "flatten": true,
                }),
                None,
            )
            .await?;
        required_string(&attached, "/sessionId")
    }

    async fn prepare_page_target_session(&mut self, target_id: &str) -> Result<String, String> {
        let session_id = self.attach_to_target(target_id).await?;
        self.send_command("Runtime.enable", json!({}), Some(&session_id))
            .await?;
        self.send_command("Page.enable", json!({}), Some(&session_id))
            .await?;
        Ok(session_id)
    }

    async fn detach_from_target(&mut self, session_id: &str) -> Result<(), String> {
        self.send_command(
            "Target.detachFromTarget",
            json!({
                "sessionId": session_id,
            }),
            None,
        )
        .await?;
        Ok(())
    }

    async fn dispatch_mouse_click(
        &mut self,
        session_id: &str,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        self.dispatch_mouse_move(session_id, x, y).await?;
        self.send_command(
            "Input.dispatchMouseEvent",
            json!({
                "type": "mousePressed",
                "x": x,
                "y": y,
                "button": "left",
                "buttons": 1,
                "clickCount": 1,
            }),
            Some(session_id),
        )
        .await?;
        self.send_command(
            "Input.dispatchMouseEvent",
            json!({
                "type": "mouseReleased",
                "x": x,
                "y": y,
                "button": "left",
                "buttons": 0,
                "clickCount": 1,
            }),
            Some(session_id),
        )
        .await?;
        Ok(())
    }

    async fn dispatch_mouse_move(
        &mut self,
        session_id: &str,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        self.send_command(
            "Input.dispatchMouseEvent",
            json!({
                "type": "mouseMoved",
                "x": x,
                "y": y,
                "button": "none",
                "buttons": 0,
            }),
            Some(session_id),
        )
        .await?;
        Ok(())
    }

    async fn dispatch_key_press(
        &mut self,
        session_id: &str,
        key: &str,
        text: Option<&str>,
    ) -> Result<(), String> {
        self.send_command(
            "Input.dispatchKeyEvent",
            json!({
                "type": "keyDown",
                "key": key,
                "text": text.unwrap_or(""),
            }),
            Some(session_id),
        )
        .await?;
        self.send_command(
            "Input.dispatchKeyEvent",
            json!({
                "type": "keyUp",
                "key": key,
                "text": text.unwrap_or(""),
            }),
            Some(session_id),
        )
        .await?;
        Ok(())
    }

    async fn ensure_chromium_bidi_mapper(
        &mut self,
        existing_mapper_target_id: Option<&str>,
    ) -> Result<MapperConnectionInfo, String> {
        let (mut mapper_target_id, mut created_target) = match existing_mapper_target_id {
            Some(existing_mapper_target_id) if !existing_mapper_target_id.trim().is_empty() => {
                (existing_mapper_target_id.to_string(), false)
            }
            _ => (self.create_bidi_mapper_target().await?, true),
        };

        let attached = match self
            .send_command(
                "Target.attachToTarget",
                json!({
                    "targetId": mapper_target_id,
                    "flatten": true,
                }),
                None,
            )
            .await
        {
            Ok(attached) => attached,
            Err(error)
                if !created_target
                    && error.contains("Target.attachToTarget failed")
                    && error.contains("No target with given id found") =>
            {
                mapper_target_id = self.create_bidi_mapper_target().await?;
                created_target = true;
                self.send_command(
                    "Target.attachToTarget",
                    json!({
                        "targetId": mapper_target_id,
                        "flatten": true,
                    }),
                    None,
                )
                .await?
            }
            Err(error) => return Err(error),
        };
        let mapper_session_id = required_string(&attached, "/sessionId")?;

        self.send_command("Runtime.enable", json!({}), Some(&mapper_session_id))
            .await?;
        self.send_command(
            "Target.exposeDevToolsProtocol",
            json!({
                "bindingName": "cdp",
                "targetId": mapper_target_id,
                "inheritPermissions": true,
            }),
            None,
        )
        .await?;
        self.ensure_runtime_binding(&mapper_session_id, "sendBidiResponse")
            .await?;

        if created_target
            || !self
                .mapper_runtime_is_initialized(&mapper_session_id)
                .await?
        {
            self.evaluate_expression(&mapper_session_id, CHROMIUM_BIDI_MAPPER_BUNDLE, false)
                .await?;
            self.evaluate_expression(
                &mapper_session_id,
                &format!(
                    "window.runMapperInstance('{}')",
                    js_single_quote(&mapper_target_id)
                ),
                true,
            )
            .await?;
            self.send_bidi_command(
                &mapper_session_id,
                &json!({
                    "id": 1,
                    "method": "session.new",
                    "params": {
                        "capabilities": {}
                    }
                }),
            )
            .await?;
        }

        Ok(MapperConnectionInfo {
            package_version: CHROMIUM_BIDI_NPM_VERSION.to_string(),
            mapper_target_id,
            mapper_session_id,
        })
    }

    async fn create_bidi_mapper_target(&mut self) -> Result<String, String> {
        let created = self
            .send_command(
                "Target.createTarget",
                json!({
                    "url": "about:blank#MAPPER_TARGET",
                    "hidden": true,
                    "background": true,
                }),
                None,
            )
            .await?;
        required_string(&created, "/targetId")
    }

    async fn ensure_runtime_binding(&mut self, session_id: &str, name: &str) -> Result<(), String> {
        match self
            .send_command(
                "Runtime.addBinding",
                json!({
                    "name": name,
                }),
                Some(session_id),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(error)
                if error.contains("already exists") || error.contains("Cannot add binding") =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn mapper_runtime_is_initialized(&mut self, session_id: &str) -> Result<bool, String> {
        let result = self
            .evaluate_expression(
                session_id,
                "typeof window.runMapperInstance === 'function' && typeof window.onBidiMessage === 'function'",
                true,
            )
            .await?;
        Ok(result
            .pointer("/result/value")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    async fn send_bidi_command(
        &mut self,
        mapper_session_id: &str,
        command: &Value,
    ) -> Result<Value, String> {
        let command_id = command
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "BiDi command is missing numeric id".to_string())?;
        let eval_id = self.next_id;
        self.next_id += 1;
        let command_json = command.to_string();
        let expression = format!(
            "onBidiMessage({})",
            serde_json::to_string(&command_json)
                .map_err(|error| format!("failed to encode BiDi command payload: {error}"))?
        );

        let payload = json!({
            "id": eval_id,
            "method": "Runtime.evaluate",
            "params": {
                "expression": expression,
                "awaitPromise": true,
            },
            "sessionId": mapper_session_id,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send mapper Runtime.evaluate call: {error}"))?;

        loop {
            let message = self.next_json_message().await?;

            if message.get("id").and_then(Value::as_u64) == Some(eval_id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("mapper Runtime.evaluate failed: {error}"));
                }
                if let Some(exception_details) = message.pointer("/result/exceptionDetails") {
                    return Err(format!(
                        "mapper Runtime.evaluate raised exception details: {exception_details}"
                    ));
                }
                continue;
            }

            if message.get("sessionId").and_then(Value::as_str) != Some(mapper_session_id) {
                continue;
            }

            if message.get("method").and_then(Value::as_str) != Some("Runtime.bindingCalled") {
                continue;
            }

            let params = message
                .get("params")
                .ok_or_else(|| "Runtime.bindingCalled message is missing params".to_string())?;
            if params.get("name").and_then(Value::as_str) != Some("sendBidiResponse") {
                continue;
            }

            let payload = params
                .get("payload")
                .and_then(Value::as_str)
                .ok_or_else(|| "Runtime.bindingCalled payload is missing".to_string())?;
            let response = serde_json::from_str::<Value>(payload)
                .map_err(|error| format!("failed to parse BiDi mapper response JSON: {error}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(command_id) {
                continue;
            }

            match response.get("type").and_then(Value::as_str) {
                Some("success") => return Ok(response),
                Some("error") => {
                    let message = response
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown BiDi mapper error");
                    let error = response
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    return Err(format!("BiDi mapper returned {error}: {message}"));
                }
                Some(other) => {
                    return Err(format!("unexpected BiDi response type {other}: {response}"));
                }
                None => {
                    return Err(format!("BiDi mapper response is missing type: {response}"));
                }
            }
        }
    }

    async fn resolve_bidi_context_id(
        &mut self,
        mapper_session_id: &str,
        existing_context_id: Option<&str>,
        current_url: Option<&str>,
    ) -> Result<String, String> {
        if let Some(existing_context_id) = existing_context_id {
            if !existing_context_id.trim().is_empty() {
                let probe = json!({
                    "id": 1,
                    "method": "browsingContext.getTree",
                    "params": {
                        "root": existing_context_id,
                        "maxDepth": 0,
                    }
                });
                match self.send_bidi_command(mapper_session_id, &probe).await {
                    Ok(_) => return Ok(existing_context_id.to_string()),
                    Err(error)
                        if error.contains("no such frame")
                            || error.contains("Context ")
                            || error.contains("invalid argument") => {}
                    Err(error) => return Err(error),
                }
            }
        }

        let target_url = current_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        for attempt in 0..20 {
            let tree = self
                .send_bidi_command(
                    mapper_session_id,
                    &json!({
                        "id": 2,
                        "method": "browsingContext.getTree",
                        "params": {
                            "maxDepth": 0,
                        }
                    }),
                )
                .await?;
            let contexts = tree
                .pointer("/result/contexts")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    format!("BiDi browsingContext.getTree returned no contexts: {tree}")
                })?;

            if let Some(target_url) = target_url.as_deref() {
                if let Some(context_id) = contexts.iter().find_map(|context| {
                    let url = context.get("url").and_then(Value::as_str)?;
                    if url == target_url {
                        context
                            .get("context")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    } else {
                        None
                    }
                }) {
                    return Ok(context_id);
                }
            }

            if let Some(context_id) = contexts
                .first()
                .and_then(|context| context.get("context"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
            {
                return Ok(context_id);
            }

            if attempt < 19 {
                sleep(Duration::from_millis(150)).await;
            } else {
                return Err(format!(
                    "BiDi browsingContext.getTree returned no usable context ids after retries: {tree}"
                ));
            }
        }

        Err("BiDi browsing context resolution exhausted retries unexpectedly".to_string())
    }

    async fn navigate_and_wait_for_load(
        &mut self,
        session_id: &str,
        url: &str,
    ) -> Result<(), String> {
        let id = self.next_id;
        self.next_id += 1;

        let payload = json!({
            "id": id,
            "method": "Page.navigate",
            "params": {
                "url": url,
            },
            "sessionId": session_id,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send CDP command Page.navigate: {error}"))?;

        let mut navigate_confirmed = false;
        let mut load_seen = false;

        loop {
            let message = self.next_json_message().await?;

            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("CDP command Page.navigate failed: {error}"));
                }
                navigate_confirmed = true;
            } else if message.get("sessionId").and_then(Value::as_str) == Some(session_id)
                && message.get("method").and_then(Value::as_str) == Some("Page.loadEventFired")
            {
                load_seen = true;
            }

            if navigate_confirmed && load_seen {
                return Ok(());
            }
        }
    }

    async fn next_json_message(&mut self) -> Result<Value, String> {
        loop {
            let message = self
                .socket
                .next()
                .await
                .ok_or_else(|| "CDP websocket closed unexpectedly".to_string())?
                .map_err(|error| format!("failed to read CDP websocket message: {error}"))?;

            match message {
                Message::Text(text) => {
                    let value = serde_json::from_str::<Value>(&text)
                        .map_err(|error| format!("failed to parse CDP JSON message: {error}"))?;
                    return Ok(value);
                }
                Message::Binary(bytes) => {
                    let value = serde_json::from_slice::<Value>(&bytes)
                        .map_err(|error| format!("failed to parse CDP JSON message: {error}"))?;
                    return Ok(value);
                }
                Message::Ping(payload) => {
                    self.socket
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|error| format!("failed to reply to CDP ping: {error}"))?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Close(frame) => {
                    return Err(format!("CDP websocket closed: {frame:?}"));
                }
            }
        }
    }
}

fn required_string(value: &Value, pointer: &str) -> Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing string field at JSON pointer {pointer}"))
}

fn js_single_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn json_u32(value: &Value, pointer: &str) -> Result<u32, String> {
    let raw = value
        .pointer(pointer)
        .ok_or_else(|| format!("missing JSON value at {pointer}: {value}"))?;

    if let Some(number) = raw.as_u64() {
        return u32::try_from(number)
            .map_err(|_| format!("numeric JSON value at {pointer} exceeds u32: {number}"));
    }

    if let Some(number) = raw.as_i64() {
        return u32::try_from(number).map_err(|_| {
            format!("numeric JSON value at {pointer} is out of range for u32: {number}")
        });
    }

    Err(format!(
        "expected numeric JSON value at {pointer}, found {raw}"
    ))
}

fn json_f64(value: &Value, pointer: &str) -> Result<f64, String> {
    let raw = value
        .pointer(pointer)
        .ok_or_else(|| format!("missing JSON value at {pointer}: {value}"))?;

    raw.as_f64()
        .ok_or_else(|| format!("expected numeric JSON value at {pointer}, found {raw}"))
}

fn json_string(value: &Value, pointer: &str) -> Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("expected string JSON value at {pointer}"))
}

fn json_string_literal(value: &str, context: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("failed to serialize {context}: {error}"))
}

fn require_non_empty_selector(command_name: &str, css_selector: &str) -> Result<(), String> {
    if css_selector.trim().is_empty() {
        Err(format!(
            "{command_name} command requires a non-empty css_selector"
        ))
    } else {
        Ok(())
    }
}

fn block_on_plugin_future<T, F>(future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to create plugin runtime: {error}"))?;
    runtime.block_on(future)
}

fn plugin_response(result: Result<PluginResult, String>) -> *mut c_char {
    let envelope = match result {
        Ok(result) => PluginEnvelope {
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => PluginEnvelope {
            ok: false,
            result: None,
            error: Some(error),
        },
    };

    let json = match serde_json::to_string(&envelope) {
        Ok(json) => json,
        Err(error) => {
            let fallback = PluginEnvelope {
                ok: false,
                result: None,
                error: Some(format!("failed to serialize plugin response: {error}")),
            };
            serde_json::to_string(&fallback).unwrap_or_else(|_| {
                "{\"ok\":false,\"result\":null,\"error\":\"failed to serialize plugin response\"}"
                    .to_string()
            })
        }
    };

    CString::new(json).unwrap().into_raw()
}

fn handle_plugin_command(command: PluginCommand) -> Result<PluginResult, String> {
    match command {
        PluginCommand::LaunchBrowser {
            browser_kind,
            browser_binary,
        } => match browser_kind {
            BrowserKind::Chromium => {
                let launch = open_chrome_window(browser_binary.as_deref())?;
                Ok(PluginResult::LaunchBrowser(BrowserLaunchInfo {
                    browser_kind,
                    browser: launch.browser,
                    note: launch.note,
                    user_data_dir: launch.user_data_dir,
                    process_id: launch.process_id,
                }))
            }
            BrowserKind::Firefox => {
                Err("firefox backend is not implemented yet for the web plugin".to_string())
            }
        },
        PluginCommand::OpenChromeWindow { chrome_binary } => {
            open_chrome_window(chrome_binary.as_deref()).map(PluginResult::OpenChromeWindow)
        }
        PluginCommand::DiscoverInitialTab { cdp_websocket_url } => {
            block_on_plugin_future(async move {
                let result = discover_initial_tab(&cdp_websocket_url).await?;
                Ok(PluginResult::DiscoverInitialTab(result))
            })
        }
        PluginCommand::OpenChromeTab { cdp_websocket_url } => {
            block_on_plugin_future(async move {
                let result = open_chrome_tab(&cdp_websocket_url).await?;
                Ok(PluginResult::OpenChromeTab(result))
            })
        }
        PluginCommand::CloseBrowserProcess { process_id } => {
            close_browser_process(process_id)?;
            Ok(PluginResult::CloseBrowserProcess)
        }
        PluginCommand::CloseChromeTab {
            cdp_websocket_url,
            target_id,
        } => block_on_plugin_future(async move {
            close_chrome_tab(&cdp_websocket_url, &target_id).await?;
            Ok(PluginResult::CloseChromeTab)
        }),
        PluginCommand::NavigateChromeTab {
            cdp_websocket_url,
            target_id,
            url,
        } => block_on_plugin_future(async move {
            let result = navigate_chrome_tab(&cdp_websocket_url, &target_id, &url).await?;
            Ok(PluginResult::NavigateChromeTab(result))
        }),
        PluginCommand::InjectChromiumBidiMapper { cdp_websocket_url } => {
            block_on_plugin_future(async move {
                let result = inject_chromium_bidi_mapper(&cdp_websocket_url).await?;
                Ok(PluginResult::InjectChromiumBidiMapper(result))
            })
        }
        PluginCommand::ResolveBidiContextForTab {
            cdp_websocket_url,
            mapper_target_id,
            browsing_context_id,
            url,
        } => block_on_plugin_future(async move {
            let (resolved_browsing_context_id, mapper) = resolve_bidi_context_for_tab(
                &cdp_websocket_url,
                mapper_target_id.as_deref(),
                browsing_context_id.as_deref(),
                url.as_deref(),
            )
            .await?;
            Ok(PluginResult::ResolveBidiContextForTab {
                browsing_context_id: resolved_browsing_context_id,
                mapper,
            })
        }),
        PluginCommand::ClickElementViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result = click_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::ClickElementViaCdp(result))
        }),
        PluginCommand::CountElementsViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result = count_elements_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::CountElementsViaCdp(result))
        }),
        PluginCommand::HighlightElementsViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
            duration_ms,
        } => block_on_plugin_future(async move {
            let duration_ms = u32::try_from(duration_ms)
                .map_err(|_| format!("highlight duration {duration_ms} exceeds u32"))?;
            let result =
                highlight_elements_via_cdp(&cdp_websocket_url, &target_id, &css_selector, duration_ms)
                    .await?;
            Ok(PluginResult::HighlightElementsViaCdp(result))
        }),
        PluginCommand::FocusElementViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result = focus_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::FocusElementViaCdp(result))
        }),
        PluginCommand::FillElementViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
            value,
        } => block_on_plugin_future(async move {
            let result =
                fill_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector, &value).await?;
            Ok(PluginResult::FillElementViaCdp(result))
        }),
        PluginCommand::HoverElementViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result = hover_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::HoverElementViaCdp(result))
        }),
        PluginCommand::PressKeyViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
            key,
            text,
        } => block_on_plugin_future(async move {
            let result = press_key_via_cdp(
                &cdp_websocket_url,
                &target_id,
                &css_selector,
                &key,
                text.as_deref(),
            )
            .await?;
            Ok(PluginResult::PressKeyViaCdp(result))
        }),
        PluginCommand::GetTextContentViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result =
                get_text_content_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::GetTextContentViaCdp(result))
        }),
        PluginCommand::GetInnerTextViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result = get_inner_text_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::GetInnerTextViaCdp(result))
        }),
        PluginCommand::WaitForSelectorViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
            visible,
        } => block_on_plugin_future(async move {
            let result =
                wait_for_selector_via_cdp(&cdp_websocket_url, &target_id, &css_selector, visible)
                    .await?;
            Ok(PluginResult::WaitForSelectorViaCdp(result))
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_api_version() -> u32 {
    ALLWRIGHT_PLUGIN_API_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn allwright_plugin_id() -> *const c_char {
    c"web".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allwright_plugin_invoke(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return plugin_response(Err("plugin request pointer is null".to_string()));
    }

    let request = match unsafe { CStr::from_ptr(request_json) }.to_str() {
        Ok(request) => request,
        Err(error) => {
            return plugin_response(Err(format!(
                "plugin request is not valid UTF-8: {error}"
            )));
        }
    };

    let command: PluginCommand = match serde_json::from_str(request) {
        Ok(command) => command,
        Err(error) => {
            return plugin_response(Err(format!(
                "failed to parse plugin request JSON: {error}"
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
