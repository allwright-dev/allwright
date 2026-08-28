use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use allwright::proto;
use allwright_surface_web as web_lib;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, Instant, sleep};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

use proto::engine_service_server::{EngineService, EngineServiceServer};
use proto::{
    BrowserKind, BrowserLaunchedEvent, BrowserSessionClosedEvent, BrowserSessionCommand,
    BrowserSessionErrorEvent, BrowserSessionEvent, ChromeLaunchedEvent,
    ChromiumBidiInjectionEvent, ClickElementCommand, CloseBrowserSessionCommand,
    CloseTabSessionCommand, CommandRetryOptions, CountElementsCommand, ElementClickedEvent,
    ElementCountedEvent, ElementFilledEvent, ElementFocusedEvent, ElementHoveredEvent,
    ElementsHighlightedEvent, FillElementCommand, FocusElementCommand, GetInnerTextCommand,
    GetTextContentCommand, HighlightElementsCommand, HoverElementCommand, InnerTextResolvedEvent,
    KeyPressedEvent, LaunchBrowserCommand, LaunchChromeCommand, NavigateTabCommand, OpenTabCommand,
    PingRequest, PingResponse, PressKeyCommand, SelectorWaitSatisfiedEvent, SessionPingCommand,
    SessionPongEvent, TabNavigatedEvent, TabOpenedEvent, TabSessionAttachedEvent,
    TabSessionClosedEvent, TabSessionCommand, TabSessionErrorEvent, TabSessionEvent,
    TabSessionPingCommand, TabSessionPongEvent, TextContentResolvedEvent, WaitForSelectorCommand,
    browser_session_command::Command as BrowserCommand, browser_session_event::Event as BrowserEvent,
    tab_session_command::Command as TabCommand, tab_session_event::Event as TabEvent,
};

static BROWSER_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static TAB_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static BIDI_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct BrowserBidiMapperState {
    bidi_session_id: String,
    mapper_target_id: String,
    mapper_session_id: String,
    package_version: String,
}

#[derive(Debug, Default)]
struct BrowserSessionState {
    launched: bool,
    cdp_websocket_url: Option<String>,
    process_id: Option<u32>,
    bidi_mapper: Option<BrowserBidiMapperState>,
}

#[derive(Debug, Clone)]
struct TabSessionState {
    browser_session_id: String,
    target_id: String,
    browsing_context_id: Option<String>,
    current_url: Option<String>,
}

#[derive(Debug, Default)]
struct EngineState {
    browser_sessions: HashMap<String, BrowserSessionState>,
    tab_sessions: HashMap<String, TabSessionState>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineGrpcService {
    state: Arc<Mutex<EngineState>>,
}

type BrowserSessionStream =
    Pin<Box<dyn Stream<Item = Result<BrowserSessionEvent, Status>> + Send + 'static>>;
type TabSessionStream =
    Pin<Box<dyn Stream<Item = Result<TabSessionEvent, Status>> + Send + 'static>>;

struct CommandOutcome {
    event: BrowserSessionEvent,
    should_close: bool,
}

struct TabCommandOutcome {
    events: Vec<TabSessionEvent>,
    should_close: bool,
}

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    timeout: Duration,
    retry_interval: Duration,
}

impl RetryPolicy {
    fn from_proto(
        options: Option<&CommandRetryOptions>,
        timeout_ms: u64,
        retry_interval_ms: u64,
    ) -> Self {
        let timeout = options
            .and_then(|options| options.timeout_ms)
            .map(u64::from)
            .unwrap_or(timeout_ms);
        let retry_interval = options
            .and_then(|options| options.retry_interval_ms)
            .map(u64::from)
            .unwrap_or(retry_interval_ms);

        Self {
            timeout: Duration::from_millis(timeout.max(1)),
            retry_interval: Duration::from_millis(retry_interval.max(1)),
        }
    }
}

fn command_retry_policy(options: Option<&CommandRetryOptions>) -> RetryPolicy {
    RetryPolicy::from_proto(options, 10_000, 250)
}

