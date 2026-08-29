use allwright_plugin_sdk::{
    ALLWRIGHT_PLUGIN_API_VERSION, AutomationSessionInfo, BrowserKind, BrowserLaunchInfo,
    BrowserSessionHandle, ChromeLaunchInfo, ChromeTabInfo, ChromiumBidiMapperInfo, ClickInfo,
    ElementCountInfo, FillInfo, FocusInfo, HighlightElementsInfo, HoverInfo, PageInfo,
    PageSessionHandle, PluginCommand, PluginEnvelope, PluginResult, PressKeyInfo, SurfaceFamily,
    SurfacePlugin, SurfacePluginDescriptor, TabNavigationInfo, TextInfo, WaitForSelectorInfo,
};
use std::borrow::Cow;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::future::Future;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, MutexGuard};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorKind {
    Css,
    XPath,
}

#[derive(Debug, Clone, Copy, Default)]
struct DiscoveryRequirements {
    require_match: bool,
    require_visible: bool,
    require_pointer_interactable: bool,
    require_editable: bool,
    focus_first_match: bool,
    require_focus: bool,
}

#[derive(Debug, Clone)]
struct ParsedSelector<'a> {
    raw: &'a str,
    value: Cow<'a, str>,
    kind: SelectorKind,
}

#[derive(Debug, Clone, Copy)]
struct ExplicitSelectorPrefix {
    kind: SelectorKind,
    prefix_len: usize,
}

const CHROMIUM_BIDI_NPM_VERSION: &str = "17.0.2";
const CHROMIUM_BIDI_MAPPER_BUNDLE: &str =
    include_str!("../third_party/chromium-bidi/17.0.2/mapperTab.js");

struct FirefoxSessionState {
    process_id: u32,
    connection: BidiConnection,
}

static FIREFOX_SESSION_REGISTRY: OnceLock<Mutex<HashMap<String, FirefoxSessionState>>> =
    OnceLock::new();
static PLUGIN_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

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
    block_on_plugin_future(async {
        let registry = firefox_session_registry();
        let mut sessions = registry.lock().await;
        sessions.retain(|_, state| state.process_id != process_id);
        Ok::<(), String>(())
    })?;
    close_browser_process_for_platform(process_id)
}

pub fn launch_browser(
    browser_kind: BrowserKind,
    browser_binary: Option<&str>,
) -> Result<BrowserLaunchInfo, String> {
    match browser_kind {
        BrowserKind::Chromium => {
            let launch = open_chrome_window(browser_binary)?;
            let initial_page = block_on_plugin_future(async {
                discover_initial_tab(&launch.cdp_websocket_url).await
            })?;
            Ok(BrowserLaunchInfo {
                browser_kind,
                browser: launch.browser,
                note: launch.note,
                user_data_dir: launch.user_data_dir,
                process_id: launch.process_id,
                browser_session: BrowserSessionHandle::Chromium {
                    cdp_websocket_url: launch.cdp_websocket_url,
                },
                initial_page: PageInfo {
                    note: initial_page.note,
                    page_session: PageSessionHandle::Chromium {
                        target_id: initial_page.target_id,
                        browsing_context_id: None,
                    },
                },
            })
        }
        BrowserKind::Firefox => launch_firefox_with_bidi(browser_binary),
    }
}

pub async fn open_page(browser_session: &BrowserSessionHandle) -> Result<PageInfo, String> {
    match browser_session {
        BrowserSessionHandle::Chromium { cdp_websocket_url } => {
            let tab = open_chrome_tab(cdp_websocket_url).await?;
            Ok(PageInfo {
                note: tab.note,
                page_session: PageSessionHandle::Chromium {
                    target_id: tab.target_id,
                    browsing_context_id: None,
                },
            })
        }
        BrowserSessionHandle::Firefox { connection_id, .. } => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let created = bidi
                .send_command(
                    "browsingContext.create",
                    json!({
                        "type": "tab",
                        "background": false,
                    }),
                )
                .await?;
            Ok(PageInfo {
                note: "opened Firefox tab via native WebDriver BiDi".to_string(),
                page_session: PageSessionHandle::Firefox {
                    browsing_context_id: required_string(&created, "/result/context")?,
                },
            })
        }
    }
}

pub async fn close_page(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
) -> Result<(), String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => close_chrome_tab(cdp_websocket_url, target_id).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            bidi.send_command(
                "browsingContext.close",
                json!({
                    "context": browsing_context_id,
                }),
            )
            .await?;
            Ok(())
        }
        _ => Err("browser/page backend mismatch while closing page".to_string()),
    }
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
        page_session: PageSessionHandle::Chromium {
            target_id: target_id.to_string(),
            browsing_context_id: Some(browsing_context_id),
        },
        automation: AutomationSessionInfo {
            bidi_session_id: String::new(),
            note: String::new(),
            mapper_target_id: None,
            mapper_session_id: None,
            package_version: None,
        },
    })
}

