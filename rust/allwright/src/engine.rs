use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::plugin_loader as web_lib;
use crate::proto;
use allwright_plugin_sdk::{BrowserSessionHandle, PageSessionHandle};
use allwright_surface_mobile::{
    ConnectOptions as MobileConnectOptions, DeviceConnectionKind as MobileDeviceConnectionKind,
    LaunchOptions as MobileLaunchOptions, MobileBrowserSessionHandle, MobilePageSessionHandle,
    MobilePlatform,
};
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, Instant, sleep};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

use proto::engine_service_server::{EngineService, EngineServiceServer};
use proto::{
    AppLaunchedEvent, BrowserKind, BrowserLaunchedEvent, ChromeLaunchedEvent,
    ChromiumBidiInjectionEvent, ClickElementCommand, CloseContextSessionCommand,
    CloseSurfaceSessionCommand, CommandRetryOptions, ConnectMobileCommand, ContextOpenedEvent,
    ContextSessionAttachedEvent, ContextSessionClosedEvent, ContextSessionCommand,
    ContextSessionErrorEvent, ContextSessionEvent, ContextSessionPingCommand,
    ContextSessionPongEvent, CountElementsCommand, DeviceConnectionKind, ElementClickedEvent,
    ElementCountedEvent, ElementFilledEvent, ElementFocusedEvent, ElementHoveredEvent,
    ElementsHighlightedEvent, FillElementCommand, FocusElementCommand, GetInnerTextCommand,
    GetTextContentCommand, HighlightElementsCommand, HoverElementCommand, InnerTextResolvedEvent,
    KeyPressedEvent, LaunchAppCommand, LaunchBrowserCommand, LaunchChromeCommand,
    MobileConnectedEvent, MobilePlatform as ProtoMobilePlatform, NavigatePageCommand,
    OpenContextCommand, PageNavigatedEvent, PingRequest, PingResponse, PressKeyCommand,
    ScreenshotCapturedEvent, ScreenshotCommand, SelectorWaitSatisfiedEvent, SessionPingCommand,
    SessionPongEvent, SurfaceSessionClosedEvent, SurfaceSessionCommand, SurfaceSessionErrorEvent,
    SurfaceSessionEvent, TextContentResolvedEvent, WaitForSelectorCommand,
    context_session_command::Command as ContextCommand,
    context_session_event::Event as ContextEvent,
    surface_session_command::Command as SurfaceCommand,
    surface_session_event::Event as SurfaceEvent,
};

static BROWSER_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static TAB_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct BrowserAutomationState {
    bidi_session_id: String,
    mapper_target_id: Option<String>,
    mapper_session_id: Option<String>,
    package_version: Option<String>,
}

#[derive(Debug, Default)]
struct BrowserSessionState {
    launched: bool,
    surface_session: Option<EngineBrowserSessionHandle>,
    process_id: Option<u32>,
    automation: Option<BrowserAutomationState>,
}

#[derive(Debug, Clone)]
struct TabSessionState {
    surface_session_id: String,
    page_session: EnginePageSessionHandle,
    current_url: Option<String>,
}

#[derive(Debug, Clone)]
enum EngineBrowserSessionHandle {
    Web(BrowserSessionHandle),
    Mobile(MobileBrowserSessionHandle),
}

#[derive(Debug, Clone)]
enum EnginePageSessionHandle {
    Web(PageSessionHandle),
    Mobile(MobilePageSessionHandle),
}

struct EnginePageOpenResult {
    note: String,
    page_session: EnginePageSessionHandle,
}

impl EnginePageSessionHandle {
    fn from_web_page(page: allwright_plugin_sdk::PageInfo) -> EnginePageOpenResult {
        EnginePageOpenResult {
            note: page.note,
            page_session: EnginePageSessionHandle::Web(page.page_session),
        }
    }

    fn from_mobile_page(page: allwright_surface_mobile::MobilePageInfo) -> EnginePageOpenResult {
        EnginePageOpenResult {
            note: page.note,
            page_session: EnginePageSessionHandle::Mobile(page.page_session),
        }
    }
}

