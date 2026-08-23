use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex, OnceLock};

use engine_lib::proto::browser_session_command::Command as BrowserCommand;
use engine_lib::proto::browser_session_event::Event as BrowserEvent;
use engine_lib::proto::engine_service_client::EngineServiceClient;
use engine_lib::proto::tab_session_command::Command as TabCommand;
use engine_lib::proto::tab_session_event::Event as TabEvent;
use engine_lib::proto::{
    BrowserSessionCommand, BrowserSessionEvent, ClickElementCommand, CloseBrowserSessionCommand,
    CloseTabSessionCommand, LaunchChromeCommand, NavigateTabCommand, OpenTabCommand, PingRequest,
    SessionPingCommand, TabSessionCommand, TabSessionEvent, TabSessionPingCommand,
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

const DEFAULT_SERVER_ADDR: &str = "http://127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR: &str = "ALLWRIGHT_SERVER_ADDR";

type Result<T> = std::result::Result<T, Error>;

static RUNTIME: OnceLock<Mutex<Option<Arc<RuntimeClient>>>> = OnceLock::new();
static SERVER_ADDR_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self::new(format!("transport error: {value}"))
    }
}

impl From<tonic::Status> for Error {
    fn from(value: tonic::Status) -> Self {
        Self::new(format!("grpc status error: {value}"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub chrome_binary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NavigateResult {
    pub url: String,
    pub note: String,
    pub bidi_session_id: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
    pub package_version: String,
}

#[derive(Debug, Clone)]
pub struct ClickResult {
    pub selector: String,
    pub note: String,
    pub bidi_session_id: String,
}

#[derive(Clone)]
pub struct Browser {
    inner: Arc<BrowserInner>,
}

#[derive(Clone)]
pub struct Tab {
    inner: Arc<TabInner>,
}

#[derive(Clone)]
struct RuntimeClient {
    engine: EngineServiceClient<Channel>,
}

struct BrowserInner {
    runtime: Arc<RuntimeClient>,
    state: AsyncMutex<BrowserState>,
    session_id: String,
    browser_name: String,
    launch_note: String,
    cdp_websocket_url: String,
    user_data_dir: String,
    initial_tab: Tab,
}

struct BrowserState {
    command_tx: mpsc::Sender<BrowserSessionCommand>,
    events: tonic::Streaming<BrowserSessionEvent>,
    closed: bool,
}

struct TabInner {
    runtime: Arc<RuntimeClient>,
    browser_session_id: String,
    session_id: String,
    state: AsyncMutex<TabState>,
}

#[derive(Default)]
struct TabState {
    handle: Option<TabHandle>,
}

struct TabHandle {
    command_tx: mpsc::Sender<TabSessionCommand>,
    events: tonic::Streaming<TabSessionEvent>,
    closed: bool,
}

pub async fn ping() -> Result<String> {
    let runtime = get_runtime().await?;
    let mut engine = runtime.engine.clone();
    let response = engine.ping(tonic::Request::new(PingRequest {})).await?;
    Ok(response.into_inner().message)
}

pub async fn launch_chrome(options: LaunchOptions) -> Result<Browser> {
    let runtime = get_runtime().await?;
    let mut engine = runtime.engine.clone();
    let (command_tx, command_rx) = mpsc::channel(16);
    let response = engine
        .browser_session(tonic::Request::new(ReceiverStream::new(command_rx)))
        .await?;
    let mut events = response.into_inner();

    command_tx
        .send(BrowserSessionCommand {
            command: Some(BrowserCommand::LaunchChrome(LaunchChromeCommand {
                chrome_binary: options.chrome_binary,
            })),
        })
        .await
        .map_err(|_| Error::new("failed to send LaunchChromeCommand to browser session"))?;

    loop {
        let event = events
            .message()
            .await?
            .ok_or_else(|| Error::new("browser session closed before launch response"))?;

        match event.event {
            Some(BrowserEvent::ChromeLaunched(launched)) => {
                let browser_session_id = event.session_id;
                let initial_tab = Tab {
                    inner: Arc::new(TabInner {
                        runtime: Arc::clone(&runtime),
                        browser_session_id: browser_session_id.clone(),
                        session_id: launched.initial_tab_session_id.clone(),
                        state: AsyncMutex::new(TabState::default()),
                    }),
                };
                return Ok(Browser {
                    inner: Arc::new(BrowserInner {
                        runtime,
                        state: AsyncMutex::new(BrowserState {
                            command_tx,
                            events,
                            closed: false,
                        }),
                        session_id: browser_session_id,
                        browser_name: launched.browser,
                        launch_note: launched.note,
                        cdp_websocket_url: launched.cdp_websocket_url,
                        user_data_dir: launched.user_data_dir,
                        initial_tab,
                    }),
                });
            }
            Some(BrowserEvent::Error(error)) => {
                return Err(Error::new(format!(
                    "browser session error during launch: {}",
                    error.message
                )));
            }
            _ => {}
        }
    }
}

pub fn set_server_addr(server_addr: impl Into<String>) -> Result<()> {
    let normalized = normalize_server_addr(&server_addr.into());
    let mut override_slot = server_addr_override_slot()
        .lock()
        .map_err(|_| Error::new("server address override lock is poisoned"))?;
    *override_slot = Some(normalized);
    drop(override_slot);

    let mut runtime = runtime_slot()
        .lock()
        .map_err(|_| Error::new("runtime singleton lock is poisoned"))?;
    *runtime = None;
    Ok(())
}

pub async fn shutdown() {
    if let Ok(mut runtime) = runtime_slot().lock() {
        *runtime = None;
    }
}

impl Browser {
    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    pub fn browser_name(&self) -> &str {
        &self.inner.browser_name
    }

    pub fn launch_note(&self) -> &str {
        &self.inner.launch_note
    }

    pub fn cdp_websocket_url(&self) -> &str {
        &self.inner.cdp_websocket_url
    }

    pub fn user_data_dir(&self) -> &str {
        &self.inner.user_data_dir
    }

    pub fn initial_tab(&self) -> Tab {
        self.inner.initial_tab.clone()
    }

    pub async fn new_tab(&self) -> Result<Tab> {
        let mut state = self.inner.state.lock().await;
        if state.closed {
            return Err(Error::new(format!(
                "browser session {} is closed",
                self.inner.session_id
            )));
        }

        state
            .command_tx
            .send(BrowserSessionCommand {
                command: Some(BrowserCommand::OpenTab(OpenTabCommand {})),
            })
            .await
            .map_err(|_| Error::new("failed to send OpenTabCommand to browser session"))?;

        loop {
            let event =
                state.events.message().await?.ok_or_else(|| {
                    Error::new("browser session closed while waiting for new tab")
                })?;

            match event.event {
                Some(BrowserEvent::TabOpened(opened)) => {
                    return Ok(Tab {
                        inner: Arc::new(TabInner {
                            runtime: Arc::clone(&self.inner.runtime),
                            browser_session_id: self.inner.session_id.clone(),
                            session_id: opened.tab_session_id,
                            state: AsyncMutex::new(TabState::default()),
                        }),
                    });
                }
                Some(BrowserEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "browser session error while opening tab: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn ping(&self, message: impl Into<String>) -> Result<String> {
        let mut state = self.inner.state.lock().await;
        if state.closed {
            return Err(Error::new(format!(
                "browser session {} is closed",
                self.inner.session_id
            )));
        }

        state
            .command_tx
            .send(BrowserSessionCommand {
                command: Some(BrowserCommand::Ping(SessionPingCommand {
                    message: message.into(),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send SessionPingCommand to browser session"))?;

        loop {
            let event = state
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("browser session closed while waiting for pong"))?;

            match event.event {
                Some(BrowserEvent::Pong(pong)) => return Ok(pong.message),
                Some(BrowserEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "browser session error while pinging: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn close(&self) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        if state.closed {
            return Ok(());
        }

        state
            .command_tx
            .send(BrowserSessionCommand {
                command: Some(BrowserCommand::Close(CloseBrowserSessionCommand {})),
            })
            .await
            .map_err(|_| Error::new("failed to send CloseBrowserSessionCommand"))?;

        loop {
            let event =
                state.events.message().await?.ok_or_else(|| {
                    Error::new("browser session closed before close confirmation")
                })?;

            match event.event {
                Some(BrowserEvent::Closed(_)) => {
                    state.closed = true;
                    return Ok(());
                }
                Some(BrowserEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "browser session error while closing: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}

impl Tab {
    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    pub async fn ping(&self, message: impl Into<String>) -> Result<String> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        if handle.closed {
            return Err(Error::new(format!(
                "tab session {} is closed",
                self.inner.session_id
            )));
        }

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::Ping(TabSessionPingCommand {
                    message: message.into(),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send TabSessionPingCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for pong"))?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::Pong(pong)) => return Ok(pong.message),
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while pinging: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for pong",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn navigate(&self, url: impl Into<String>) -> Result<NavigateResult> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        if handle.closed {
            return Err(Error::new(format!(
                "tab session {} is closed",
                self.inner.session_id
            )));
        }

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::Navigate(NavigateTabCommand { url: url.into() })),
            })
            .await
            .map_err(|_| Error::new("failed to send NavigateTabCommand"))?;

        let mut navigated = None;
        let mut injection = None;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for navigation"))?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::Navigated(navigated_event)) => {
                    navigated = Some(navigated_event);
                }
                Some(TabEvent::ChromiumBidiInjection(injection_event)) => {
                    injection = Some(injection_event);
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while navigating: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while navigating",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }

            if let (Some(navigated_event), Some(injection_event)) =
                (navigated.take(), injection.take())
            {
                return Ok(NavigateResult {
                    url: navigated_event.url,
                    note: navigated_event.note,
                    bidi_session_id: injection_event.bidi_session_id,
                    mapper_target_id: injection_event.mapper_target_id,
                    mapper_session_id: injection_event.mapper_session_id,
                    package_version: injection_event.package_version,
                });
            }
        }
    }

    pub async fn click(&self, css_selector: impl Into<String>) -> Result<ClickResult> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        if handle.closed {
            return Err(Error::new(format!(
                "tab session {} is closed",
                self.inner.session_id
            )));
        }

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::ClickElement(ClickElementCommand {
                    css_selector: css_selector.into(),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send ClickElementCommand"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for click result")
                })?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementClicked(clicked)) => {
                    return Ok(ClickResult {
                        selector: clicked.css_selector,
                        note: clicked.note,
                        bidi_session_id: clicked.bidi_session_id,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while clicking: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for click result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn close(&self) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        if handle.closed {
            return Ok(());
        }

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::Close(CloseTabSessionCommand {})),
            })
            .await
            .map_err(|_| Error::new("failed to send CloseTabSessionCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed before close confirmation"))?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Ok(());
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while closing: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }

    async fn ensure_handle<'a>(&self, state: &'a mut TabState) -> Result<&'a mut TabHandle> {
        if state.handle.is_none() {
            let mut engine = self.inner.runtime.engine.clone();
            let (command_tx, command_rx) = mpsc::channel(16);
            let response = engine
                .tab_session(tonic::Request::new(ReceiverStream::new(command_rx)))
                .await?;
            state.handle = Some(TabHandle {
                command_tx,
                events: response.into_inner(),
                closed: false,
            });
        }

        state
            .handle
            .as_mut()
            .ok_or_else(|| Error::new("tab session handle was not initialized"))
    }
}

async fn get_runtime() -> Result<Arc<RuntimeClient>> {
    if let Ok(runtime) = runtime_slot().lock() {
        if let Some(existing) = runtime.as_ref() {
            return Ok(Arc::clone(existing));
        }
    }

    let endpoint = configured_server_addr();
    let engine = EngineServiceClient::connect(endpoint).await?;
    let runtime = Arc::new(RuntimeClient { engine });

    let mut slot = runtime_slot()
        .lock()
        .map_err(|_| Error::new("runtime singleton lock is poisoned"))?;
    if let Some(existing) = slot.as_ref() {
        return Ok(Arc::clone(existing));
    }
    *slot = Some(Arc::clone(&runtime));
    Ok(runtime)
}

fn runtime_slot() -> &'static Mutex<Option<Arc<RuntimeClient>>> {
    RUNTIME.get_or_init(|| Mutex::new(None))
}

fn server_addr_override_slot() -> &'static Mutex<Option<String>> {
    SERVER_ADDR_OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn configured_server_addr() -> String {
    if let Ok(server_addr_override) = server_addr_override_slot().lock() {
        if let Some(server_addr) = server_addr_override.as_ref() {
            return server_addr.clone();
        }
    }

    normalize_server_addr(
        std::env::var(SERVER_ADDR_ENV_VAR)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .as_deref()
            .unwrap_or(DEFAULT_SERVER_ADDR),
    )
}

fn normalize_server_addr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}