pub async fn navigate_page(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium {
                target_id,
                browsing_context_id,
            },
        ) => {
            let navigation = navigate_chrome_tab(cdp_websocket_url, target_id, url).await?;
            let (resolved_browsing_context_id, mapper) = resolve_bidi_context_for_tab(
                cdp_websocket_url,
                None,
                browsing_context_id.as_deref(),
                Some(&navigation.url),
            )
            .await?;
            Ok(TabNavigationInfo {
                url: navigation.url,
                note: navigation.note,
                page_session: PageSessionHandle::Chromium {
                    target_id: target_id.to_string(),
                    browsing_context_id: Some(resolved_browsing_context_id),
                },
                automation: AutomationSessionInfo {
                    bidi_session_id: format!("chromium-bidi:{target_id}"),
                    note: format!(
                        "chromium-bidi mapper ready from pinned chromium-bidi@{} after navigation",
                        mapper.package_version
                    ),
                    mapper_target_id: Some(mapper.mapper_target_id),
                    mapper_session_id: Some(mapper.mapper_session_id),
                    package_version: Some(mapper.package_version),
                },
            })
        }
        (
            BrowserSessionHandle::Firefox {
                connection_id,
                bidi_session_id,
            },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let result = bidi
                .send_command(
                    "browsingContext.navigate",
                    json!({
                        "context": browsing_context_id,
                        "url": url,
                        "wait": "complete",
                    }),
                )
                .await?;
            Ok(TabNavigationInfo {
                url: json_string(&result, "/result/url")?,
                note: "navigated Firefox tab via native WebDriver BiDi".to_string(),
                page_session: page_session.clone(),
                automation: AutomationSessionInfo {
                    bidi_session_id: bidi_session_id.clone(),
                    note: "connected to native Firefox WebDriver BiDi session".to_string(),
                    mapper_target_id: None,
                    mapper_session_id: None,
                    package_version: None,
                },
            })
        }
        _ => Err("browser/page backend mismatch while navigating".to_string()),
    }
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
    let parsed = parse_selector("click_element", css_selector)?;

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered = discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            require_pointer_interactable: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    let center = discovered.first_center.ok_or_else(|| {
        format!(
            "element discovery did not return a clickable center for {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        )
    })?;
    cdp.dispatch_mouse_click(&session_id, center.x, center.y)
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(ClickInfo {
        css_selector: css_selector.to_string(),
        note: format!(
            "clicked element via CDP mouse events using {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
        bidi_session_id: String::new(),
    })
}

pub async fn count_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    let parsed = parse_selector("count_elements", css_selector)?;

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered = discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements::default(),
    )
    .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(ElementCountInfo {
        css_selector: css_selector.to_string(),
        count: discovered.count,
        note: format!(
            "counted {} element(s) matching {} {}",
            discovered.count,
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
}

pub async fn highlight_elements_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    duration_ms: u32,
) -> Result<HighlightElementsInfo, String> {
    let parsed = parse_selector("highlight_elements", css_selector)?;

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered = discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements::default(),
    )
    .await?;

    let selector_literal = serde_json::to_string(&parsed.value)
        .map_err(|error| format!("failed to serialize selector for highlight: {error}"))?;
    let query_all_js = selector_query_all_js(parsed.kind);
    let duration_ms = duration_ms.max(1);
    cdp.evaluate_expression(
        &session_id,
        &format!(
            "(() => {{
                const selector = {selector_literal};
                const durationMs = {duration_ms};
                const elements = {query_all_js};
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
            "highlighted {} element(s) matching {} {} for {duration_ms}ms",
            discovered.count,
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
}

pub async fn focus_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    let parsed = parse_selector("focus_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            focus_first_match: true,
            require_focus: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(FocusInfo {
        css_selector: css_selector.to_string(),
        note: format!(
            "focused element matching {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
}

pub async fn fill_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    let parsed = parse_selector("fill_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            require_editable: true,
            focus_first_match: true,
            require_focus: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    let selector_literal = json_string_literal(&parsed.value, "fill selector")?;
    let value_literal = json_string_literal(value, "fill value")?;
    cdp.evaluate_expression(
        &session_id,
        &dom_set_value_and_dispatch_events_js(parsed.kind, &selector_literal, &value_literal),
        true,
    )
    .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(FillInfo {
        css_selector: css_selector.to_string(),
        value: value.to_string(),
        note: format!(
            "filled element matching {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
}

pub async fn hover_element_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    let parsed = parse_selector("hover_element", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered = discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            require_pointer_interactable: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    let center = discovered.first_center.ok_or_else(|| {
        format!(
            "element discovery did not return a hover center for {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        )
    })?;
    cdp.dispatch_mouse_move(&session_id, center.x, center.y)
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(HoverInfo {
        css_selector: css_selector.to_string(),
        note: format!(
            "hovered element matching {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
}

pub async fn press_key_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    let parsed = parse_selector("press_key", css_selector)?;
    if key.trim().is_empty() {
        return Err("press_key command requires a non-empty key".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            focus_first_match: true,
            require_focus: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    cdp.dispatch_key_press(&session_id, key, text).await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(PressKeyInfo {
        css_selector: css_selector.to_string(),
        key: key.to_string(),
        note: format!(
            "pressed key {key} on element matching {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
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
            let parsed = parse_selector("get_text_content", css_selector)
                .expect("selector was already validated for textContent resolution");
            info.note = format!(
                "resolved textContent for {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            );
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
            let parsed = parse_selector("get_inner_text", css_selector)
                .expect("selector was already validated for innerText resolution");
            info.note = format!(
                "resolved innerText for {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            );
            info
        })
}

pub async fn wait_for_selector_via_cdp(
    cdp_websocket_url: &str,
    target_id: &str,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    let parsed = parse_selector("wait_for_selector", css_selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    let discovered = discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        css_selector,
        DiscoveryRequirements {
            require_visible: visible,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    let success = if visible {
        discovered.first_center.is_some()
    } else {
        discovered.count > 0
    };
    cdp.detach_from_target(&session_id).await?;

    if !success {
        return Err(if visible {
            format!(
                "no visible element matches {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            )
        } else {
            format!(
                "no element matches {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            )
        });
    }

    Ok(WaitForSelectorInfo {
        css_selector: css_selector.to_string(),
        visible,
        note: if visible {
            format!(
                "visible element matched {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            )
        } else {
            format!(
                "element matched {} {}",
                selector_kind_label(parsed.kind),
                parsed.value
            )
        },
    })
}

pub async fn click_element(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ClickInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => click_element_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox {
                connection_id,
                bidi_session_id,
            },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("click_element", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_first_js = selector_query_first_js(parsed.kind);
            firefox_evaluate_void(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const requirements = {};
                        {}
                        const element = {query_first_js};
                        const prepared = allwrightPrepareElement(element, selector, '{}', requirements);
                        prepared.element.click();
                    }})()",
                    json_string_literal(&parsed.value, "click selector")?,
                    discovery_requirements_literal(DiscoveryRequirements {
                        require_match: true,
                        require_visible: true,
                        require_pointer_interactable: true,
                        ..DiscoveryRequirements::default()
                    }),
                    actionability_helpers_js(),
                    selector_kind_label(parsed.kind),
                ),
            )
            .await?;
            Ok(ClickInfo {
                css_selector: css_selector.to_string(),
                note: format!(
                    "clicked element via Firefox WebDriver BiDi using {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
                bidi_session_id: bidi_session_id.clone(),
            })
        }
        _ => Err("browser/page backend mismatch while clicking".to_string()),
    }
}

pub async fn count_elements(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<ElementCountInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => count_elements_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("count_elements", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_all_js = selector_query_all_js(parsed.kind);
            let count = firefox_evaluate_u32(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        return {query_all_js}.length;
                    }})()",
                    json_string_literal(&parsed.value, "count selector")?,
                ),
            )
            .await?;
            Ok(ElementCountInfo {
                css_selector: css_selector.to_string(),
                count,
                note: format!(
                    "counted {count} element(s) matching {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while counting elements".to_string()),
    }
}

pub async fn highlight_elements(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    duration_ms: u64,
) -> Result<HighlightElementsInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => {
            highlight_elements_via_cdp(
                cdp_websocket_url,
                target_id,
                css_selector,
                u32::try_from(duration_ms).unwrap_or(u32::MAX),
            )
            .await
        }
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("highlight_elements", css_selector)?;
            let duration_ms = u32::try_from(duration_ms).unwrap_or(u32::MAX).max(1);
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_all_js = selector_query_all_js(parsed.kind);
            let count = firefox_evaluate_u32(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const durationMs = {duration_ms};
                        const elements = {query_all_js};
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
                    }})()",
                    json_string_literal(&parsed.value, "highlight selector")?,
                ),
            )
            .await?;
            Ok(HighlightElementsInfo {
                css_selector: css_selector.to_string(),
                count,
                note: format!(
                    "highlighted {count} element(s) matching {} {} for {duration_ms}ms",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while highlighting elements".to_string()),
    }
}

pub async fn focus_element(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<FocusInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => focus_element_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("focus_element", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_first_js = selector_query_first_js(parsed.kind);
            firefox_evaluate_void(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const requirements = {};
                        {}
                        const element = {query_first_js};
                        allwrightPrepareElement(element, selector, '{}', requirements);
                    }})()",
                    json_string_literal(&parsed.value, "focus selector")?,
                    discovery_requirements_literal(DiscoveryRequirements {
                        require_match: true,
                        require_visible: true,
                        focus_first_match: true,
                        require_focus: true,
                        ..DiscoveryRequirements::default()
                    }),
                    actionability_helpers_js(),
                    selector_kind_label(parsed.kind),
                ),
            )
            .await?;
            Ok(FocusInfo {
                css_selector: css_selector.to_string(),
                note: format!(
                    "focused element matching {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while focusing element".to_string()),
    }
}

pub async fn fill_element(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    value: &str,
) -> Result<FillInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => fill_element_via_cdp(cdp_websocket_url, target_id, css_selector, value).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("fill_element", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            firefox_evaluate_void(
                bidi,
                browsing_context_id,
                &dom_set_value_and_dispatch_events_js(
                    parsed.kind,
                    &json_string_literal(&parsed.value, "fill selector")?,
                    &json_string_literal(value, "fill value")?,
                ),
            )
            .await?;
            Ok(FillInfo {
                css_selector: css_selector.to_string(),
                value: value.to_string(),
                note: format!(
                    "filled element matching {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while filling element".to_string()),
    }
}

pub async fn hover_element(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<HoverInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => hover_element_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("hover_element", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_first_js = selector_query_first_js(parsed.kind);
            firefox_evaluate_void(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const requirements = {};
                        {}
                        const element = {query_first_js};
                        const prepared = allwrightPrepareElement(element, selector, '{}', requirements);
                        for (const type of ['pointerover', 'mouseover', 'mouseenter', 'mousemove']) {{
                            prepared.element.dispatchEvent(new MouseEvent(type, {{ bubbles: true, cancelable: true, view: window }}));
                        }}
                    }})()",
                    json_string_literal(&parsed.value, "hover selector")?,
                    discovery_requirements_literal(DiscoveryRequirements {
                        require_match: true,
                        require_visible: true,
                        require_pointer_interactable: true,
                        ..DiscoveryRequirements::default()
                    }),
                    actionability_helpers_js(),
                    selector_kind_label(parsed.kind),
                ),
            )
            .await?;
            Ok(HoverInfo {
                css_selector: css_selector.to_string(),
                note: format!(
                    "hovered element matching {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while hovering element".to_string()),
    }
}

pub async fn press_key(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    key: &str,
    text: Option<&str>,
) -> Result<PressKeyInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => press_key_via_cdp(cdp_websocket_url, target_id, css_selector, key, text).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("press_key", css_selector)?;
            if key.trim().is_empty() {
                return Err("press_key command requires a non-empty key".to_string());
            }
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_first_js = selector_query_first_js(parsed.kind);
            firefox_evaluate_void(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const key = {};
                        const text = {};
                        const requirements = {};
                        {}
                        const element = {query_first_js};
                        const prepared = allwrightPrepareElement(element, selector, '{}', requirements);
                        prepared.element.dispatchEvent(new KeyboardEvent('keydown', {{ key, bubbles: true }}));
                        if (text && 'value' in prepared.element) {{
                            prepared.element.value += text;
                            prepared.element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                            prepared.element.dispatchEvent(new Event('change', {{ bubbles: true }}));
                        }}
                        prepared.element.dispatchEvent(new KeyboardEvent('keyup', {{ key, bubbles: true }}));
                    }})()",
                    json_string_literal(&parsed.value, "press selector")?,
                    json_string_literal(key, "press key")?,
                    json_string_literal(text.unwrap_or(""), "press text")?,
                    discovery_requirements_literal(DiscoveryRequirements {
                        require_match: true,
                        require_visible: true,
                        focus_first_match: true,
                        require_focus: true,
                        ..DiscoveryRequirements::default()
                    }),
                    actionability_helpers_js(),
                    selector_kind_label(parsed.kind),
                ),
            )
            .await?;
            Ok(PressKeyInfo {
                css_selector: css_selector.to_string(),
                key: key.to_string(),
                note: format!(
                    "pressed key {key} on element matching {} {}",
                    selector_kind_label(parsed.kind),
                    parsed.value
                ),
            })
        }
        _ => Err("browser/page backend mismatch while pressing a key".to_string()),
    }
}

pub async fn get_text_content(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => get_text_content_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            firefox_get_text(
                connection_id,
                browsing_context_id,
                css_selector,
                "textContent",
            )
            .await
        }
        _ => Err("browser/page backend mismatch while resolving textContent".to_string()),
    }
}