fn mobile_platform_from_proto(value: i32) -> Result<MobilePlatform, String> {
    match ProtoMobilePlatform::try_from(value).unwrap_or(ProtoMobilePlatform::Unspecified) {
        ProtoMobilePlatform::Android => Ok(MobilePlatform::Android),
        ProtoMobilePlatform::Ios => Ok(MobilePlatform::Ios),
        ProtoMobilePlatform::Unspecified => {
            Err("connect_mobile requires a supported platform".to_string())
        }
    }
}

fn proto_mobile_platform(value: MobilePlatform) -> ProtoMobilePlatform {
    match value {
        MobilePlatform::Android => ProtoMobilePlatform::Android,
        MobilePlatform::Ios => ProtoMobilePlatform::Ios,
    }
}

fn proto_device_connection_kind(value: MobileDeviceConnectionKind) -> DeviceConnectionKind {
    match value {
        MobileDeviceConnectionKind::Usb => DeviceConnectionKind::Usb,
        MobileDeviceConnectionKind::Emulator => DeviceConnectionKind::Emulator,
        MobileDeviceConnectionKind::RemoteAdb => DeviceConnectionKind::RemoteAdb,
    }
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

type SurfaceSessionStream =
    Pin<Box<dyn Stream<Item = Result<SurfaceSessionEvent, Status>> + Send + 'static>>;
type ContextSessionStream =
    Pin<Box<dyn Stream<Item = Result<ContextSessionEvent, Status>> + Send + 'static>>;

struct CommandOutcome {
    event: SurfaceSessionEvent,
    should_close: bool,
}

struct TabCommandOutcome {
    events: Vec<ContextSessionEvent>,
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

fn next_surface_session_id() -> String {
    format!(
        "browser-session-{}",
        BROWSER_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_context_session_id() -> String {
    format!(
        "tab-session-{}",
        TAB_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn browser_event(session_id: &str, event: SurfaceEvent) -> SurfaceSessionEvent {
    SurfaceSessionEvent {
        session_id: session_id.to_string(),
        event: Some(event),
    }
}

fn tab_event(context_session_id: &str, event: ContextEvent) -> ContextSessionEvent {
    ContextSessionEvent {
        context_session_id: context_session_id.to_string(),
        event: Some(event),
    }
}

async fn handle_browser_command(
    state: Arc<Mutex<EngineState>>,
    session_id: &str,
    command: SurfaceSessionCommand,
) -> Result<CommandOutcome, Status> {
    match command.command {
        Some(SurfaceCommand::LaunchBrowser(LaunchBrowserCommand {
            browser_kind,
            browser_binary,
            retry_options,
        })) => {
            let browser_kind =
                BrowserKind::try_from(browser_kind).unwrap_or(BrowserKind::Unspecified);
            match browser_kind {
                BrowserKind::Chromium | BrowserKind::Firefox => {
                    let retry_policy = command_retry_policy(retry_options.as_ref());
                    let launch = retry_with_timeout(retry_policy, || async {
                        web_lib::launch_browser(
                            match browser_kind {
                                BrowserKind::Chromium => {
                                    allwright_plugin_sdk::BrowserKind::Chromium
                                }
                                BrowserKind::Firefox => allwright_plugin_sdk::BrowserKind::Firefox,
                                BrowserKind::Unspecified => unreachable!(),
                            },
                            browser_binary.as_deref(),
                        )
                        .await
                    })
                    .await
                    .map_err(Status::internal)?;
                    let initial_page_session_id = next_context_session_id();
                    state.lock().await.browser_sessions.insert(
                        session_id.to_string(),
                        BrowserSessionState {
                            launched: true,
                            surface_session: Some(EngineBrowserSessionHandle::Web(
                                launch.browser_session.clone(),
                            )),
                            process_id: Some(launch.process_id),
                            automation: None,
                        },
                    );
                    state.lock().await.tab_sessions.insert(
                        initial_page_session_id.clone(),
                        TabSessionState {
                            surface_session_id: session_id.to_string(),
                            page_session: EnginePageSessionHandle::Web(
                                launch.initial_page.page_session.clone(),
                            ),
                            current_url: None,
                        },
                    );

                    Ok(CommandOutcome {
                        event: browser_event(
                            session_id,
                            SurfaceEvent::BrowserLaunched(BrowserLaunchedEvent {
                                browser_kind: browser_kind as i32,
                                browser: launch.browser,
                                note: format!("{}; {}", launch.note, launch.initial_page.note),
                                user_data_dir: launch.user_data_dir,
                                initial_page_session_id,
                            }),
                        ),
                        should_close: false,
                    })
                }
                BrowserKind::Unspecified => Ok(CommandOutcome {
                    event: browser_event(
                        session_id,
                        SurfaceEvent::Error(SurfaceSessionErrorEvent {
                            message: "launch_browser requires a supported browser_kind".to_string(),
                        }),
                    ),
                    should_close: false,
                }),
            }
        }
        Some(SurfaceCommand::LaunchChrome(LaunchChromeCommand {
            chrome_binary,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let (launch, initial_tab) = retry_with_timeout(retry_policy, || async {
                let launch = web_lib::open_chrome_window(chrome_binary.as_deref()).await?;
                let initial_tab = web_lib::discover_initial_tab(&launch.cdp_websocket_url).await?;
                Ok((launch, initial_tab))
            })
            .await
            .map_err(Status::internal)?;
            let initial_page_session_id = next_context_session_id();
            state.lock().await.browser_sessions.insert(
                session_id.to_string(),
                BrowserSessionState {
                    launched: true,
                    surface_session: Some(EngineBrowserSessionHandle::Web(
                        BrowserSessionHandle::Chromium {
                            cdp_websocket_url: launch.cdp_websocket_url.clone(),
                        },
                    )),
                    process_id: Some(launch.process_id),
                    automation: None,
                },
            );
            state.lock().await.tab_sessions.insert(
                initial_page_session_id.clone(),
                TabSessionState {
                    surface_session_id: session_id.to_string(),
                    page_session: EnginePageSessionHandle::Web(PageSessionHandle::Chromium {
                        target_id: initial_tab.target_id,
                        browsing_context_id: None,
                    }),
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    SurfaceEvent::ChromeLaunched(ChromeLaunchedEvent {
                        browser: launch.browser,
                        note: format!("{}; {}", launch.note, initial_tab.note),
                        cdp_websocket_url: launch.cdp_websocket_url,
                        user_data_dir: launch.user_data_dir,
                        initial_page_session_id,
                    }),
                ),
                should_close: false,
            })
        }
        Some(SurfaceCommand::OpenContext(OpenContextCommand { retry_options })) => {
            let surface_session = {
                let state = state.lock().await;
                match state.browser_sessions.get(session_id) {
                    Some(surface_session) if surface_session.launched => {
                        surface_session.surface_session.clone().ok_or_else(|| {
                            Status::internal("browser session is missing backend session metadata")
                        })?
                    }
                    Some(_) => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                SurfaceEvent::Error(SurfaceSessionErrorEvent {
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
                                SurfaceEvent::Error(SurfaceSessionErrorEvent {
                                    message: "browser session is not registered".to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let page = retry_with_timeout(retry_policy, || async {
                match &surface_session {
                    EngineBrowserSessionHandle::Web(surface_session) => {
                        web_lib::open_page(surface_session)
                            .await
                            .map(EnginePageSessionHandle::from_web_page)
                    }
                    EngineBrowserSessionHandle::Mobile(surface_session) => {
                        web_lib::open_mobile_page(surface_session)
                            .await
                            .map(EnginePageSessionHandle::from_mobile_page)
                    }
                }
            })
            .await
            .map_err(Status::internal)?;
            let context_session_id = next_context_session_id();
            state.lock().await.tab_sessions.insert(
                context_session_id.clone(),
                TabSessionState {
                    surface_session_id: session_id.to_string(),
                    page_session: page.page_session,
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    SurfaceEvent::ContextOpened(ContextOpenedEvent {
                        context_session_id,
                        note: page.note,
                    }),
                ),
                should_close: false,
            })
        }
        Some(SurfaceCommand::ConnectMobile(ConnectMobileCommand {
            platform,
            device,
            adb_endpoint,
            preserve_app_state,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let connect = retry_with_timeout(retry_policy, || async {
                web_lib::connect_mobile(MobileConnectOptions {
                    platform: mobile_platform_from_proto(platform)?,
                    device: device.clone(),
                    adb_endpoint: adb_endpoint.clone(),
                    preserve_app_state,
                    timeout_ms: retry_options
                        .as_ref()
                        .and_then(|options| options.timeout_ms),
                })
                .await
            })
            .await
            .map_err(Status::internal)?;
            let initial_page_session_id = next_context_session_id();
            state.lock().await.browser_sessions.insert(
                session_id.to_string(),
                BrowserSessionState {
                    launched: true,
                    surface_session: Some(EngineBrowserSessionHandle::Mobile(
                        connect.browser_session.clone(),
                    )),
                    process_id: None,
                    automation: None,
                },
            );
            state.lock().await.tab_sessions.insert(
                initial_page_session_id.clone(),
                TabSessionState {
                    surface_session_id: session_id.to_string(),
                    page_session: EnginePageSessionHandle::Mobile(
                        connect.initial_page.page_session.clone(),
                    ),
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    SurfaceEvent::MobileConnected(MobileConnectedEvent {
                        platform: proto_mobile_platform(connect.browser_session.platform) as i32,
                        device_name: connect.browser,
                        note: format!("{}; {}", connect.note, connect.initial_page.note),
                        device_id: connect.browser_session.device.device_id,
                        connection_kind: proto_device_connection_kind(
                            connect.browser_session.device.connection_kind,
                        ) as i32,
                        backend: connect.browser_session.automation.backend,
                        device_session_id: connect.browser_session.automation.session_id,
                        initial_app_session_id: initial_page_session_id,
                        package_name: connect.initial_page.page_session.package_name,
                        activity_name: connect.initial_page.page_session.activity_name,
                    }),
                ),
                should_close: false,
            })
        }
        Some(SurfaceCommand::LaunchApp(LaunchAppCommand {
            apk_path,
            app_id,
            launch_activity,
            stop_before_launch,
            retry_options,
        })) => {
            let surface_session = {
                let state = state.lock().await;
                match state.browser_sessions.get(session_id) {
                    Some(surface_session) if surface_session.launched => {
                        surface_session.surface_session.clone().ok_or_else(|| {
                            Status::internal("browser session is missing backend session metadata")
                        })?
                    }
                    Some(_) => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                SurfaceEvent::Error(SurfaceSessionErrorEvent {
                                    message: "browser must be connected before launching an app"
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
                                SurfaceEvent::Error(SurfaceSessionErrorEvent {
                                    message: "browser session is not registered".to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                }
            };

            let EngineBrowserSessionHandle::Mobile(surface_session) = surface_session else {
                return Ok(CommandOutcome {
                    event: browser_event(
                        session_id,
                        SurfaceEvent::Error(SurfaceSessionErrorEvent {
                            message: "launch_app is only supported for mobile browser sessions"
                                .to_string(),
                        }),
                    ),
                    should_close: false,
                });
            };

            let retry_policy = command_retry_policy(retry_options.as_ref());
            let page = retry_with_timeout(retry_policy, || async {
                web_lib::launch_mobile_app(
                    &surface_session,
                    MobileLaunchOptions {
                        apk_path: apk_path.clone(),
                        app_id: app_id.clone(),
                        launch_activity: launch_activity.clone(),
                        stop_before_launch,
                        timeout_ms: retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    },
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            let context_session_id = next_context_session_id();
            state.lock().await.tab_sessions.insert(
                context_session_id.clone(),
                TabSessionState {
                    surface_session_id: session_id.to_string(),
                    page_session: EnginePageSessionHandle::Mobile(page.page_session.clone()),
                    current_url: None,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    SurfaceEvent::AppLaunched(AppLaunchedEvent {
                        app_session_id: context_session_id,
                        note: page.note,
                        package_name: page.page_session.package_name,
                        activity_name: page.page_session.activity_name,
                        webview_context: page.page_session.webview_context,
                    }),
                ),
                should_close: false,
            })
        }
        Some(SurfaceCommand::Ping(SessionPingCommand { message })) => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                SurfaceEvent::Pong(SessionPongEvent {
                    message: if message.is_empty() {
                        "pong".to_string()
                    } else {
                        format!("pong: {message}")
                    },
                }),
            ),
            should_close: false,
        }),
        Some(SurfaceCommand::Close(CloseSurfaceSessionCommand {})) => {
            let (process_id, surface_session) = {
                let state = state.lock().await;
                let session = state.browser_sessions.get(session_id);
                (
                    session.and_then(|session| session.process_id),
                    session.and_then(|session| session.surface_session.clone()),
                )
            };
            if let Some(process_id) = process_id {
                web_lib::close_browser_process(process_id).map_err(Status::internal)?;
            } else if let Some(EngineBrowserSessionHandle::Mobile(_)) = surface_session {
                // Mobile sessions currently have no process to tear down at the engine layer.
            }
            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    SurfaceEvent::Closed(SurfaceSessionClosedEvent {
                        reason: "browser session closed by client".to_string(),
                    }),
                ),
                should_close: true,
            })
        }
        None => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                SurfaceEvent::Error(SurfaceSessionErrorEvent {
                    message: "browser session command payload is missing".to_string(),
                }),
            ),
            should_close: false,
        }),
    }
}

async fn handle_tab_command(
    state: Arc<Mutex<EngineState>>,
    command: ContextSessionCommand,
) -> Result<TabCommandOutcome, Status> {
    let surface_session_id = command.surface_session_id;
    let context_session_id = command.context_session_id;
    if surface_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                if context_session_id.trim().is_empty() {
                    "unknown-tab-session"
                } else {
                    &context_session_id
                },
                ContextEvent::Error(ContextSessionErrorEvent {
                    message: "surface_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    if context_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                "unknown-tab-session",
                ContextEvent::Error(ContextSessionErrorEvent {
                    message: "context_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    let (surface_session, page_session, existing_automation) = {
        let state = state.lock().await;
        let surface_session = match state.browser_sessions.get(&surface_session_id) {
            Some(surface_session) => surface_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!(
                                "surface_session_id {surface_session_id} is not active"
                            ),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        let context_session = match state.tab_sessions.get(&context_session_id) {
            Some(context_session) => context_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!("unknown context_session_id {context_session_id}"),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        if context_session.surface_session_id != surface_session_id {
            return Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::Error(ContextSessionErrorEvent {
                        message: format!(
                            "context_session_id {context_session_id} belongs to surface_session_id {}, not {surface_session_id}",
                            context_session.surface_session_id
                        ),
                    }),
                )],
                should_close: false,
            });
        }

        (
            surface_session.surface_session.clone().ok_or_else(|| {
                Status::internal("browser session is missing backend session metadata")
            })?,
            context_session.page_session.clone(),
            surface_session.automation.clone(),
        )
    };

    match command.command {
        Some(ContextCommand::Ping(ContextSessionPingCommand { message })) => {
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::Pong(ContextSessionPongEvent {
                        message: if message.is_empty() {
                            "tab-pong".to_string()
                        } else {
                            format!("tab-pong: {message}")
                        },
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::Close(CloseContextSessionCommand {})) => {
            match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => {
                    web_lib::close_page(surface_session, page_session)
                        .await
                        .map_err(Status::internal)?;
                }
                (
                    EngineBrowserSessionHandle::Mobile(surface_session),
                    EnginePageSessionHandle::Mobile(page_session),
                ) => {
                    web_lib::close_mobile_page(surface_session, page_session)
                        .await
                        .map_err(Status::internal)?;
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            }
            state.lock().await.tab_sessions.remove(&context_session_id);
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::Closed(ContextSessionClosedEvent {
                        reason: "tab session closed by client".to_string(),
                    }),
                )],
                should_close: true,
            })
        }
        Some(ContextCommand::Navigate(NavigatePageCommand { url, retry_options })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "navigate is not supported for mobile tab sessions"
                                    .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let navigation = retry_with_timeout(retry_policy, || async {
                web_lib::navigate_page(&surface_session, &page_session, &url).await
            })
            .await
            .map_err(Status::internal)?;
            {
                let mut state = state.lock().await;
                let context_session = state
                    .tab_sessions
                    .get_mut(&context_session_id)
                    .ok_or_else(|| Status::internal("tab session disappeared during navigation"))?;
                context_session.page_session =
                    EnginePageSessionHandle::Web(navigation.page_session.clone());
                context_session.current_url = Some(navigation.url.clone());
                if let Some(surface_session) = state.browser_sessions.get_mut(&surface_session_id) {
                    surface_session.automation = Some(BrowserAutomationState {
                        bidi_session_id: navigation.automation.bidi_session_id.clone(),
                        mapper_target_id: navigation.automation.mapper_target_id.clone(),
                        mapper_session_id: navigation.automation.mapper_session_id.clone(),
                        package_version: navigation.automation.package_version.clone(),
                    });
                }
            }
            Ok(TabCommandOutcome {
                events: vec![
                    tab_event(
                        &context_session_id,
                        ContextEvent::Navigated(PageNavigatedEvent {
                            url: navigation.url,
                            note: navigation.note,
                        }),
                    ),
                    tab_event(
                        &context_session_id,
                        ContextEvent::ChromiumBidiInjection(ChromiumBidiInjectionEvent {
                            note: navigation.automation.note,
                            bidi_session_id: navigation.automation.bidi_session_id,
                            mapper_target_id: navigation
                                .automation
                                .mapper_target_id
                                .unwrap_or_default(),
                            mapper_session_id: navigation
                                .automation
                                .mapper_session_id
                                .unwrap_or_default(),
                            package_version: navigation
                                .automation
                                .package_version
                                .unwrap_or_default(),
                        }),
                    ),
                ],
                should_close: false,
            })
        }
        Some(ContextCommand::ClickElement(ClickElementCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let click = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::click_element(surface_session, page_session, &css_selector)
                        .await
                        .map(|click| (click.css_selector, click.note, click.bidi_session_id)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::click_mobile_element(
                        surface_session,
                        page_session,
                        &css_selector,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|click| (click.selector, click.note, click.session_id)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementClicked(ElementClickedEvent {
                        css_selector: click.0,
                        note: click.1,
                        bidi_session_id: if click.2.is_empty() {
                            existing_automation
                                .as_ref()
                                .map(|automation| automation.bidi_session_id.clone())
                                .unwrap_or_default()
                        } else {
                            click.2
                        },
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::CountElements(CountElementsCommand {
            css_selector,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message:
                                    "count_elements is not supported for mobile tab sessions yet"
                                        .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let count = retry_with_timeout(retry_policy, || async {
                web_lib::count_elements(&surface_session, &page_session, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementCounted(ElementCountedEvent {
                        css_selector: count.css_selector,
                        count: count.count,
                        note: count.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::HighlightElements(HighlightElementsCommand {
            css_selector,
            duration_ms,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message:
                                    "highlight_elements is not supported for mobile tab sessions"
                                        .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let highlight = retry_with_timeout(retry_policy, || async {
                web_lib::highlight_elements(
                    &surface_session,
                    &page_session,
                    &css_selector,
                    duration_ms.unwrap_or(2_000).into(),
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementsHighlighted(ElementsHighlightedEvent {
                        css_selector: highlight.css_selector,
                        count: highlight.count,
                        note: highlight.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::FocusElement(FocusElementCommand {
            css_selector,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "focus_element is not supported for mobile tab sessions"
                                    .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let focus = retry_with_timeout(retry_policy, || async {
                web_lib::focus_element(&surface_session, &page_session, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementFocused(ElementFocusedEvent {
                        css_selector: focus.css_selector,
                        note: focus.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::FillElement(FillElementCommand {
            css_selector,
            value,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let fill = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => {
                        web_lib::fill_element(surface_session, page_session, &css_selector, &value)
                            .await
                            .map(|fill| (fill.css_selector, fill.value, fill.note))
                    }
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::fill_mobile_element(
                        surface_session,
                        page_session,
                        &css_selector,
                        &value,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|fill| (fill.selector, fill.value, fill.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementFilled(ElementFilledEvent {
                        css_selector: fill.0,
                        value: fill.1,
                        note: fill.2,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::HoverElement(HoverElementCommand {
            css_selector,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "hover_element is not supported for mobile tab sessions"
                                    .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let hover = retry_with_timeout(retry_policy, || async {
                web_lib::hover_element(&surface_session, &page_session, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementHovered(ElementHoveredEvent {
                        css_selector: hover.css_selector,
                        note: hover.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::PressKey(PressKeyCommand {
            css_selector,
            key,
            text,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "press_key is not supported for mobile tab sessions"
                                    .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let press = retry_with_timeout(retry_policy, || async {
                web_lib::press_key(
                    &surface_session,
                    &page_session,
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
                    &context_session_id,
                    ContextEvent::KeyPressed(KeyPressedEvent {
                        css_selector: press.css_selector,
                        key: press.key,
                        note: press.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::GetTextContent(GetTextContentCommand {
            css_selector,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message:
                                    "get_text_content is not supported for mobile tab sessions yet"
                                        .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                web_lib::get_text_content(&surface_session, &page_session, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::TextContentResolved(TextContentResolvedEvent {
                        css_selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::GetInnerText(GetInnerTextCommand {
            css_selector,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message:
                                    "get_inner_text is not supported for mobile tab sessions yet"
                                        .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                web_lib::get_inner_text(&surface_session, &page_session, &css_selector).await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::InnerTextResolved(InnerTextResolvedEvent {
                        css_selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::WaitForSelector(WaitForSelectorCommand {
            css_selector,
            visible,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                (EngineBrowserSessionHandle::Mobile(_), EnginePageSessionHandle::Mobile(_)) => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message:
                                    "wait_for_selector is not supported for mobile tab sessions yet"
                                        .to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "tab session backend metadata is inconsistent".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let wait = retry_with_timeout(retry_policy, || async {
                web_lib::wait_for_selector(
                    &surface_session,
                    &page_session,
                    &css_selector,
                    visible.unwrap_or(false),
                )
                .await
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::SelectorWaitSatisfied(SelectorWaitSatisfiedEvent {
                        css_selector: wait.css_selector,
                        visible: wait.visible,
                        note: wait.note,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::Screenshot(ScreenshotCommand {
            retry_options,
            full_page,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let screenshot = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::screenshot_page(
                        surface_session,
                        page_session,
                        full_page.unwrap_or(false),
                    )
                    .await
                    .map(|shot| (shot.png_data, shot.note)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::screenshot_mobile(
                        surface_session,
                        page_session,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                        full_page.unwrap_or(false),
                    )
                    .await
                    .map(|shot| (shot.png_data, shot.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ScreenshotCaptured(ScreenshotCapturedEvent {
                        png_data: screenshot.0,
                        note: screenshot.1,
                    }),
                )],
                should_close: false,
            })
        }
        None => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &context_session_id,
                ContextEvent::Error(ContextSessionErrorEvent {
                    message: "tab session command payload is missing".to_string(),
                }),
            )],
            should_close: false,
        }),
    }
}

#[tonic::async_trait]
impl EngineService for EngineGrpcService {
    type SurfaceSessionStream = SurfaceSessionStream;
    type ContextSessionStream = ContextSessionStream;

    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn surface_session(
        &self,
        request: Request<tonic::Streaming<SurfaceSessionCommand>>,
    ) -> Result<Response<Self::SurfaceSessionStream>, Status> {
        let session_id = next_surface_session_id();
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
                .retain(|_, context_session| context_session.surface_session_id != session_id);
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn context_session(
        &self,
        request: Request<tonic::Streaming<ContextSessionCommand>>,
    ) -> Result<Response<Self::ContextSessionStream>, Status> {
        let mut inbound = request.into_inner();
        let state = Arc::clone(&self.state);
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            let mut attached = false;

            loop {
                match inbound.message().await {
                    Ok(Some(command)) => {
                        let attach_context_session_id = command.context_session_id.clone();
                        if !attached {
                            attached = true;
                            if tx
                                .send(Ok(tab_event(
                                    &attach_context_session_id,
                                    ContextEvent::Attached(ContextSessionAttachedEvent {
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