async fn retry_with_timeout<T, F, Fut>(policy: RetryPolicy, mut operation: F) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let start = Instant::now();

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if start.elapsed() >= policy.timeout {
                    return Err(error);
                }
            }
        }

        sleep(policy.retry_interval).await;
    }
}

fn next_browser_session_id() -> String {
    format!(
        "browser-session-{}",
        BROWSER_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_tab_session_id() -> String {
    format!(
        "tab-session-{}",
        TAB_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_bidi_session_id() -> String {
    format!(
        "bidi-session-{}",
        BIDI_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn browser_event(session_id: &str, event: BrowserEvent) -> BrowserSessionEvent {
    BrowserSessionEvent {
        session_id: session_id.to_string(),
        event: Some(event),
    }
}

fn tab_event(tab_session_id: &str, event: TabEvent) -> TabSessionEvent {
    TabSessionEvent {
        tab_session_id: tab_session_id.to_string(),
        event: Some(event),
    }
}

async fn ensure_browser_bidi_mapper(
    state: Arc<Mutex<EngineState>>,
    browser_session_id: &str,
    cdp_websocket_url: &str,
) -> Result<(BrowserBidiMapperState, bool), Status> {
    if let Some(existing) = state
        .lock()
        .await
        .browser_sessions
        .get(browser_session_id)
        .and_then(|session| session.bidi_mapper.clone())
    {
        return Ok((existing, false));
    }

    let injected = web_lib::inject_chromium_bidi_mapper(cdp_websocket_url)
        .await
        .map_err(Status::internal)?;
    let mapper_state = BrowserBidiMapperState {
        bidi_session_id: next_bidi_session_id(),
        mapper_target_id: injected.mapper_target_id,
        mapper_session_id: injected.mapper_session_id,
        package_version: injected.package_version,
    };

    let mut state = state.lock().await;
    let browser_session = state
        .browser_sessions
        .get_mut(browser_session_id)
        .ok_or_else(|| Status::internal("browser session disappeared during mapper injection"))?;

    if let Some(existing) = browser_session.bidi_mapper.clone() {
        return Ok((existing, false));
    }

    browser_session.bidi_mapper = Some(mapper_state.clone());
    Ok((mapper_state, true))
}

async fn handle_browser_command(
    state: Arc<Mutex<EngineState>>,
    session_id: &str,
    command: BrowserSessionCommand,
) -> Result<CommandOutcome, Status> {
    match command.command {
        Some(BrowserCommand::LaunchBrowser(LaunchBrowserCommand {
            browser_kind,
            browser_binary,
            retry_options,
        })) => {
            let browser_kind = BrowserKind::try_from(browser_kind).unwrap_or(BrowserKind::Unspecified);
            match browser_kind {
                BrowserKind::Chromium => {
                    let retry_policy = command_retry_policy(retry_options.as_ref());
                    let (launch, initial_tab) = retry_with_timeout(retry_policy, || async {
                        let launch = web_lib::open_chrome_window(browser_binary.as_deref())?;
                        let initial_tab = web_lib::discover_initial_tab(&launch.cdp_websocket_url).await?;
                        Ok((launch, initial_tab))
                    })
                    .await
                    .map_err(Status::internal)?;
                    let initial_tab_session_id = next_tab_session_id();
                    state.lock().await.browser_sessions.insert(
                        session_id.to_string(),
                        BrowserSessionState {
                            launched: true,
                            cdp_websocket_url: Some(launch.cdp_websocket_url.clone()),
                            process_id: Some(launch.process_id),
                            bidi_mapper: None,
                        },
                    );
                    state.lock().await.tab_sessions.insert(
                        initial_tab_session_id.clone(),
                        TabSessionState {
                            browser_session_id: session_id.to_string(),
                            target_id: initial_tab.target_id,
                            browsing_context_id: None,
                            current_url: None,
                        },
                    );

                    Ok(CommandOutcome {
                        event: browser_event(
                            session_id,
                            BrowserEvent::BrowserLaunched(BrowserLaunchedEvent {
                                browser_kind: BrowserKind::Chromium as i32,
                                browser: launch.browser,
                                note: format!("{}; {}", launch.note, initial_tab.note),
                                user_data_dir: launch.user_data_dir,
                                initial_tab_session_id,
                            }),
                        ),
                        should_close: false,
                    })
                }
                BrowserKind::Firefox => Ok(CommandOutcome {
                    event: browser_event(
                        session_id,
                        BrowserEvent::Error(BrowserSessionErrorEvent {
                            message: "firefox backend is not implemented yet".to_string(),
                        }),
                    ),
                    should_close: false,
                }),
                BrowserKind::Unspecified => Ok(CommandOutcome {
                    event: browser_event(
                        session_id,
                        BrowserEvent::Error(BrowserSessionErrorEvent {
                            message: "launch_browser requires a supported browser_kind".to_string(),
                        }),
                    ),
                    should_close: false,
                }),
            }
        }
        Some(BrowserCommand::LaunchChrome(LaunchChromeCommand {
            chrome_binary,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let (launch, initial_tab) = retry_with_timeout(retry_policy, || async {
                let launch = web_lib::open_chrome_window(chrome_binary.as_deref())?;
                let initial_tab = web_lib::discover_initial_tab(&launch.cdp_websocket_url).await?;
                Ok((launch, initial_tab))
            })
            .await
            .map_err(Status::internal)?;
            let initial_tab_session_id = next_tab_session_id();
            state.lock().await.browser_sessions.insert(
                session_id.to_string(),
                BrowserSessionState {
                    launched: true,
                    cdp_websocket_url: Some(launch.cdp_websocket_url.clone()),
                    process_id: Some(launch.process_id),
                    bidi_mapper: None,
                },
            );
            state.lock().await.tab_sessions.insert(
                initial_tab_session_id.clone(),
                TabSessionState {
                    browser_session_id: session_id.to_string(),
                    target_id: initial_tab.target_id,
                    browsing_context_id: None,
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::ChromeLaunched(ChromeLaunchedEvent {
                        browser: launch.browser,
                        note: format!("{}; {}", launch.note, initial_tab.note),
                        cdp_websocket_url: launch.cdp_websocket_url,
                        user_data_dir: launch.user_data_dir,
                        initial_tab_session_id,
                    }),
                ),
                should_close: false,
            })
        }
        Some(BrowserCommand::OpenTab(OpenTabCommand { retry_options })) => {
            let cdp_websocket_url = {
                let state = state.lock().await;
                match state.browser_sessions.get(session_id) {
                    Some(browser_session) if browser_session.launched => {
                        browser_session.cdp_websocket_url.clone().ok_or_else(|| {
                            Status::internal("browser session is missing CDP websocket metadata")
                        })?
                    }
                    Some(_) => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                BrowserEvent::Error(BrowserSessionErrorEvent {
                                    message: "browser must be launched before opening a tab"
                                        .to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                    None => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                BrowserEvent::Error(BrowserSessionErrorEvent {
                                    message: "browser session is not registered".to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let tab = retry_with_timeout(retry_policy, || async {
                web_lib::open_chrome_tab(&cdp_websocket_url).await
            })
            .await
            .map_err(Status::internal)?;
            let tab_session_id = next_tab_session_id();
            state.lock().await.tab_sessions.insert(
                tab_session_id.clone(),
                TabSessionState {
                    browser_session_id: session_id.to_string(),
                    target_id: tab.target_id,
                    browsing_context_id: None,
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::TabOpened(TabOpenedEvent {
                        tab_session_id,
                        note: tab.note,
                    }),
                ),
                should_close: false,
            })
        }
        Some(BrowserCommand::Ping(SessionPingCommand { message })) => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                BrowserEvent::Pong(SessionPongEvent {
                    message: if message.is_empty() {
                        "pong".to_string()
                    } else {
                        format!("pong: {message}")
                    },
                }),
            ),
            should_close: false,
        }),
        Some(BrowserCommand::Close(CloseBrowserSessionCommand {})) => {
            let process_id = state
                .lock()
                .await
                .browser_sessions
                .get(session_id)
                .and_then(|session| session.process_id);
            if let Some(process_id) = process_id {
                web_lib::close_browser_process(process_id).map_err(Status::internal)?;
            }
            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::Closed(BrowserSessionClosedEvent {
                        reason: "browser session closed by client".to_string(),
                    }),
                ),
                should_close: true,
            })
        }
        None => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                BrowserEvent::Error(BrowserSessionErrorEvent {
                    message: "browser session command payload is missing".to_string(),
                }),
            ),
            should_close: false,
        }),
    }
}

async fn handle_tab_command(
    state: Arc<Mutex<EngineState>>,
    command: TabSessionCommand,
) -> Result<TabCommandOutcome, Status> {
    let browser_session_id = command.browser_session_id;
    let tab_session_id = command.tab_session_id;
    if browser_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                if tab_session_id.trim().is_empty() {
                    "unknown-tab-session"
                } else {
                    &tab_session_id
                },
                TabEvent::Error(TabSessionErrorEvent {
                    message: "browser_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    if tab_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                "unknown-tab-session",
                TabEvent::Error(TabSessionErrorEvent {
                    message: "tab_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    let (cdp_websocket_url, target_id, existing_bidi_mapper) = {
        let state = state.lock().await;
        let browser_session = match state.browser_sessions.get(&browser_session_id) {
            Some(browser_session) => browser_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &tab_session_id,
                        TabEvent::Error(TabSessionErrorEvent {
                            message: format!(
                                "browser_session_id {browser_session_id} is not active"
                            ),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        let tab_session = match state.tab_sessions.get(&tab_session_id) {
            Some(tab_session) => tab_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &tab_session_id,
                        TabEvent::Error(TabSessionErrorEvent {
                            message: format!("unknown tab_session_id {tab_session_id}"),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        if tab_session.browser_session_id != browser_session_id {
            return Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Error(TabSessionErrorEvent {
                        message: format!(
                            "tab_session_id {tab_session_id} belongs to browser_session_id {}, not {browser_session_id}",
                            tab_session.browser_session_id
                        ),
                    }),
                )],
                should_close: false,
            });
        }

        (
            browser_session.cdp_websocket_url.clone().ok_or_else(|| {
                Status::internal("browser session is missing CDP websocket metadata")
            })?,
            tab_session.target_id.clone(),
            browser_session.bidi_mapper.clone(),
        )
    };

    match command.command {
        Some(TabCommand::Ping(TabSessionPingCommand { message })) => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &tab_session_id,
                TabEvent::Pong(TabSessionPongEvent {
                    message: if message.is_empty() {
                        "tab-pong".to_string()
                    } else {
                        format!("tab-pong: {message}")
                    },
                }),
            )],
            should_close: false,
        }),
        Some(TabCommand::Close(CloseTabSessionCommand {})) => {
            web_lib::close_chrome_tab(&cdp_websocket_url, &target_id)
                .await
                .map_err(Status::internal)?;
            state.lock().await.tab_sessions.remove(&tab_session_id);
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Closed(TabSessionClosedEvent {
                        reason: "tab session closed by client".to_string(),
                    }),
                )],
                should_close: true,
            })
        }
        Some(TabCommand::Navigate(NavigateTabCommand { url, retry_options })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let (navigation, resolved_browsing_context_id, bidi_mapper, created) =
                retry_with_timeout(retry_policy, || async {
                    let navigation =
                        web_lib::navigate_chrome_tab(&cdp_websocket_url, &target_id, &url).await?;
                    let (bidi_mapper, created) = ensure_browser_bidi_mapper(
                        Arc::clone(&state),
                        &browser_session_id,
                        &cdp_websocket_url,
                    )
                    .await
                    .map_err(|status| status.message().to_string())?;
                    let (resolved_browsing_context_id, resolved_mapper) =
                        web_lib::resolve_bidi_context_for_tab(
                            &cdp_websocket_url,
                            Some(&bidi_mapper.mapper_target_id),
                            Some(&navigation.browsing_context_id),
                            Some(&navigation.url),
                        )
                        .await?;
                    Ok((
                        navigation,
                        resolved_browsing_context_id,
                        BrowserBidiMapperState {
                            bidi_session_id: bidi_mapper.bidi_session_id.clone(),
                            mapper_target_id: resolved_mapper.mapper_target_id,
                            mapper_session_id: resolved_mapper.mapper_session_id,
                            package_version: resolved_mapper.package_version,
                        },
                        created,
                    ))
                })
                .await
                .map_err(Status::internal)?;
            {
                let mut state = state.lock().await;
                let tab_session = state
                    .tab_sessions
                    .get_mut(&tab_session_id)
                    .ok_or_else(|| Status::internal("tab session disappeared during navigation"))?;
                tab_session.browsing_context_id = Some(resolved_browsing_context_id);
                tab_session.current_url = Some(navigation.url.clone());
                if let Some(browser_session) = state.browser_sessions.get_mut(&browser_session_id) {
                    browser_session.bidi_mapper = Some(bidi_mapper.clone());
                }
            }
            let bidi_note = if created {
                format!(
                    "chromium-bidi mapper injected from pinned chromium-bidi@{} into hidden mapper target after navigation",
                    bidi_mapper.package_version
                )
            } else {
                format!(
                    "reused existing chromium-bidi mapper session {} from pinned chromium-bidi@{} after navigation",
                    bidi_mapper.bidi_session_id, bidi_mapper.package_version
                )
            };
            Ok(TabCommandOutcome {
                events: vec![
                    tab_event(
                        &tab_session_id,
                        TabEvent::Navigated(TabNavigatedEvent {
                            url: navigation.url,
                            note: navigation.note,
                        }),
                    ),
                    tab_event(
                        &tab_session_id,
                        TabEvent::ChromiumBidiInjection(ChromiumBidiInjectionEvent {
                            note: bidi_note,
                            bidi_session_id: bidi_mapper.bidi_session_id,
                            mapper_target_id: bidi_mapper.mapper_target_id,
                            mapper_session_id: bidi_mapper.mapper_session_id,
                            package_version: bidi_mapper.package_version,
                        }),
                    ),
                ],
                should_close: false,
            })
        }
        Some(TabCommand::ClickElement(ClickElementCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let click = retry_with_timeout(retry_policy, || async {
                web_lib::click_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementClicked(ElementClickedEvent {
                        css_selector: click.css_selector,
                        note: click.note,
                        bidi_session_id: existing_bidi_mapper
                            .as_ref()
                            .map(|bidi_mapper| bidi_mapper.bidi_session_id.clone())
                            .unwrap_or_default(),
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::CountElements(CountElementsCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let count = retry_with_timeout(retry_policy, || async {
                web_lib::count_elements_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementCounted(ElementCountedEvent {
                        css_selector: count.css_selector,
                        count: count.count,
                        note: count.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::HighlightElements(HighlightElementsCommand {
            css_selector,
            duration_ms,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let highlight = retry_with_timeout(retry_policy, || async {
                web_lib::highlight_elements_via_cdp(
                    &cdp_websocket_url,
                    &target_id,
                    &css_selector,
                    duration_ms.unwrap_or(2_000),
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementsHighlighted(ElementsHighlightedEvent {
                        css_selector: highlight.css_selector,
                        count: highlight.count,
                        note: highlight.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::FocusElement(FocusElementCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let focus = retry_with_timeout(retry_policy, || async {
                web_lib::focus_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementFocused(ElementFocusedEvent {
                        css_selector: focus.css_selector,
                        note: focus.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::FillElement(FillElementCommand {
            css_selector,
            value,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let fill = retry_with_timeout(retry_policy, || async {
                web_lib::fill_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector, &value)
                    .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementFilled(ElementFilledEvent {
                        css_selector: fill.css_selector,
                        value: fill.value,
                        note: fill.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::HoverElement(HoverElementCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let hover = retry_with_timeout(retry_policy, || async {
                web_lib::hover_element_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementHovered(ElementHoveredEvent {
                        css_selector: hover.css_selector,
                        note: hover.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::PressKey(PressKeyCommand {
            css_selector,
            key,
            text,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let press = retry_with_timeout(retry_policy, || async {
                web_lib::press_key_via_cdp(
                    &cdp_websocket_url,
                    &target_id,
                    &css_selector,
                    &key,
                    text.as_deref(),
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::KeyPressed(KeyPressedEvent {
                        css_selector: press.css_selector,
                        key: press.key,
                        note: press.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::GetTextContent(GetTextContentCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                web_lib::get_text_content_via_cdp(&cdp_websocket_url, &target_id, &css_selector)
                    .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::TextContentResolved(TextContentResolvedEvent {
                        css_selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::GetInnerText(GetInnerTextCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                web_lib::get_inner_text_via_cdp(&cdp_websocket_url, &target_id, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::InnerTextResolved(InnerTextResolvedEvent {
                        css_selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::WaitForSelector(WaitForSelectorCommand {
            css_selector,
            visible,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let wait = retry_with_timeout(retry_policy, || async {
                web_lib::wait_for_selector_via_cdp(
                    &cdp_websocket_url,
                    &target_id,
                    &css_selector,
                    visible.unwrap_or(false),
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::SelectorWaitSatisfied(SelectorWaitSatisfiedEvent {
                        css_selector: wait.css_selector,
                        visible: wait.visible,
                        note: wait.note,
                    }),
                )],
                should_close: false,
            })
        }
        None => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &tab_session_id,
                TabEvent::Error(TabSessionErrorEvent {
                    message: "tab session command payload is missing".to_string(),
                }),
            )],
            should_close: false,
        }),
    }
}

#[tonic::async_trait]
impl EngineService for EngineGrpcService {
    type BrowserSessionStream = BrowserSessionStream;
    type TabSessionStream = TabSessionStream;

    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn browser_session(
        &self,
        request: Request<tonic::Streaming<BrowserSessionCommand>>,
    ) -> Result<Response<Self::BrowserSessionStream>, Status> {
        let session_id = next_browser_session_id();
        self.state
            .lock()
            .await
            .browser_sessions
            .insert(session_id.clone(), BrowserSessionState::default());
        let mut inbound = request.into_inner();
        let state = Arc::clone(&self.state);
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            loop {
                match inbound.message().await {
                    Ok(Some(command)) => {
                        match handle_browser_command(Arc::clone(&state), &session_id, command).await
                        {
                            Ok(outcome) => {
                                let should_close = outcome.should_close;
                                if tx.send(Ok(outcome.event)).await.is_err() {
                                    break;
                                }

                                if should_close {
                                    break;
                                }
                            }
                            Err(status) => {
                                let _ = tx.send(Err(status)).await;
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }

            let mut state = state.lock().await;
            state.browser_sessions.remove(&session_id);
            state
                .tab_sessions
                .retain(|_, tab_session| tab_session.browser_session_id != session_id);
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn tab_session(
        &self,
        request: Request<tonic::Streaming<TabSessionCommand>>,
    ) -> Result<Response<Self::TabSessionStream>, Status> {
        let mut inbound = request.into_inner();
        let state = Arc::clone(&self.state);
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            let mut attached = false;

            loop {
                match inbound.message().await {
                    Ok(Some(command)) => {
                        let attach_tab_session_id = command.tab_session_id.clone();
                        if !attached {
                            attached = true;
                            if tx
                                .send(Ok(tab_event(
                                    &attach_tab_session_id,
                                    TabEvent::Attached(TabSessionAttachedEvent {
                                        note: "tab session attached".to_string(),
                                    }),
                                )))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }

                        match handle_tab_command(Arc::clone(&state), command).await {
                            Ok(outcome) => {
                                let should_close = outcome.should_close;
                                for event in outcome.events {
                                    if tx.send(Ok(event)).await.is_err() {
                                        return;
                                    }
                                }

                                if should_close {
                                    break;
                                }
                            }
                            Err(status) => {
                                let _ = tx.send(Err(status)).await;
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

pub async fn serve(addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(EngineServiceServer::new(EngineGrpcService::default()))
        .serve(addr)
        .await
}