pub async fn get_inner_text(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
) -> Result<TextInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => get_inner_text_via_cdp(cdp_websocket_url, target_id, css_selector).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            firefox_get_text(
                connection_id,
                browsing_context_id,
                css_selector,
                "innerText",
            )
            .await
        }
        _ => Err("browser/page backend mismatch while resolving innerText".to_string()),
    }
}

pub async fn wait_for_selector(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    css_selector: &str,
    visible: bool,
) -> Result<WaitForSelectorInfo, String> {
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium { target_id, .. },
        ) => wait_for_selector_via_cdp(cdp_websocket_url, target_id, css_selector, visible).await,
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let parsed = parse_selector("wait_for_selector", css_selector)?;
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("Firefox session guard validated connection id")
                .connection;
            let query_first_js = selector_query_first_js(parsed.kind);
            let success = firefox_evaluate_bool(
                bidi,
                browsing_context_id,
                &format!(
                    "(() => {{
                        const selector = {};
                        const requirements = {};
                        {}
                        const element = {query_first_js};
                        if (!element) {{
                            return false;
                        }}
                        if (!{}) {{
                            return true;
                        }}
                        try {{
                            return !!allwrightPrepareElement(element, selector, '{}', requirements);
                        }} catch (_error) {{
                            return false;
                        }}
                    }})()",
                    json_string_literal(&parsed.value, "wait selector")?,
                    discovery_requirements_literal(DiscoveryRequirements {
                        require_visible: visible,
                        ..DiscoveryRequirements::default()
                    }),
                    actionability_helpers_js(),
                    if visible { "true" } else { "false" },
                    selector_kind_label(parsed.kind),
                ),
            )
            .await?;
            if !success {
                return Err(if visible {
                    format!(
                        "no visible element matches {} {}",
                        selector_kind_label(parsed.kind),
                        parsed.value
                    )
                } else {
                    format!(
                        "no element matches {} {}",
                        selector_kind_label(parsed.kind),
                        parsed.value
                    )
                });
            }
            Ok(WaitForSelectorInfo {
                css_selector: css_selector.to_string(),
                visible,
                note: if visible {
                    format!(
                        "visible element matched {} {}",
                        selector_kind_label(parsed.kind),
                        parsed.value
                    )
                } else {
                    format!(
                        "element matched {} {}",
                        selector_kind_label(parsed.kind),
                        parsed.value
                    )
                },
            })
        }
        _ => Err("browser/page backend mismatch while waiting for selector".to_string()),
    }
}

