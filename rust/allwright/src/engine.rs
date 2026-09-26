use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::plugin_loader as web_lib;
use crate::proto;
use allwright_plugin_sdk::{
    BrowserSessionHandle, HookRegistration, HookResult, HookType as PluginHookType,
    PageSessionHandle,
};
use allwright_surface_mobile::{
    ConnectOptions as MobileConnectOptions, DeviceConnectionKind as MobileDeviceConnectionKind,
    LaunchOptions as MobileLaunchOptions, MobileBrowserSessionHandle, MobileHookRegistration,
    MobileHookResult, MobileHookType, MobilePageSessionHandle, MobilePlatform,
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
    ContextSessionPongEvent, CountElementsCommand, DeviceConnectionKind, DownloadHookResult,
    DownloadSavedEvent, ElementClickedEvent, ElementCountedEvent, ElementFilledEvent,
    ElementFocusedEvent, ElementHoveredEvent, ElementsHighlightedEvent, FileChooserFilesSetEvent,
    FileChooserHookResult, FileChunkEvent, FileUploadedEvent, FillElementCommand,
    FocusElementCommand, GetInnerTextCommand, GetTextContentCommand, HighlightElementsCommand,
    HookCompletedEvent, HookRegisteredEvent, HoverElementCommand, InnerTextResolvedEvent,
    KeyPressedEvent, LaunchAppCommand, LaunchBrowserCommand, LaunchChromeCommand,
    MobileConnectedEvent, MobileDownloadHookResult, MobileDownloadSavedEvent,
    MobileFileChooserFilesSetEvent, MobileFileChooserHookResult,
    MobilePlatform as ProtoMobilePlatform, NavigatePageCommand, NewPageHookResult,
    OpenContextCommand, PageNavigatedEvent, PingRequest, PingResponse, PressKeyCommand,
    ReadFileChunkCommand, RegisterHookCommand, SaveDownloadCommand, SaveMobileDownloadCommand,
    ScreenshotCapturedEvent, ScreenshotCommand, SelectorWaitSatisfiedEvent, SessionPingCommand,
    SessionPongEvent, SetFileChooserFilesCommand, SetMobileFileChooserFilesCommand,
    SurfaceSessionClosedEvent, SurfaceSessionCommand, SurfaceSessionErrorEvent,
    SurfaceSessionEvent, TextContentResolvedEvent, UploadFileChunkCommand, WaitForHookCommand,
    WaitForSelectorCommand, context_session_command::Command as ContextCommand,
    context_session_event::Event as ContextEvent,
    hook_completed_event::Result as HookCompletionResult,
    register_hook_command::Hook as RegisterHook,
    surface_session_command::Command as SurfaceCommand,
    surface_session_event::Event as SurfaceEvent,
};

static BROWSER_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static TAB_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static HOOK_COUNTER: AtomicU64 = AtomicU64::new(1);
static FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

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
struct HookState {
    surface_session_id: String,
    context_session_id: String,
    registration: EngineHookRegistration,
}

#[derive(Debug, Clone)]
enum EngineHookRegistration {
    Web(HookRegistration),
    Mobile(MobileHookRegistration),
}

#[derive(Debug)]
struct FileUploadState {
    context_session_id: String,
    name: String,
    path: PathBuf,
    size: u64,
}