async fn discover_elements_via_cdp(
    cdp: &mut CdpConnection,
    session_id: &str,
    selector: &str,
    requirements: DiscoveryRequirements,
) -> Result<DiscoveredElements, String> {
    let selector_literal = json_string_literal(selector, "selector for discovery")?;
    let selector_segments_literal = selector_segments_literal("discover_elements", selector)?;
    let selector_kind_label = selector_chain_kind_label("discover_elements", selector)?;
    let query_all_js = selector_chain_query_all_js();
    let requirements_literal = discovery_requirements_literal(requirements);
    let expression = format!(
        "(() => {{
            const selector = {selector_literal};
            const selectorSegments = {selector_segments_literal};
            const selectorChainQueryAll = () => {query_all_js};
            const elements = {query_all_js};
            const first = elements[0] ?? null;
            const requirements = {requirements_literal};
            {actionability_helpers_js}
            if (requirements.requireMatch && !first) {{
                throw new Error(`No element matches {selector_kind_label}: ${{selector}}`);
            }}
            const prepared = first
                ? allwrightPrepareElement(first, selector, '{selector_kind_label}', requirements)
                : null;
            return {{
                count: elements.length,
                first: prepared?.center ?? null
            }};
        }})()",
        actionability_helpers_js = actionability_helpers_js(),
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
    selector: &str,
    property: &str,
) -> Result<TextInfo, String> {
    let parsed = parse_selector("get_text", selector)?;
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let session_id = cdp.prepare_page_target_session(target_id).await?;
    discover_elements_via_cdp(
        &mut cdp,
        &session_id,
        selector,
        DiscoveryRequirements {
            require_match: true,
            ..DiscoveryRequirements::default()
        },
    )
    .await?;
    let selector_literal = json_string_literal(selector, "text selector")?;
    let selector_segments_literal = selector_segments_literal("get_text", selector)?;
    let property_literal = json_string_literal(property, "text property")?;
    let selector_kind_label = selector_chain_kind_label("get_text", selector)?;
    let query_first_js = selector_chain_query_first_js();
    let result = cdp
        .evaluate_expression(
            &session_id,
            &format!(
                "(() => {{
                    const selector = {selector_literal};
                    const selectorSegments = {selector_segments_literal};
                    const selectorChainQueryAll = () => {};
                    const property = {property_literal};
                    const element = {query_first_js};
                    if (!element) {{
                        throw new Error(`No element matches {selector_kind_label}: ${{selector}}`);
                    }}
                    const value = element[property];
                    return typeof value === 'string' ? value : '';
                }})()",
                selector_chain_query_all_js(),
            ),
            true,
        )
        .await?;
    cdp.detach_from_target(&session_id).await?;

    Ok(TextInfo {
        css_selector: parsed.raw.to_string(),
        text: json_string(&result, "/result/value")?,
        note: String::new(),
    })
}

fn dom_set_value_and_dispatch_events_js(
    selector_kind: SelectorKind,
    selector_literal: &str,
    value_literal: &str,
) -> String {
    let query_first_js = selector_query_first_js(selector_kind);
    let selector_kind_label = selector_kind_label(selector_kind);
    format!(
        "(() => {{
            const selector = {selector_literal};
            const nextValue = {value_literal};
            const requirements = {requirements_literal};
            {actionability_helpers_js}
            const element = {query_first_js};
            const prepared = allwrightPrepareElement(
                element,
                selector,
                '{selector_kind_label}',
                requirements,
            );
            const editableElement = prepared.element;

            if (editableElement instanceof HTMLInputElement || editableElement instanceof HTMLTextAreaElement) {{
                const prototype = editableElement instanceof HTMLInputElement
                    ? HTMLInputElement.prototype
                    : HTMLTextAreaElement.prototype;
                const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');
                const setter = descriptor?.set;
                if (!setter) {{
                    throw new Error(`Element value setter is unavailable for {selector_kind_label}: ${{selector}}`);
                }}
                setter.call(editableElement, nextValue);
            }} else if (editableElement.isContentEditable) {{
                editableElement.textContent = nextValue;
            }} else {{
                throw new Error(`Element matched by {selector_kind_label} does not support fill: ${{selector}}`);
            }}

            const inputEvent = typeof InputEvent === 'function'
                ? new InputEvent('input', {{
                    bubbles: true,
                    composed: true,
                    data: nextValue,
                    inputType: 'insertText'
                }})
                : new Event('input', {{ bubbles: true, composed: true }});
            editableElement.dispatchEvent(inputEvent);
            editableElement.dispatchEvent(new Event('change', {{ bubbles: true, composed: true }}));
            return true;
        }})()",
        requirements_literal = discovery_requirements_literal(DiscoveryRequirements {
            require_match: true,
            require_visible: true,
            require_editable: true,
            focus_first_match: true,
            require_focus: true,
            ..DiscoveryRequirements::default()
        }),
        actionability_helpers_js = actionability_helpers_js(),
    )
}

fn discovery_requirements_literal(requirements: DiscoveryRequirements) -> String {
    format!(
        "{{ requireMatch: {}, requireVisible: {}, requirePointerInteractable: {}, requireEditable: {}, focusFirstMatch: {}, requireFocus: {} }}",
        requirements.require_match,
        requirements.require_visible,
        requirements.require_pointer_interactable,
        requirements.require_editable,
        requirements.focus_first_match,
        requirements.require_focus,
    )
}

fn actionability_helpers_js() -> &'static str {
    r#"
            const allwrightCanFocusElement = (element) => {
                if (!(element instanceof HTMLElement)) {
                    return false;
                }
                if (typeof element.matches === 'function' && element.matches(':disabled')) {
                    return false;
                }
                return !element.closest('[inert]');
            };
            const allwrightIsEditableElement = (element) => {
                if (!(element instanceof HTMLElement)) {
                    return false;
                }
                if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
                    return !element.disabled && !element.readOnly;
                }
                return element.isContentEditable && !element.closest('[inert]');
            };
            const allwrightIsVisibleElement = (rect, style) => {
                if (!rect || rect.width <= 0 || rect.height <= 0) {
                    return false;
                }
                if (!style) {
                    return false;
                }
                if (style.display === 'none' || style.visibility === 'hidden' || style.visibility === 'collapse') {
                    return false;
                }
                const opacity = Number.parseFloat(style.opacity ?? '1');
                if (!Number.isNaN(opacity) && opacity <= 0) {
                    return false;
                }
                return true;
            };
            const allwrightReceivesPointerEvents = (element, center, style) => {
                if (style.pointerEvents === 'none') {
                    return false;
                }
                const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
                const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
                if (center.x < 0 || center.y < 0 || center.x > viewportWidth || center.y > viewportHeight) {
                    return false;
                }
                const hit = document.elementFromPoint(center.x, center.y);
                return !!hit && (hit === element || element.contains(hit) || hit.contains(element));
            };
            const allwrightPrepareElement = (element, selector, selectorKindLabel, requirements) => {
                if (!element) {
                    throw new Error(`No element matches ${selectorKindLabel}: ${selector}`);
                }
                if (!(element instanceof Element)) {
                    throw new Error(`${selectorKindLabel} did not resolve to a DOM Element: ${selector}`);
                }
                if (!element.isConnected) {
                    throw new Error(`Element matched by ${selectorKindLabel} is detached from the live DOM: ${selector}`);
                }
                element.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
                const rect = element.getBoundingClientRect();
                const style = window.getComputedStyle(element);
                if (requirements.requireVisible && !allwrightIsVisibleElement(rect, style)) {
                    throw new Error(`Element matched by ${selectorKindLabel} is not visible yet: ${selector}`);
                }
                const center = rect.width > 0 && rect.height > 0
                    ? { x: rect.left + (rect.width / 2), y: rect.top + (rect.height / 2) }
                    : null;
                if (requirements.requirePointerInteractable) {
                    if (!center || !allwrightReceivesPointerEvents(element, center, style)) {
                        throw new Error(`Element matched by ${selectorKindLabel} is not clickable yet: ${selector}`);
                    }
                }
                if (requirements.requireEditable && !allwrightIsEditableElement(element)) {
                    throw new Error(`Element matched by ${selectorKindLabel} is not editable yet: ${selector}`);
                }
                if (requirements.focusFirstMatch || requirements.requireFocus) {
                    if (!allwrightCanFocusElement(element)) {
                        throw new Error(`Element matched by ${selectorKindLabel} is not focusable yet: ${selector}`);
                    }
                    element.focus();
                    const activeElement = document.activeElement;
                    if (requirements.requireFocus && activeElement !== element && !element.contains(activeElement)) {
                        throw new Error(`Element matched by ${selectorKindLabel} did not receive focus yet: ${selector}`);
                    }
                }
                return { element, rect, center, style };
            };
    "#
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

#[cfg(target_os = "macos")]
fn launch_firefox_with_bidi(browser_binary: Option<&str>) -> Result<BrowserLaunchInfo, String> {
    let browser_binary = browser_binary
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "/Applications/Firefox.app/Contents/MacOS/firefox".to_string());
    launch_firefox_with_remote_agent(&browser_binary)
}

#[cfg(target_os = "linux")]
fn launch_firefox_with_bidi(browser_binary: Option<&str>) -> Result<BrowserLaunchInfo, String> {
    if let Some(browser_binary) = browser_binary {
        return launch_firefox_with_remote_agent(browser_binary);
    }

    for candidate in ["firefox", "firefox-esr"] {
        match launch_firefox_with_remote_agent(candidate) {
            Ok(launch_info) => return Ok(launch_info),
            Err(error)
                if error.contains("No such file or directory")
                    || error.contains("os error 2")
                    || error.contains("not found") => {}
            Err(error) => return Err(format!("failed to launch {candidate}: {error}")),
        }
    }

    Err("could not find a Firefox browser binary".to_string())
}

#[cfg(target_os = "windows")]
fn launch_firefox_with_bidi(browser_binary: Option<&str>) -> Result<BrowserLaunchInfo, String> {
    let browser_binary = browser_binary.unwrap_or("firefox");
    launch_firefox_with_remote_agent(browser_binary)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn launch_firefox_with_bidi(browser_binary: Option<&str>) -> Result<BrowserLaunchInfo, String> {
    match browser_binary {
        Some(browser_binary) => launch_firefox_with_remote_agent(browser_binary),
        None => Err("Firefox launching is not supported on this platform yet".to_string()),
    }
}

fn launch_firefox_with_remote_agent(browser_binary: &str) -> Result<BrowserLaunchInfo, String> {
    let user_data_dir = create_firefox_user_data_dir()?;
    let user_data_dir_str = user_data_dir.to_string_lossy().to_string();
    let remote_port = choose_loopback_port()?;

    let child = Command::new(browser_binary)
        .args([
            "--new-instance",
            "--no-remote",
            "--remote-debugging-port",
            &remote_port.to_string(),
            "--profile",
            &user_data_dir_str,
            "about:blank",
        ])
        .spawn()
        .map_err(|error| format!("failed to launch Firefox binary {browser_binary}: {error}"))?;
    let process_id = child.id();

    let bootstrap_ws_url = format!("ws://127.0.0.1:{remote_port}/session");
    let (connection_id, bidi_session_id, initial_context_id) = block_on_plugin_future(async {
        let mut bidi = wait_for_bidi_endpoint(&bootstrap_ws_url).await?;
        let session = bidi
            .send_command(
                "session.new",
                json!({
                    "capabilities": {
                        "alwaysMatch": {
                            "acceptInsecureCerts": true
                        }
                    }
                }),
            )
            .await?;
        let session_id = required_string(&session, "/result/sessionId")?;
        let initial_context_id = bidi.wait_for_top_level_context().await?;
        let connection_id = format!("firefox-bidi-{process_id}-{session_id}");
        firefox_session_registry().lock().await.insert(
            connection_id.clone(),
            FirefoxSessionState {
                process_id,
                connection: bidi,
            },
        );
        Ok((connection_id, session_id, initial_context_id))
    })?;

    Ok(BrowserLaunchInfo {
        browser_kind: BrowserKind::Firefox,
        browser: browser_binary.to_string(),
        note: "launched Firefox window with native WebDriver BiDi enabled".to_string(),
        user_data_dir: user_data_dir_str,
        process_id,
        browser_session: BrowserSessionHandle::Firefox {
            connection_id,
            bidi_session_id: bidi_session_id.clone(),
        },
        initial_page: PageInfo {
            note: "tracked initial Firefox tab via native WebDriver BiDi".to_string(),
            page_session: PageSessionHandle::Firefox {
                browsing_context_id: initial_context_id,
            },
        },
    })
}

fn create_firefox_user_data_dir() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute timestamp: {error}"))?
        .as_millis();
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("allwright-firefox-{pid}-{timestamp}"));
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create Firefox profile dir {}: {error}",
            dir.display()
        )
    })?;
    Ok(dir)
}

fn choose_loopback_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to reserve a loopback port for Firefox: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("failed to read reserved loopback port: {error}"))?
        .port();
    drop(listener);
    Ok(port)
}

async fn wait_for_bidi_endpoint(ws_url: &str) -> Result<BidiConnection, String> {
    let mut last_error = None;
    for _ in 0..100 {
        match BidiConnection::connect(ws_url).await {
            Ok(connection) => return Ok(connection),
            Err(error) => {
                last_error = Some(error);
                sleep(Duration::from_millis(50)).await;
            }
        }
    }

    Err(format!(
        "timed out waiting for WebDriver BiDi endpoint {ws_url}: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn parse_selector<'a>(command_name: &str, selector: &'a str) -> Result<ParsedSelector<'a>, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{command_name} command requires a non-empty selector"
        ));
    }

    let lowercase = trimmed.to_ascii_lowercase();
    let parsed = if lowercase.starts_with("xpath=") {
        let value = parse_selector_value(&trimmed[6..])?;
        ParsedSelector {
            raw: selector,
            value,
            kind: SelectorKind::XPath,
        }
    } else if lowercase.starts_with("xpath:") {
        let value = parse_selector_value(&trimmed[6..])?;
        ParsedSelector {
            raw: selector,
            value,
            kind: SelectorKind::XPath,
        }
    } else if lowercase.starts_with("css=") {
        let value = parse_selector_value(&trimmed[4..])?;
        ParsedSelector {
            raw: selector,
            value,
            kind: SelectorKind::Css,
        }
    } else if lowercase.starts_with("css:") {
        let value = parse_selector_value(&trimmed[4..])?;
        ParsedSelector {
            raw: selector,
            value,
            kind: SelectorKind::Css,
        }
    } else if looks_like_xpath(trimmed) {
        ParsedSelector {
            raw: selector,
            value: Cow::Borrowed(trimmed),
            kind: SelectorKind::XPath,
        }
    } else {
        ParsedSelector {
            raw: selector,
            value: Cow::Borrowed(trimmed),
            kind: SelectorKind::Css,
        }
    };

    if parsed.value.is_empty() {
        return Err(format!(
            "{command_name} command requires a non-empty selector"
        ));
    }

    Ok(parsed)
}