#[derive(Debug, Clone)]
struct StagedFile {
    context_session_id: String,
    path: PathBuf,
    size: u64,
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
    frame_sessions: std::collections::HashSet<String>,
    browser_sessions: HashMap<String, BrowserSessionState>,
    tab_sessions: HashMap<String, TabSessionState>,
    hooks: HashMap<String, HookState>,
    file_uploads: HashMap<String, FileUploadState>,
    staged_files: HashMap<String, StagedFile>,
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

async fn resolve_frame_before_deadline<T, F, Fut>(
    policy: RetryPolicy,
    operation: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    tokio::time::timeout(policy.timeout, retry_with_timeout(policy, operation))
        .await
        .unwrap_or_else(
            |_| Err("timed out waiting for frame to load and become stable".to_string()),
        )
}

async fn hook_operation_before_deadline<T>(
    options: Option<&CommandRetryOptions>,
    name: &str,
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::time::timeout(command_retry_policy(options).timeout, operation)
        .await
        .unwrap_or_else(|_| {
            Err(format!(
                "timed out {name}; operation may still complete, do not retry blindly"
            ))
        })
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

fn next_hook_id() -> String {
    format!("hook-{}", HOOK_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn next_file_id(prefix: &str) -> String {
    format!("{prefix}-{}", FILE_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn safe_transfer_name(name: &str) -> Result<String, String> {
    let name = Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .ok_or_else(|| "uploaded file requires a valid file name".to_string())?;
    Ok(name.to_string())
}

fn transfer_path(id: &str, name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("allwright-transfer-{}", std::process::id()))
        .join(id)
        .join(name)
}

fn remove_staged_file(file: &StagedFile) {
    let _ = fs::remove_file(&file.path);
    if let Some(parent) = file.path.parent() {
        let _ = fs::remove_dir(parent);
    }
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
                        mapper_target_id: None,
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
        Some(ContextCommand::UploadFileChunk(UploadFileChunkCommand {
            transfer_id,
            name,
            offset,
            data,
            last,
        })) => {
            if transfer_id.trim().is_empty() {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "upload_file_chunk requires transfer_id".to_string(),
                        }),
                    )],
                    should_close: false,
                });
            }
            if data.len() > 1024 * 1024 {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "upload file chunks must not exceed 1 MiB".to_string(),
                        }),
                    )],
                    should_close: false,
                });
            }

            let mut engine = state.lock().await;
            if !engine.file_uploads.contains_key(&transfer_id) {
                if offset != 0 {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "first upload chunk must start at offset 0".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
                let name = match safe_transfer_name(&name) {
                    Ok(name) => name,
                    Err(message) => {
                        return Ok(TabCommandOutcome {
                            events: vec![tab_event(
                                &context_session_id,
                                ContextEvent::Error(ContextSessionErrorEvent { message }),
                            )],
                            should_close: false,
                        });
                    }
                };
                let path = transfer_path(&next_file_id("upload"), &name);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| Status::internal(error.to_string()))?;
                }
                engine.file_uploads.insert(
                    transfer_id.clone(),
                    FileUploadState {
                        context_session_id: context_session_id.clone(),
                        name,
                        path,
                        size: 0,
                    },
                );
            }

            let upload = engine
                .file_uploads
                .get_mut(&transfer_id)
                .expect("upload inserted");
            if upload.context_session_id != context_session_id {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "file upload belongs to a different context session"
                                .to_string(),
                        }),
                    )],
                    should_close: false,
                });
            }
            if offset != upload.size {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!(
                                "upload chunk offset {offset} does not match expected offset {}",
                                upload.size
                            ),
                        }),
                    )],
                    should_close: false,
                });
            }
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&upload.path)
                .map_err(|error| Status::internal(error.to_string()))?;
            file.write_all(&data)
                .map_err(|error| Status::internal(error.to_string()))?;
            upload.size += data.len() as u64;

            if !last {
                return Ok(TabCommandOutcome {
                    events: Vec::new(),
                    should_close: false,
                });
            }

            let upload = engine
                .file_uploads
                .remove(&transfer_id)
                .expect("upload exists");
            let file_id = next_file_id("file");
            let event = FileUploadedEvent {
                transfer_id,
                file_id: file_id.clone(),
                name: upload.name.clone(),
                size: upload.size,
            };
            engine.staged_files.insert(
                file_id,
                StagedFile {
                    context_session_id: context_session_id.clone(),
                    path: upload.path,
                    size: upload.size,
                },
            );
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::FileUploaded(event),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::ReadFileChunk(ReadFileChunkCommand {
            file_id,
            offset,
            max_bytes,
        })) => {
            let file = state.lock().await.staged_files.get(&file_id).cloned();
            let Some(file) = file.filter(|file| file.context_session_id == context_session_id)
            else {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!("unknown staged file {file_id}"),
                        }),
                    )],
                    should_close: false,
                });
            };
            if offset > file.size {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!("file offset {offset} exceeds size {}", file.size),
                        }),
                    )],
                    should_close: false,
                });
            }
            let read_size = usize::try_from(max_bytes.clamp(1, 1024 * 1024)).unwrap_or(1024 * 1024);
            let mut source =
                fs::File::open(&file.path).map_err(|error| Status::internal(error.to_string()))?;
            source
                .seek(SeekFrom::Start(offset))
                .map_err(|error| Status::internal(error.to_string()))?;
            let mut data = vec![0; read_size];
            let count = source
                .read(&mut data)
                .map_err(|error| Status::internal(error.to_string()))?;
            data.truncate(count);
            let last = offset + count as u64 >= file.size;
            if last {
                if let Some(file) = state.lock().await.staged_files.remove(&file_id) {
                    remove_staged_file(&file);
                }
            }
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::FileChunk(FileChunkEvent {
                        file_id,
                        offset,
                        data,
                        last,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::ResolveFrame(command)) => {
            let policy = command_retry_policy(command.retry_options.as_ref());
            let started = Instant::now();
            let result = resolve_frame_before_deadline(policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(browser),
                        EnginePageSessionHandle::Web(page),
                    ) => {
                        web_lib::resolve_frame(
                            browser,
                            page,
                            &command.css_selector,
                            policy.timeout.saturating_sub(started.elapsed()).as_millis() as u64,
                        )
                        .await
                    }
                    _ => Err("frames are only supported on web pages".to_string()),
                }
            })
            .await;
            let event = match result {
                Ok(page) => {
                    let id = next_context_session_id();
                    let mut state = state.lock().await;
                    state.frame_sessions.insert(id.clone());
                    state.tab_sessions.insert(
                        id.clone(),
                        TabSessionState {
                            surface_session_id: surface_session_id.clone(),
                            page_session: EnginePageSessionHandle::Web(page.page_session),
                            current_url: None,
                        },
                    );
                    ContextEvent::FrameResolved(proto::FrameResolvedEvent {
                        context_session_id: id,
                    })
                }
                Err(message) => ContextEvent::Error(ContextSessionErrorEvent { message }),
            };
            Ok(TabCommandOutcome {
                events: vec![tab_event(&context_session_id, event)],
                should_close: false,
            })
        }
        Some(ContextCommand::RegisterHook(RegisterHookCommand { hook })) => {
            let registration = match (hook, &surface_session, &page_session) {
                (
                    Some(RegisterHook::Dialog(_)),
                    EngineBrowserSessionHandle::Web(browser),
                    EnginePageSessionHandle::Web(page),
                ) => web_lib::register_hook(browser, page, PluginHookType::Dialog)
                    .await
                    .map(EngineHookRegistration::Web),
                (
                    Some(RegisterHook::NewPage(_)),
                    EngineBrowserSessionHandle::Web(browser),
                    EnginePageSessionHandle::Web(page),
                ) => web_lib::register_hook(browser, page, PluginHookType::NewPage)
                    .await
                    .map(EngineHookRegistration::Web),
                (
                    Some(RegisterHook::FileChooser(_)),
                    EngineBrowserSessionHandle::Web(browser),
                    EnginePageSessionHandle::Web(page),
                ) => web_lib::register_hook(browser, page, PluginHookType::FileChooser)
                    .await
                    .map(EngineHookRegistration::Web),
                (
                    Some(RegisterHook::Download(_)),
                    EngineBrowserSessionHandle::Web(browser),
                    EnginePageSessionHandle::Web(page),
                ) => web_lib::register_hook(browser, page, PluginHookType::Download)
                    .await
                    .map(EngineHookRegistration::Web),
                (
                    Some(RegisterHook::MobileFileChooser(_)),
                    EngineBrowserSessionHandle::Mobile(browser),
                    EnginePageSessionHandle::Mobile(page),
                ) => web_lib::register_mobile_hook(browser, page, MobileHookType::FileChooser)
                    .await
                    .map(EngineHookRegistration::Mobile),
                (
                    Some(RegisterHook::MobileDownload(_)),
                    EngineBrowserSessionHandle::Mobile(browser),
                    EnginePageSessionHandle::Mobile(page),
                ) => web_lib::register_mobile_hook(browser, page, MobileHookType::Download)
                    .await
                    .map(EngineHookRegistration::Mobile),
                (None, _, _) => Err("register_hook requires a typed hook payload".to_string()),
                _ => Err("hook type is not supported by this context surface".to_string()),
            };

            match registration {
                Ok(registration) => {
                    let hook_id = next_hook_id();
                    state.lock().await.hooks.insert(
                        hook_id.clone(),
                        HookState {
                            surface_session_id: surface_session_id.clone(),
                            context_session_id: context_session_id.clone(),
                            registration,
                        },
                    );
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::HookRegistered(HookRegisteredEvent { hook_id }),
                        )],
                        should_close: false,
                    })
                }
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
        Some(ContextCommand::WaitForHook(WaitForHookCommand {
            hook_id,
            retry_options,
        })) => {
            let hook = state.lock().await.hooks.get(&hook_id).cloned();
            let Some(hook) = hook.filter(|hook| {
                hook.surface_session_id == surface_session_id
                    && hook.context_session_id == context_session_id
            }) else {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: format!("hook {hook_id} is not registered on this page"),
                        }),
                    )],
                    should_close: false,
                });
            };
            if let EngineHookRegistration::Mobile(registration) = &hook.registration {
                let EngineBrowserSessionHandle::Mobile(mobile_session) = &surface_session else {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "mobile hook session is not available".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                };
                let result =
                    retry_with_timeout(command_retry_policy(retry_options.as_ref()), || async {
                        web_lib::poll_mobile_hook(mobile_session, registration).await
                    })
                    .await;
                return match result {
                    Ok(MobileHookResult::FileChooser(chooser)) => {
                        state.lock().await.hooks.remove(&hook_id);
                        Ok(TabCommandOutcome {
                            events: vec![tab_event(
                                &context_session_id,
                                ContextEvent::HookCompleted(HookCompletedEvent {
                                    hook_id,
                                    result: Some(HookCompletionResult::MobileFileChooser(
                                        MobileFileChooserHookResult {
                                            file_chooser_id: chooser.file_chooser_id,
                                            is_multiple: chooser.is_multiple,
                                            note: chooser.note,
                                        },
                                    )),
                                }),
                            )],
                            should_close: false,
                        })
                    }
                    Ok(MobileHookResult::Download(download)) => {
                        state.lock().await.hooks.remove(&hook_id);
                        Ok(TabCommandOutcome {
                            events: vec![tab_event(
                                &context_session_id,
                                ContextEvent::HookCompleted(HookCompletedEvent {
                                    hook_id,
                                    result: Some(HookCompletionResult::MobileDownload(
                                        MobileDownloadHookResult {
                                            download_id: download.download_id,
                                            suggested_filename: download.suggested_filename,
                                            note: download.note,
                                        },
                                    )),
                                }),
                            )],
                            should_close: false,
                        })
                    }
                    Err(message) => Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent { message }),
                        )],
                        should_close: false,
                    }),
                };
            }
            let EngineHookRegistration::Web(registration) = &hook.registration else {
                unreachable!("mobile hook returned above")
            };
            let EngineBrowserSessionHandle::Web(surface_session) = &surface_session else {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "hook browser session is not available".to_string(),
                        }),
                    )],
                    should_close: false,
                });
            };

            let retry_policy = command_retry_policy(retry_options.as_ref());
            match retry_with_timeout(retry_policy, || async {
                web_lib::poll_hook(surface_session, registration).await
            })
            .await
            {
                Ok(HookResult::NewPage(page)) => {
                    let new_context_session_id = next_context_session_id();
                    let note = page.note;
                    let mut state = state.lock().await;
                    state.hooks.remove(&hook_id);
                    state.tab_sessions.insert(
                        new_context_session_id.clone(),
                        TabSessionState {
                            surface_session_id: surface_session_id.clone(),
                            page_session: EnginePageSessionHandle::Web(page.page_session),
                            current_url: None,
                        },
                    );
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::HookCompleted(HookCompletedEvent {
                                hook_id,
                                result: Some(HookCompletionResult::NewPage(NewPageHookResult {
                                    context_session_id: new_context_session_id,
                                    note,
                                })),
                            }),
                        )],
                        should_close: false,
                    })
                }
                Ok(HookResult::FileChooser(file_chooser)) => {
                    state.lock().await.hooks.remove(&hook_id);
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::HookCompleted(HookCompletedEvent {
                                hook_id,
                                result: Some(HookCompletionResult::FileChooser(
                                    FileChooserHookResult {
                                        file_chooser_id: file_chooser.file_chooser_id,
                                        is_multiple: file_chooser.is_multiple,
                                        note: file_chooser.note,
                                    },
                                )),
                            }),
                        )],
                        should_close: false,
                    })
                }
                Ok(HookResult::Dialog(dialog)) => {
                    state.lock().await.hooks.remove(&hook_id);
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::HookCompleted(HookCompletedEvent {
                                hook_id,
                                result: Some(HookCompletionResult::Dialog(
                                    proto::DialogHookResult {
                                        dialog_id: dialog.dialog_id,
                                        r#type: dialog.kind,
                                        message: dialog.message,
                                        default_value: dialog.default_value,
                                    },
                                )),
                            }),
                        )],
                        should_close: false,
                    })
                }
                Ok(HookResult::Download(download)) => {
                    state.lock().await.hooks.remove(&hook_id);
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::HookCompleted(HookCompletedEvent {
                                hook_id,
                                result: Some(HookCompletionResult::Download(DownloadHookResult {
                                    download_id: download.download_id,
                                    url: download.url,
                                    suggested_filename: download.suggested_filename,
                                    note: download.note,
                                })),
                            }),
                        )],
                        should_close: false,
                    })
                }
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
        Some(ContextCommand::HandleDialog(command)) => {
            let result = match (&surface_session, &page_session) {
                (EngineBrowserSessionHandle::Web(browser), EnginePageSessionHandle::Web(page)) => {
                    hook_operation_before_deadline(
                        command.retry_options.as_ref(),
                        "handling dialog",
                        web_lib::handle_dialog(
                            browser,
                            page,
                            command.dialog_id.clone(),
                            command.accept,
                            command.prompt_text,
                        ),
                    )
                    .await
                }
                _ => Err("dialogs require a web page session".into()),
            };
            let event = match result {
                Ok(()) => ContextEvent::DialogHandled(proto::DialogHandledEvent {
                    dialog_id: command.dialog_id,
                }),
                Err(message) => ContextEvent::Error(ContextSessionErrorEvent { message }),
            };
            Ok(TabCommandOutcome {
                events: vec![tab_event(&context_session_id, event)],
                should_close: false,
            })
        }
        Some(ContextCommand::SetFileChooserFiles(SetFileChooserFilesCommand {
            file_chooser_id,
            file_ids,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "file chooser requires a web page session".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let files = {
                let engine = state.lock().await;
                let mut files = Vec::with_capacity(file_ids.len());
                for file_id in &file_ids {
                    let Some(file) = engine
                        .staged_files
                        .get(file_id)
                        .filter(|file| file.context_session_id == context_session_id)
                    else {
                        return Ok(TabCommandOutcome {
                            events: vec![tab_event(
                                &context_session_id,
                                ContextEvent::Error(ContextSessionErrorEvent {
                                    message: format!("unknown staged file {file_id}"),
                                }),
                            )],
                            should_close: false,
                        });
                    };
                    files.push(file.path.to_string_lossy().to_string());
                }
                files
            };
            let result =
                retry_with_timeout(command_retry_policy(retry_options.as_ref()), || async {
                    web_lib::set_file_chooser_files(
                        surface_session,
                        page_session,
                        &file_chooser_id,
                        &files,
                    )
                    .await
                })
                .await;
            match result {
                Ok(result) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::FileChooserFilesSet(FileChooserFilesSetEvent {
                            file_chooser_id: result.file_chooser_id,
                            file_ids,
                            note: result.note,
                        }),
                    )],
                    should_close: false,
                }),
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
        Some(ContextCommand::SaveDownload(SaveDownloadCommand {
            download_id,
            retry_options,
        })) => {
            let (surface_session, page_session) = match (&surface_session, &page_session) {
                (
                    EngineBrowserSessionHandle::Web(surface_session),
                    EnginePageSessionHandle::Web(page_session),
                ) => (surface_session, page_session),
                _ => {
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::Error(ContextSessionErrorEvent {
                                message: "download requires a web page session".to_string(),
                            }),
                        )],
                        should_close: false,
                    });
                }
            };
            let server_file_id = next_file_id("download-file");
            let destination = transfer_path(&server_file_id, "download");
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| Status::internal(error.to_string()))?;
            }
            let destination_string = destination.to_string_lossy().to_string();
            let result =
                retry_with_timeout(command_retry_policy(retry_options.as_ref()), || async {
                    web_lib::save_download(
                        surface_session,
                        page_session,
                        &download_id,
                        &destination_string,
                    )
                    .await
                })
                .await;
            match result {
                Ok(result) => {
                    state.lock().await.staged_files.insert(
                        server_file_id.clone(),
                        StagedFile {
                            context_session_id: context_session_id.clone(),
                            path: PathBuf::from(&result.path),
                            size: result.size,
                        },
                    );
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::DownloadSaved(DownloadSavedEvent {
                                download_id: result.download_id,
                                file_id: server_file_id,
                                suggested_filename: result.suggested_filename,
                                size: result.size,
                                note: result.note,
                            }),
                        )],
                        should_close: false,
                    })
                }
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
        Some(ContextCommand::SetMobileFileChooserFiles(SetMobileFileChooserFilesCommand {
            file_chooser_id,
            file_ids,
            retry_options,
        })) => {
            let (
                EngineBrowserSessionHandle::Mobile(mobile_session),
                EnginePageSessionHandle::Mobile(mobile_page),
            ) = (&surface_session, &page_session)
            else {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "mobile file chooser requires a mobile app context"
                                .to_string(),
                        }),
                    )],
                    should_close: false,
                });
            };
            let files = {
                let engine = state.lock().await;
                let mut files = Vec::with_capacity(file_ids.len());
                for file_id in &file_ids {
                    let Some(file) = engine
                        .staged_files
                        .get(file_id)
                        .filter(|file| file.context_session_id == context_session_id)
                    else {
                        return Ok(TabCommandOutcome {
                            events: vec![tab_event(
                                &context_session_id,
                                ContextEvent::Error(ContextSessionErrorEvent {
                                    message: format!("unknown staged file {file_id}"),
                                }),
                            )],
                            should_close: false,
                        });
                    };
                    files.push(file.path.to_string_lossy().to_string());
                }
                files
            };
            let result =
                retry_with_timeout(command_retry_policy(retry_options.as_ref()), || async {
                    web_lib::set_mobile_file_chooser_files(
                        mobile_session,
                        mobile_page,
                        &file_chooser_id,
                        &files,
                    )
                    .await
                })
                .await;
            {
                let mut engine = state.lock().await;
                for file_id in &file_ids {
                    if let Some(file) = engine.staged_files.remove(file_id) {
                        remove_staged_file(&file);
                    }
                }
            }
            match result {
                Ok(result) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::MobileFileChooserFilesSet(MobileFileChooserFilesSetEvent {
                            file_chooser_id: result.file_chooser_id,
                            file_ids,
                            note: result.note,
                        }),
                    )],
                    should_close: false,
                }),
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
        Some(ContextCommand::SaveMobileDownload(SaveMobileDownloadCommand {
            download_id,
            retry_options,
        })) => {
            let (
                EngineBrowserSessionHandle::Mobile(mobile_session),
                EnginePageSessionHandle::Mobile(mobile_page),
            ) = (&surface_session, &page_session)
            else {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent {
                            message: "mobile download requires a mobile app context".to_string(),
                        }),
                    )],
                    should_close: false,
                });
            };
            let server_file_id = next_file_id("mobile-download-file");
            let destination = transfer_path(&server_file_id, "download");
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| Status::internal(error.to_string()))?;
            }
            let destination_string = destination.to_string_lossy().to_string();
            let result =
                retry_with_timeout(command_retry_policy(retry_options.as_ref()), || async {
                    web_lib::save_mobile_download(
                        mobile_session,
                        mobile_page,
                        &download_id,
                        &destination_string,
                    )
                    .await
                })
                .await;
            match result {
                Ok(result) => {
                    state.lock().await.staged_files.insert(
                        server_file_id.clone(),
                        StagedFile {
                            context_session_id: context_session_id.clone(),
                            path: PathBuf::from(&result.path),
                            size: result.size,
                        },
                    );
                    Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::MobileDownloadSaved(MobileDownloadSavedEvent {
                                download_id: result.download_id,
                                file_id: server_file_id,
                                suggested_filename: result.suggested_filename,
                                size: result.size,
                                note: result.note,
                            }),
                        )],
                        should_close: false,
                    })
                }
                Err(message) => Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &context_session_id,
                        ContextEvent::Error(ContextSessionErrorEvent { message }),
                    )],
                    should_close: false,
                }),
            }
        }
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
                    if !state
                        .lock()
                        .await
                        .frame_sessions
                        .remove(&context_session_id)
                    {
                        web_lib::close_page(surface_session, page_session)
                            .await
                            .map_err(Status::internal)?;
                    }
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
            let mut state = state.lock().await;
            state.tab_sessions.remove(&context_session_id);
            state
                .hooks
                .retain(|_, hook| hook.context_session_id != context_session_id);
            let upload_ids = state
                .file_uploads
                .iter()
                .filter(|(_, upload)| upload.context_session_id == context_session_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in upload_ids {
                if let Some(upload) = state.file_uploads.remove(&id) {
                    let _ = fs::remove_file(upload.path);
                }
            }
            let file_ids = state
                .staged_files
                .iter()
                .filter(|(_, file)| file.context_session_id == context_session_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in file_ids {
                if let Some(file) = state.staged_files.remove(&id) {
                    remove_staged_file(&file);
                }
            }
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
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let count = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::count_elements(surface_session, page_session, &css_selector)
                        .await
                        .map(|count| (count.css_selector, count.count, count.note)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::count_mobile_elements(
                        surface_session,
                        page_session,
                        &css_selector,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|count| (count.selector, count.count, count.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::ElementCounted(ElementCountedEvent {
                        css_selector: count.0,
                        count: count.1,
                        note: count.2,
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
                                message: "highlight is not supported for Android app sessions"
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
                (
                    EngineBrowserSessionHandle::Mobile(surface_session),
                    EnginePageSessionHandle::Mobile(page_session),
                ) => {
                    let retry_policy = command_retry_policy(retry_options.as_ref());
                    let focus = retry_with_timeout(retry_policy, || async {
                        web_lib::focus_mobile_element(
                            surface_session,
                            page_session,
                            &css_selector,
                            retry_options
                                .as_ref()
                                .and_then(|options| options.timeout_ms),
                        )
                        .await
                    })
                    .await
                    .map_err(Status::internal)?;
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::ElementFocused(ElementFocusedEvent {
                                css_selector: focus.selector,
                                note: focus.note,
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
                (
                    EngineBrowserSessionHandle::Mobile(surface_session),
                    EnginePageSessionHandle::Mobile(page_session),
                ) => {
                    let retry_policy = command_retry_policy(retry_options.as_ref());
                    let press = retry_with_timeout(retry_policy, || async {
                        web_lib::press_mobile_key(
                            surface_session,
                            page_session,
                            &css_selector,
                            &key,
                            text.as_deref(),
                            retry_options
                                .as_ref()
                                .and_then(|options| options.timeout_ms),
                        )
                        .await
                    })
                    .await
                    .map_err(Status::internal)?;
                    return Ok(TabCommandOutcome {
                        events: vec![tab_event(
                            &context_session_id,
                            ContextEvent::KeyPressed(KeyPressedEvent {
                                css_selector: press.selector,
                                key: press.key,
                                note: press.note,
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
        Some(ContextCommand::Capture(command)) => {
            let kind = proto::CaptureKind::try_from(command.kind)
                .map_err(|_| Status::invalid_argument("unknown capture kind"))?;
            let kind = match kind {
                proto::CaptureKind::Url => "url",
                proto::CaptureKind::InputValue => "input_value",
                proto::CaptureKind::SelectedOptions => "selected_options",
                proto::CaptureKind::SelectedText => "selected_text",
                proto::CaptureKind::Checked => "checked",
                proto::CaptureKind::Attribute => "attribute",
                proto::CaptureKind::BoundingBox => "bounding_box",
                proto::CaptureKind::Unspecified => {
                    return Err(Status::invalid_argument("capture kind is required"));
                }
            };
            let (EngineBrowserSessionHandle::Web(browser), EnginePageSessionHandle::Web(page)) =
                (&surface_session, &page_session)
            else {
                return Err(Status::invalid_argument("capture requires a web context"));
            };
            let result =
                retry_with_timeout(command_retry_policy(command.retry_options.as_ref()), || {
                    web_lib::capture(
                        browser,
                        page,
                        kind,
                        &command.css_selector,
                        &command.attribute_name,
                    )
                })
                .await
                .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::CaptureResolved(proto::CaptureResolvedEvent {
                        value: result.value,
                        checked: result.checked,
                        selected_options: result
                            .selected_options
                            .into_iter()
                            .map(|o| proto::CapturedOption {
                                value: o.value,
                                label: o.label,
                                index: o.index,
                            })
                            .collect(),
                        bounding_box: result.bounding_box.map(|b| proto::BoundingBox {
                            x: b.x,
                            y: b.y,
                            width: b.width,
                            height: b.height,
                        }),
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::GetTextContent(GetTextContentCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::get_text_content(surface_session, page_session, &css_selector)
                        .await
                        .map(|text| (text.css_selector, text.text, text.note)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::get_mobile_text(
                        surface_session,
                        page_session,
                        &css_selector,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|text| (text.selector, text.text, text.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::TextContentResolved(TextContentResolvedEvent {
                        css_selector: text.0,
                        text: text.1,
                        note: text.2,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::GetInnerText(GetInnerTextCommand {
            css_selector,
            retry_options,
        })) => {
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let text = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::get_inner_text(surface_session, page_session, &css_selector)
                        .await
                        .map(|text| (text.css_selector, text.text, text.note)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::get_mobile_inner_text(
                        surface_session,
                        page_session,
                        &css_selector,
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|text| (text.selector, text.text, text.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::InnerTextResolved(InnerTextResolvedEvent {
                        css_selector: text.0,
                        text: text.1,
                        note: text.2,
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
            let retry_policy = command_retry_policy(retry_options.as_ref());
            let wait = retry_with_timeout(retry_policy, || async {
                match (&surface_session, &page_session) {
                    (
                        EngineBrowserSessionHandle::Web(surface_session),
                        EnginePageSessionHandle::Web(page_session),
                    ) => web_lib::wait_for_selector(
                        surface_session,
                        page_session,
                        &css_selector,
                        visible.unwrap_or(false),
                    )
                    .await
                    .map(|wait| (wait.css_selector, wait.visible, wait.note)),
                    (
                        EngineBrowserSessionHandle::Mobile(surface_session),
                        EnginePageSessionHandle::Mobile(page_session),
                    ) => web_lib::wait_for_mobile_selector(
                        surface_session,
                        page_session,
                        &css_selector,
                        visible.unwrap_or(false),
                        retry_options
                            .as_ref()
                            .and_then(|options| options.timeout_ms),
                    )
                    .await
                    .map(|wait| (wait.selector, wait.visible, wait.note)),
                    _ => Err("tab session backend metadata is inconsistent".to_string()),
                }
            })
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::SelectorWaitSatisfied(SelectorWaitSatisfiedEvent {
                        css_selector: wait.0,
                        visible: wait.1,
                        note: wait.2,
                    }),
                )],
                should_close: false,
            })
        }
        Some(ContextCommand::AccessibilitySnapshot(command)) => {
            let snapshot = retry_with_timeout(
                command_retry_policy(command.retry_options.as_ref()),
                || async {
                    match (&surface_session, &page_session) {
                        (
                            EngineBrowserSessionHandle::Web(surface),
                            EnginePageSessionHandle::Web(page),
                        ) => {
                            web_lib::accessibility_snapshot(
                                surface,
                                page,
                                &command.format,
                                &command.mode,
                            )
                            .await
                        }
                        (
                            EngineBrowserSessionHandle::Mobile(surface),
                            EnginePageSessionHandle::Mobile(page),
                        ) => {
                            web_lib::accessibility_snapshot_mobile(
                                surface,
                                page,
                                &command.format,
                                &command.mode,
                            )
                            .await
                        }
                        _ => Err("inconsistent context backend for accessibility snapshot".into()),
                    }
                },
            )
            .await
            .map_err(Status::internal)?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &context_session_id,
                    ContextEvent::AccessibilitySnapshotCaptured(
                        proto::AccessibilitySnapshotCapturedEvent {
                            snapshot: snapshot.snapshot,
                            format: snapshot.format,
                        },
                    ),
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
            let live_contexts: std::collections::HashSet<_> =
                state.tab_sessions.keys().cloned().collect();
            state.frame_sessions.retain(|id| live_contexts.contains(id));
            state
                .hooks
                .retain(|_, hook| hook.surface_session_id != session_id);
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
                                // Command failures (including stale references) must leave
                                // the context usable for recovery, such as a fresh snapshot.
                                if tx
                                    .send(Ok(tab_event(
                                        &attach_context_session_id,
                                        ContextEvent::Error(ContextSessionErrorEvent {
                                            message: status.message().to_string(),
                                        }),
                                    )))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn transfer_test_state() -> Arc<Mutex<EngineState>> {
        let mut state = EngineState::default();
        state.browser_sessions.insert(
            "surface-1".to_string(),
            BrowserSessionState {
                launched: true,
                surface_session: Some(EngineBrowserSessionHandle::Web(
                    BrowserSessionHandle::Chromium {
                        cdp_websocket_url: "ws://unused".to_string(),
                    },
                )),
                process_id: None,
                automation: None,
            },
        );
        state.tab_sessions.insert(
            "context-1".to_string(),
            TabSessionState {
                surface_session_id: "surface-1".to_string(),
                page_session: EnginePageSessionHandle::Web(PageSessionHandle::Chromium {
                    target_id: "unused".to_string(),
                    browsing_context_id: None,
                    mapper_target_id: None,
                }),
                current_url: None,
            },
        );
        Arc::new(Mutex::new(state))
    }

    #[tokio::test]
    async fn frame_deadline_covers_in_flight_resolution_and_retries() {
        let policy = RetryPolicy {
            timeout: Duration::from_millis(30),
            retry_interval: Duration::from_millis(1),
        };
        let start = Instant::now();
        let result = resolve_frame_before_deadline(policy, || async {
            sleep(Duration::from_secs(5)).await;
            Ok(())
        })
        .await;
        assert!(result.unwrap_err().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(1));
        let mut attempts = 0;
        let result = resolve_frame_before_deadline(policy, || {
            attempts += 1;
            std::future::ready(if attempts == 2 {
                Ok(())
            } else {
                Err("not ready".into())
            })
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn hook_deadlines_bound_single_attempts() {
        let options = CommandRetryOptions {
            timeout_ms: Some(10),
            retry_interval_ms: None,
        };
        let mut attempts = 0;
        let error = hook_operation_before_deadline(Some(&options), "handling dialog", async {
            attempts += 1;
            std::future::pending::<Result<(), String>>().await
        })
        .await
        .unwrap_err();
        assert!(error.contains("timed out handling dialog"));
        assert_eq!(attempts, 1);
        let failure = hook_operation_before_deadline(Some(&options), "handling dialog", async {
            Err::<(), _>("unsupported dialog".to_string())
        })
        .await;
        assert_eq!(failure.unwrap_err(), "unsupported dialog");
        assert_eq!(
            hook_operation_before_deadline(None, "handling dialog", async { Ok(42) })
                .await
                .unwrap(),
            42
        );
    }

    #[tokio::test]
    async fn closing_frame_releases_only_its_session() {
        let state = transfer_test_state();
        {
            let mut state = state.lock().await;
            let parent = state.tab_sessions.get("context-1").unwrap().clone();
            state.tab_sessions.insert("frame-1".into(), parent);
            state.frame_sessions.insert("frame-1".into());
        }
        let outcome = handle_tab_command(
            state.clone(),
            ContextSessionCommand {
                surface_session_id: "surface-1".into(),
                context_session_id: "frame-1".into(),
                command: Some(ContextCommand::Close(CloseContextSessionCommand {})),
            },
        )
        .await
        .unwrap();
        assert!(outcome.should_close);
        let state = state.lock().await;
        assert!(state.tab_sessions.contains_key("context-1"));
        assert!(!state.tab_sessions.contains_key("frame-1"));
        assert!(!state.frame_sessions.contains("frame-1"));
    }

    #[tokio::test]
    async fn staged_files_round_trip_in_chunks_without_server_paths_in_the_contract() {
        let state = transfer_test_state();
        let upload = handle_tab_command(
            Arc::clone(&state),
            ContextSessionCommand {
                surface_session_id: "surface-1".to_string(),
                context_session_id: "context-1".to_string(),
                command: Some(ContextCommand::UploadFileChunk(UploadFileChunkCommand {
                    transfer_id: "transfer-1".to_string(),
                    name: "fixture.txt".to_string(),
                    offset: 0,
                    data: b"client bytes".to_vec(),
                    last: true,
                })),
            },
        )
        .await
        .unwrap();
        let Some(ContextEvent::FileUploaded(uploaded)) = &upload.events[0].event else {
            panic!("expected file uploaded event")
        };
        let file_id = uploaded.file_id.clone();
        assert_ne!(file_id, "fixture.txt");

        let read = handle_tab_command(
            state,
            ContextSessionCommand {
                surface_session_id: "surface-1".to_string(),
                context_session_id: "context-1".to_string(),
                command: Some(ContextCommand::ReadFileChunk(ReadFileChunkCommand {
                    file_id,
                    offset: 0,
                    max_bytes: 64,
                })),
            },
        )
        .await
        .unwrap();
        let Some(ContextEvent::FileChunk(chunk)) = &read.events[0].event else {
            panic!("expected file chunk event")
        };
        assert_eq!(chunk.data, b"client bytes");
        assert!(chunk.last);
    }
}