fn parse_selector_value(raw: &str) -> Result<Cow<'_, str>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("selector value must not be empty".to_string());
    }
    if trimmed.starts_with('"') {
        let end = find_json_string_end(trimmed)
            .ok_or_else(|| "selector value contains an unterminated JSON string".to_string())?;
        if trimmed[end..].trim().is_empty() {
            let decoded = serde_json::from_str::<String>(&trimmed[..end])
                .map_err(|error| format!("failed to decode selector JSON string: {error}"))?;
            return Ok(Cow::Owned(decoded));
        }
    }
    Ok(Cow::Borrowed(trimmed))
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

fn looks_like_xpath(selector: &str) -> bool {
    selector.starts_with("//")
        || selector.starts_with(".//")
        || selector.starts_with("../")
        || selector.starts_with('/')
        || selector.starts_with('(')
}

fn parse_explicit_selector_prefix(selector: &str) -> Option<ExplicitSelectorPrefix> {
    let lower = selector.to_ascii_lowercase();
    if lower.starts_with("xpath=") || lower.starts_with("xpath:") {
        return Some(ExplicitSelectorPrefix {
            kind: SelectorKind::XPath,
            prefix_len: 6,
        });
    }
    if lower.starts_with("css=") || lower.starts_with("css:") {
        return Some(ExplicitSelectorPrefix {
            kind: SelectorKind::Css,
            prefix_len: 4,
        });
    }
    None
}

fn parse_selector_chain<'a>(
    command_name: &str,
    selector: &'a str,
) -> Result<Vec<ParsedSelector<'a>>, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{command_name} command requires a non-empty selector"
        ));
    }

    let Some(first_prefix) = parse_explicit_selector_prefix(trimmed) else {
        return Ok(vec![parse_selector(command_name, selector)?]);
    };

    let mut segments = Vec::new();
    let mut current_start = 0usize;
    let mut current_prefix = first_prefix;
    let bytes = trimmed.as_bytes();
    let mut scan = current_prefix.prefix_len;
    let mut in_json_string = false;
    let mut escaped = false;

    while scan < bytes.len() {
        let byte = bytes[scan];
        if in_json_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_json_string = false;
            }
            scan += 1;
            continue;
        }

        if byte == b'"' {
            in_json_string = true;
            scan += 1;
            continue;
        }

        if byte.is_ascii_whitespace() {
            let next = scan + 1;
            if next < bytes.len()
                && let Some(prefix) = parse_explicit_selector_prefix(&trimmed[next..])
            {
                let value = parse_selector_value(
                    &trimmed[current_start + current_prefix.prefix_len..scan],
                )?;
                if value.is_empty() {
                    return Err(format!(
                        "{command_name} command requires a non-empty selector"
                    ));
                }
                segments.push(ParsedSelector {
                    raw: selector,
                    value,
                    kind: current_prefix.kind,
                });
                current_start = next;
                current_prefix = prefix;
                scan = current_start + current_prefix.prefix_len;
                continue;
            }
        }
        scan += 1;
    }

    let value = parse_selector_value(&trimmed[current_start + current_prefix.prefix_len..])?;
    if value.is_empty() {
        return Err(format!(
            "{command_name} command requires a non-empty selector"
        ));
    }
    segments.push(ParsedSelector {
        raw: selector,
        value,
        kind: current_prefix.kind,
    });
    Ok(segments)
}

fn selector_kind_label(kind: SelectorKind) -> &'static str {
    match kind {
        SelectorKind::Css => "css selector",
        SelectorKind::XPath => "xpath selector",
    }
}

fn selector_query_all_js(kind: SelectorKind) -> &'static str {
    match kind {
        SelectorKind::Css => "Array.from(document.querySelectorAll(selector))",
        SelectorKind::XPath => {
            "(() => {
                const result = document.evaluate(
                    selector,
                    document,
                    null,
                    XPathResult.ORDERED_NODE_SNAPSHOT_TYPE,
                    null
                );
                const elements = [];
                for (let index = 0; index < result.snapshotLength; index += 1) {
                    const candidate = result.snapshotItem(index);
                    if (candidate instanceof Element) {
                        elements.push(candidate);
                    }
                }
                return elements;
            })()"
        }
    }
}

fn selector_query_first_js(kind: SelectorKind) -> &'static str {
    match kind {
        SelectorKind::Css => "document.querySelector(selector)",
        SelectorKind::XPath => {
            "(() => {
                const result = document.evaluate(
                    selector,
                    document,
                    null,
                    XPathResult.FIRST_ORDERED_NODE_TYPE,
                    null
                );
                const candidate = result.singleNodeValue;
                return candidate instanceof Element ? candidate : null;
            })()"
        }
    }
}

fn selector_segments_literal(command_name: &str, selector: &str) -> Result<String, String> {
    let segments = parse_selector_chain(command_name, selector)?;
    let serialized = segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            let value = if index > 0 && segment.kind == SelectorKind::XPath {
                if segment.value.starts_with("//") {
                    format!(".{}", segment.value)
                } else if segment.value.starts_with('/') {
                    format!(".{}", segment.value)
                } else if segment.value.starts_with('.') {
                    segment.value.to_string()
                } else {
                    format!(".//{}", segment.value)
                }
            } else {
                segment.value.to_string()
            };
            json!({
                "kind": match segment.kind {
                    SelectorKind::Css => "css",
                    SelectorKind::XPath => "xpath",
                },
                "value": value,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&serialized)
        .map_err(|error| format!("failed to serialize selector chain: {error}"))
}

fn selector_chain_kind_label(command_name: &str, selector: &str) -> Result<String, String> {
    let segments = parse_selector_chain(command_name, selector)?;
    if segments.len() == 1 {
        Ok(selector_kind_label(segments[0].kind).to_string())
    } else {
        Ok("selector chain".to_string())
    }
}

fn selector_chain_query_all_js() -> &'static str {
    "(() => {
        let roots = [document];
        for (const segment of selectorSegments) {
            const nextRoots = [];
            for (const root of roots) {
                if (segment.kind === 'css') {
                    const matches = root.querySelectorAll(segment.value);
                    for (const match of matches) {
                        if (match instanceof Element) {
                            nextRoots.push(match);
                        }
                    }
                    continue;
                }
                const result = document.evaluate(
                    segment.value,
                    root,
                    null,
                    XPathResult.ORDERED_NODE_SNAPSHOT_TYPE,
                    null
                );
                for (let index = 0; index < result.snapshotLength; index += 1) {
                    const candidate = result.snapshotItem(index);
                    if (candidate instanceof Element) {
                        nextRoots.push(candidate);
                    }
                }
            }
            roots = nextRoots;
        }
        return roots;
    })()"
}

fn selector_chain_query_first_js() -> &'static str {
    "(() => {
        const elements = selectorChainQueryAll();
        return elements[0] ?? null;
    })()"
}

fn firefox_session_registry() -> &'static Mutex<HashMap<String, FirefoxSessionState>> {
    FIREFOX_SESSION_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn firefox_session_guard(
    connection_id: &str,
) -> Result<MutexGuard<'static, HashMap<String, FirefoxSessionState>>, String> {
    let registry = firefox_session_registry();
    let sessions = registry.lock().await;
    if sessions.contains_key(connection_id) {
        Ok(sessions)
    } else {
        Err(format!(
            "Firefox automation session {connection_id} is not available"
        ))
    }
}

async fn firefox_evaluate_json(
    bidi: &mut BidiConnection,
    browsing_context_id: &str,
    expression: &str,
) -> Result<Value, String> {
    let result = bidi
        .send_command(
            "script.evaluate",
            json!({
                "expression": expression,
                "target": {
                    "context": browsing_context_id,
                },
                "awaitPromise": true,
            }),
        )
        .await?;
    let result_type = json_string(&result, "/result/type")?;
    if result_type != "success" {
        let details = result
            .pointer("/result/exceptionDetails")
            .cloned()
            .unwrap_or(Value::Null);
        return Err(format!("Firefox script.evaluate failed: {details}"));
    }
    let value = json_string(&result, "/result/result/value")?;
    serde_json::from_str::<Value>(&value)
        .map_err(|error| format!("failed to parse Firefox script result JSON: {error}"))
}

async fn firefox_evaluate_void(
    bidi: &mut BidiConnection,
    browsing_context_id: &str,
    expression: &str,
) -> Result<(), String> {
    firefox_evaluate_json(
        bidi,
        browsing_context_id,
        &format!("JSON.stringify((() => {{ {expression}; return null; }})())"),
    )
    .await
    .map(|_| ())
}

async fn firefox_evaluate_u32(
    bidi: &mut BidiConnection,
    browsing_context_id: &str,
    expression: &str,
) -> Result<u32, String> {
    let value = firefox_evaluate_json(
        bidi,
        browsing_context_id,
        &format!("JSON.stringify({expression})"),
    )
    .await?;
    if let Some(number) = value.as_u64() {
        return u32::try_from(number)
            .map_err(|_| format!("Firefox script result exceeds u32: {number}"));
    }
    if let Some(number) = value.as_i64() {
        return u32::try_from(number)
            .map_err(|_| format!("Firefox script result is out of range for u32: {number}"));
    }
    Err(format!(
        "expected numeric Firefox script result, found {value}"
    ))
}

async fn firefox_evaluate_bool(
    bidi: &mut BidiConnection,
    browsing_context_id: &str,
    expression: &str,
) -> Result<bool, String> {
    let value = firefox_evaluate_json(
        bidi,
        browsing_context_id,
        &format!("JSON.stringify({expression})"),
    )
    .await?;
    value
        .as_bool()
        .ok_or_else(|| format!("expected boolean Firefox script result, found {value}"))
}

async fn firefox_get_text(
    connection_id: &str,
    browsing_context_id: &str,
    css_selector: &str,
    property: &str,
) -> Result<TextInfo, String> {
    let parsed = parse_selector("get_text", css_selector)?;
    let mut sessions = firefox_session_guard(connection_id).await?;
    let bidi = &mut sessions
        .get_mut(connection_id)
        .expect("Firefox session guard validated connection id")
        .connection;
    let query_first_js = selector_query_first_js(parsed.kind);
    let value = firefox_evaluate_json(
        bidi,
        browsing_context_id,
        &format!(
            "JSON.stringify((() => {{
                const selector = {};
                const property = {};
                const element = {query_first_js};
                if (!element) {{
                    throw new Error(`No element matches {}: ${{selector}}`);
                }}
                const value = element[property];
                return typeof value === 'string' ? value : '';
            }})())",
            json_string_literal(&parsed.value, "text selector")?,
            json_string_literal(property, "text property")?,
            selector_kind_label(parsed.kind),
        ),
    )
    .await?;

    Ok(TextInfo {
        css_selector: css_selector.to_string(),
        text: value.as_str().map(str::to_string).unwrap_or_default(),
        note: format!(
            "resolved {property} for {} {}",
            selector_kind_label(parsed.kind),
            parsed.value
        ),
    })
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
type BidiSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
            return Err(format_runtime_exception_message(exception_details));
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
                    return Err(format_mapper_runtime_exception_message(exception_details));
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

struct BidiConnection {
    socket: BidiSocket,
    next_id: u64,
}

impl BidiConnection {
    async fn connect(bidi_websocket_url: &str) -> Result<Self, String> {
        let (socket, _) = connect_async(bidi_websocket_url).await.map_err(|error| {
            format!("failed to connect to WebDriver BiDi websocket {bidi_websocket_url}: {error}")
        })?;
        Ok(Self { socket, next_id: 1 })
    }

    async fn send_command(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send WebDriver BiDi command {method}: {error}"))?;

        loop {
            let message = self.next_json_message().await?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error").and_then(Value::as_str) {
                let message_text = message
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown WebDriver BiDi error");
                return Err(format!(
                    "WebDriver BiDi command {method} returned {error}: {message_text}"
                ));
            }

            return Ok(message);
        }
    }

    async fn wait_for_top_level_context(&mut self) -> Result<String, String> {
        for _ in 0..100 {
            let tree = self
                .send_command(
                    "browsingContext.getTree",
                    json!({
                        "maxDepth": 0,
                    }),
                )
                .await?;
            if let Some(context_id) = tree
                .pointer("/result/contexts")
                .and_then(Value::as_array)
                .and_then(|contexts| contexts.first())
                .and_then(|context| context.get("context"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                return Ok(context_id.to_string());
            }
            sleep(Duration::from_millis(50)).await;
        }

        Err("timed out waiting for an initial Firefox browsing context".to_string())
    }

    async fn next_json_message(&mut self) -> Result<Value, String> {
        loop {
            let message = self
                .socket
                .next()
                .await
                .ok_or_else(|| "WebDriver BiDi websocket closed unexpectedly".to_string())?
                .map_err(|error| {
                    format!("failed to read WebDriver BiDi websocket message: {error}")
                })?;

            match message {
                Message::Text(text) => {
                    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
                        format!("failed to parse WebDriver BiDi JSON message: {error}")
                    })?;
                    return Ok(value);
                }
                Message::Binary(bytes) => {
                    let value = serde_json::from_slice::<Value>(&bytes).map_err(|error| {
                        format!("failed to parse WebDriver BiDi JSON message: {error}")
                    })?;
                    return Ok(value);
                }
                Message::Ping(payload) => {
                    self.socket
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|error| {
                            format!("failed to reply to WebDriver BiDi ping: {error}")
                        })?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Close(frame) => {
                    return Err(format!("WebDriver BiDi websocket closed: {frame:?}"));
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

fn format_runtime_exception_message(exception_details: &Value) -> String {
    format_exception_message(
        "Runtime.evaluate failed",
        exception_details,
        "Runtime.evaluate failed with exception details",
    )
}

fn format_mapper_runtime_exception_message(exception_details: &Value) -> String {
    format_exception_message(
        "mapper Runtime.evaluate failed",
        exception_details,
        "mapper Runtime.evaluate raised exception details",
    )
}

fn format_exception_message(
    prefix: &str,
    exception_details: &Value,
    fallback_prefix: &str,
) -> String {
    let message = exception_details
        .pointer("/exception/preview/properties")
        .and_then(Value::as_array)
        .and_then(|properties| {
            properties.iter().find_map(|property| {
                let name = property.get("name").and_then(Value::as_str)?;
                let value = property.get("value").and_then(Value::as_str)?;
                (name == "message").then_some(value)
            })
        })
        .or_else(|| {
            exception_details
                .pointer("/exception/message")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            exception_details
                .pointer("/exception/description")
                .and_then(Value::as_str)
        })
        .unwrap_or_default();

    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(selector) = extract_invalid_selector(&compact) {
        return format!("{prefix}: invalid selector {selector}");
    }
    if !compact.is_empty() {
        return format!(
            "{prefix}: {}",
            compact
                .trim_start_matches("SyntaxError: ")
                .trim_start_matches("DOMException: ")
        );
    }
    format!("{fallback_prefix}: {exception_details}")
}

fn extract_invalid_selector(message: &str) -> Option<String> {
    let prefix = "Failed to execute 'querySelectorAll' on 'Document': '";
    let suffix = "' is not a valid selector.";
    let start = message.find(prefix)? + prefix.len();
    let end = message[start..].find(suffix)? + start;
    Some(message[start..end].to_string())
}

fn block_on_plugin_future<T, F>(future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let runtime = plugin_runtime()?;
    runtime.block_on(future)
}

fn plugin_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    if let Some(runtime) = PLUGIN_RUNTIME.get() {
        return Ok(runtime);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .thread_name("allwright-web-plugin")
        .build()
        .map_err(|error| format!("failed to create plugin runtime: {error}"))?;
    let _ = PLUGIN_RUNTIME.set(runtime);
    PLUGIN_RUNTIME
        .get()
        .ok_or_else(|| "failed to initialize plugin runtime".to_string())
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
        } => {
            launch_browser(browser_kind, browser_binary.as_deref()).map(PluginResult::LaunchBrowser)
        }
        PluginCommand::OpenPage { browser_session } => block_on_plugin_future(async move {
            open_page(&browser_session)
                .await
                .map(PluginResult::OpenPage)
        }),
        PluginCommand::ClosePage {
            browser_session,
            page_session,
        } => block_on_plugin_future(async move {
            close_page(&browser_session, &page_session).await?;
            Ok(PluginResult::ClosePage)
        }),
        PluginCommand::NavigatePage {
            browser_session,
            page_session,
            url,
        } => block_on_plugin_future(async move {
            navigate_page(&browser_session, &page_session, &url)
                .await
                .map(PluginResult::NavigatePage)
        }),
        PluginCommand::ClickElement {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            click_element(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::ClickElement)
        }),
        PluginCommand::CountElements {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            count_elements(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::CountElements)
        }),
        PluginCommand::HighlightElements {
            browser_session,
            page_session,
            css_selector,
            duration_ms,
        } => block_on_plugin_future(async move {
            highlight_elements(&browser_session, &page_session, &css_selector, duration_ms)
                .await
                .map(PluginResult::HighlightElements)
        }),
        PluginCommand::FocusElement {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            focus_element(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::FocusElement)
        }),
        PluginCommand::FillElement {
            browser_session,
            page_session,
            css_selector,
            value,
        } => block_on_plugin_future(async move {
            fill_element(&browser_session, &page_session, &css_selector, &value)
                .await
                .map(PluginResult::FillElement)
        }),
        PluginCommand::HoverElement {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            hover_element(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::HoverElement)
        }),
        PluginCommand::PressKey {
            browser_session,
            page_session,
            css_selector,
            key,
            text,
        } => block_on_plugin_future(async move {
            press_key(
                &browser_session,
                &page_session,
                &css_selector,
                &key,
                text.as_deref(),
            )
            .await
            .map(PluginResult::PressKey)
        }),
        PluginCommand::GetTextContent {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            get_text_content(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::GetTextContent)
        }),
        PluginCommand::GetInnerText {
            browser_session,
            page_session,
            css_selector,
        } => block_on_plugin_future(async move {
            get_inner_text(&browser_session, &page_session, &css_selector)
                .await
                .map(PluginResult::GetInnerText)
        }),
        PluginCommand::WaitForSelector {
            browser_session,
            page_session,
            css_selector,
            visible,
        } => block_on_plugin_future(async move {
            wait_for_selector(&browser_session, &page_session, &css_selector, visible)
                .await
                .map(PluginResult::WaitForSelector)
        }),
        PluginCommand::OpenChromeWindow { chrome_binary } => {
            open_chrome_window(chrome_binary.as_deref()).map(PluginResult::OpenChromeWindow)
        }
        PluginCommand::DiscoverInitialTab { cdp_websocket_url } => {
            block_on_plugin_future(async move {
                let result = discover_initial_tab(&cdp_websocket_url).await?;
                Ok(PluginResult::DiscoverInitialTab(result))
            })
        }
        PluginCommand::OpenChromeTab { cdp_websocket_url } => block_on_plugin_future(async move {
            let result = open_chrome_tab(&cdp_websocket_url).await?;
            Ok(PluginResult::OpenChromeTab(result))
        }),
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
            let result =
                click_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
            Ok(PluginResult::ClickElementViaCdp(result))
        }),
        PluginCommand::CountElementsViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result =
                count_elements_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
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
            let result = highlight_elements_via_cdp(
                &cdp_websocket_url,
                &target_id,
                &css_selector,
                duration_ms,
            )
            .await?;
            Ok(PluginResult::HighlightElementsViaCdp(result))
        }),
        PluginCommand::FocusElementViaCdp {
            cdp_websocket_url,
            target_id,
            css_selector,
        } => block_on_plugin_future(async move {
            let result =
                focus_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
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
            let result =
                hover_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
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
            let result =
                get_inner_text_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await?;
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
            return plugin_response(Err(format!("plugin request is not valid UTF-8: {error}")));
        }
    };

    let command: PluginCommand = match serde_json::from_str(request) {
        Ok(command) => command,
        Err(error) => {
            return plugin_response(Err(format!("failed to parse plugin request JSON: {error}")));
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

#[cfg(test)]
mod tests {
    use super::{SelectorKind, parse_selector};

    #[test]
    fn parse_selector_detects_explicit_xpath_prefix() {
        let parsed = parse_selector("click_element", "xpath=//button[normalize-space()='Save']")
            .expect("selector should parse");
        assert_eq!(parsed.kind, SelectorKind::XPath);
        assert_eq!(parsed.value, "//button[normalize-space()='Save']");
    }

    #[test]
    fn parse_selector_detects_xpath_heuristically() {
        let parsed = parse_selector("click_element", "//div[@role='dialog']")
            .expect("selector should parse");
        assert_eq!(parsed.kind, SelectorKind::XPath);
    }

    #[test]
    fn parse_selector_preserves_css_by_default() {
        let parsed = parse_selector("click_element", "button[data-testid='save']")
            .expect("selector should parse");
        assert_eq!(parsed.kind, SelectorKind::Css);
        assert_eq!(parsed.value, "button[data-testid='save']");
    }

    #[test]
    fn parse_selector_decodes_json_quoted_xpath_payload() {
        let parsed = parse_selector(
            "click_element",
            r#"xpath="//button[normalize-space()='Save Changes']""#,
        )
        .expect("selector should parse");
        assert_eq!(parsed.kind, SelectorKind::XPath);
        assert_eq!(parsed.value, "//button[normalize-space()='Save Changes']");
    }

    #[test]
    fn parse_selector_chain_handles_quoted_hybrid_segments() {
        let segments = super::parse_selector_chain(
            "click_element",
            r#"xpath="//div[@role='dialog']" css="button.primary""#,
        )
        .expect("selector chain should parse");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].kind, SelectorKind::XPath);
        assert_eq!(segments[0].value, "//div[@role='dialog']");
        assert_eq!(segments[1].kind, SelectorKind::Css);
        assert_eq!(segments[1].value, "button.primary");
    }
}
