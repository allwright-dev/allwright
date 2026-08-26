use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::proto::browser_session_command::Command as BrowserCommand;
use crate::proto::browser_session_event::Event as BrowserEvent;
use crate::proto::engine_service_client::EngineServiceClient;
use crate::proto::tab_session_command::Command as TabCommand;
use crate::proto::tab_session_event::Event as TabEvent;
use crate::proto::{
    BrowserKind as ProtoBrowserKind, BrowserLaunchedEvent, BrowserSessionCommand,
    BrowserSessionEvent, ClickElementCommand, CloseBrowserSessionCommand, CloseTabSessionCommand,
    CommandRetryOptions, CountElementsCommand, ElementCountedEvent, ElementsHighlightedEvent,
    FillElementCommand, FocusElementCommand, GetInnerTextCommand, GetTextContentCommand,
    HighlightElementsCommand, HoverElementCommand, LaunchBrowserCommand, NavigateTabCommand,
    OpenTabCommand, PingRequest, PressKeyCommand, SessionPingCommand, TabSessionCommand,
    TabSessionEvent, TabSessionPingCommand, WaitForSelectorCommand,
};
use serde::Deserialize;
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

const DEFAULT_SERVER_ADDR: &str = "http://127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR: &str = "ALLWRIGHT_SERVER_ADDR";
const CONFIG_FILENAMES: [&str; 6] = [
    "allwright.config.yaml",
    "allwright.config.yml",
    "allwright.config.json",
    ".allwright/config.yaml",
    ".allwright/config.yml",
    ".allwright/config.json",
];

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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    pub browser_binary: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    pub timeout_ms: Option<u32>,
    pub interval_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chromium,
    Firefox,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigServer {
    addr: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigBrowser {
    name: Option<BrowserKind>,
    binary: Option<String>,
    launch_options: Option<LaunchOptions>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuiteConfig {
    server: Option<ConfigServer>,
    browser: Option<ConfigBrowser>,
    expect: Option<RetryConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllwrightConfig {
    schema_version: Option<u32>,
    server: Option<ConfigServer>,
    browser: Option<ConfigBrowser>,
    expect: Option<RetryConfig>,
    suites: Option<std::collections::BTreeMap<String, SuiteConfig>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub config_file_path: Option<PathBuf>,
    pub suite_name: Option<String>,
    pub server_addr: Option<String>,
    pub browser_name: BrowserKind,
    pub browser_binary: Option<String>,
    pub launch_options: LaunchOptions,
    pub expect: RetryConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ResolveConfigOptions {
    pub cwd: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub suite: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserType {
    browser_kind: BrowserKind,
}

#[derive(Debug, Clone, Default)]
pub struct CommandOptions {
    pub timeout_ms: Option<u32>,
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

#[derive(Debug, Clone)]
pub struct CountResult {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct HighlightOptions {
    pub timeout_ms: Option<u32>,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HighlightResult {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ElementResult {
    pub selector: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct FillResult {
    pub selector: String,
    pub value: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct PressResult {
    pub selector: String,
    pub key: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct TextResult {
    pub selector: String,
    pub text: String,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct PressOptions {
    pub timeout_ms: Option<u32>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WaitForSelectorOptions {
    pub timeout_ms: Option<u32>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct WaitForSelectorResult {
    pub selector: String,
    pub visible: bool,
    pub note: String,
}

#[derive(Clone)]
pub struct Browser {
    inner: Arc<BrowserInner>,
}

#[derive(Clone)]
pub struct Tab {
    inner: Arc<TabInner>,
}

pub type Page = Tab;

#[derive(Clone)]
pub struct Locator {
    page: Tab,
    selector: String,
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
    launch_browser(BrowserKind::Chromium, options).await
}

pub async fn launch_firefox(options: LaunchOptions) -> Result<Browser> {
    launch_browser(BrowserKind::Firefox, options).await
}

pub async fn launch_browser(browser_kind: BrowserKind, options: LaunchOptions) -> Result<Browser> {
    let runtime = get_runtime().await?;
    let mut engine = runtime.engine.clone();
    let (command_tx, command_rx) = mpsc::channel(16);
    let response = engine
        .browser_session(tonic::Request::new(ReceiverStream::new(command_rx)))
        .await?;
    let mut events = response.into_inner();

    command_tx
        .send(BrowserSessionCommand {
            command: Some(BrowserCommand::LaunchBrowser(LaunchBrowserCommand {
                browser_kind: match browser_kind {
                    BrowserKind::Chromium => ProtoBrowserKind::Chromium as i32,
                    BrowserKind::Firefox => ProtoBrowserKind::Firefox as i32,
                },
                browser_binary: options.browser_binary,
                retry_options: command_retry_options(options.timeout_ms),
            })),
        })
        .await
        .map_err(|_| Error::new("failed to send LaunchBrowserCommand to browser session"))?;

    loop {
        let event = events
            .message()
            .await?
            .ok_or_else(|| Error::new("browser session closed before launch response"))?;

        match event.event {
            Some(BrowserEvent::BrowserLaunched(BrowserLaunchedEvent {
                browser,
                note,
                user_data_dir,
                initial_tab_session_id,
                ..
            })) => {
                let browser_session_id = event.session_id;
                let initial_tab = Tab {
                    inner: Arc::new(TabInner {
                        runtime: Arc::clone(&runtime),
                        browser_session_id: browser_session_id.clone(),
                        session_id: initial_tab_session_id,
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
                        browser_name: browser,
                        launch_note: note,
                        cdp_websocket_url: String::new(),
                        user_data_dir,
                        initial_tab,
                    }),
                });
            }
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

pub fn chromium() -> BrowserType {
    BrowserType {
        browser_kind: BrowserKind::Chromium,
    }
}

pub fn firefox() -> BrowserType {
    BrowserType {
        browser_kind: BrowserKind::Firefox,
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

pub fn find_config_file(start_dir: impl AsRef<Path>) -> Option<PathBuf> {
    let mut current_dir = start_dir.as_ref().to_path_buf();

    loop {
        for filename in CONFIG_FILENAMES {
            let candidate = current_dir.join(filename);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if !current_dir.pop() {
            return None;
        }
    }
}

pub fn load_config_file(config_file: impl AsRef<Path>) -> Result<AllwrightConfig> {
    let resolved = config_file.as_ref().to_path_buf();
    let raw = fs::read_to_string(&resolved).map_err(|error| {
        Error::new(format!(
            "failed to read allwright config {}: {error}",
            resolved.display()
        ))
    })?;

    let extension = resolved
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let config = match extension.as_str() {
        "json" => serde_json::from_str::<AllwrightConfig>(&raw).map_err(|error| {
            Error::new(format!(
                "failed to parse allwright config {} as JSON: {error}",
                resolved.display()
            ))
        })?,
        "yaml" | "yml" => serde_yaml::from_str::<AllwrightConfig>(&raw).map_err(|error| {
            Error::new(format!(
                "failed to parse allwright config {} as YAML: {error}",
                resolved.display()
            ))
        })?,
        _ => {
            return Err(Error::new(format!(
                "unsupported allwright config file extension .{} for {}",
                if extension.is_empty() {
                    "<none>"
                } else {
                    &extension
                },
                resolved.display()
            )));
        }
    };

    validate_config_shape(&config, &resolved)?;
    Ok(config)
}

pub fn resolve_config(options: ResolveConfigOptions) -> Result<ResolvedConfig> {
    let cwd = options
        .cwd
        .unwrap_or(std::env::current_dir().map_err(|error| {
            Error::new(format!(
                "failed to determine current working directory: {error}"
            ))
        })?);
    let config_file_path = match options.config_file {
        Some(path) => Some(path),
        None => find_config_file(cwd),
    };
    let file_config = match &config_file_path {
        Some(path) => load_config_file(path)?,
        None => AllwrightConfig::default(),
    };
    let suite_name = options.suite.and_then(|suite| {
        let trimmed = suite.trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let suite_config = match &suite_name {
        Some(name) => {
            let suite = file_config
                .suites
                .as_ref()
                .and_then(|suites| suites.get(name))
                .cloned();
            if suite.is_none() {
                return Err(Error::new(format!(
                    "allwright config suite \"{}\" was not found in {}",
                    name,
                    config_file_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "the resolved config file".to_string())
                )));
            }
            suite
        }
        None => None,
    };

    let server_addr = suite_config
        .as_ref()
        .and_then(|suite| suite.server.as_ref())
        .and_then(|server| server.addr.clone())
        .or_else(|| {
            file_config
                .server
                .as_ref()
                .and_then(|server| server.addr.clone())
        });
    let browser_name = suite_config
        .as_ref()
        .and_then(|suite| suite.browser.as_ref())
        .and_then(|browser| browser.name)
        .or_else(|| {
            file_config
                .browser
                .as_ref()
                .and_then(|browser| browser.name)
        })
        .unwrap_or(BrowserKind::Chromium);
    let browser_binary = suite_config
        .as_ref()
        .and_then(|suite| suite.browser.as_ref())
        .and_then(|browser| browser.binary.clone())
        .or_else(|| {
            file_config
                .browser
                .as_ref()
                .and_then(|browser| browser.binary.clone())
        });
    let mut launch_options = merge_launch_options(
        file_config
            .browser
            .as_ref()
            .and_then(|browser| browser.launch_options.clone()),
        suite_config
            .as_ref()
            .and_then(|suite| suite.browser.as_ref())
            .and_then(|browser| browser.launch_options.clone()),
    );
    if let Some(binary) = &browser_binary {
        launch_options.browser_binary = Some(binary.clone());
    }
    let expect = merge_retry_config(
        file_config.expect.clone(),
        suite_config.and_then(|suite| suite.expect),
    );

    Ok(ResolvedConfig {
        config_file_path,
        suite_name,
        server_addr,
        browser_name,
        browser_binary,
        launch_options,
        expect,
    })
}

pub async fn launch_configured_browser(config: &ResolvedConfig) -> Result<Browser> {
    if let Some(server_addr) = &config.server_addr {
        set_server_addr(server_addr.clone())?;
    }
    launch_browser(config.browser_name, config.launch_options.clone()).await
}

impl Browser {
    pub fn page(&self) -> Page {
        self.initial_tab()
    }

    pub fn initial_page(&self) -> Page {
        self.initial_tab()
    }

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
        self.new_tab_with_options(CommandOptions::default()).await
    }

    pub async fn new_page(&self) -> Result<Page> {
        self.new_tab().await
    }

    pub async fn new_tab_with_options(&self, options: CommandOptions) -> Result<Tab> {
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
                command: Some(BrowserCommand::OpenTab(OpenTabCommand {
                    retry_options: command_retry_options(options.timeout_ms),
                })),
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
    pub fn locator(&self, css_selector: impl Into<String>) -> Locator {
        Locator {
            page: self.clone(),
            selector: css_selector.into(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    pub async fn goto(&self, url: impl Into<String>) -> Result<NavigateResult> {
        self.navigate(url).await
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
        self.navigate_with_options(url, CommandOptions::default())
            .await
    }

    pub async fn navigate_with_options(
        &self,
        url: impl Into<String>,
        options: CommandOptions,
    ) -> Result<NavigateResult> {
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
                command: Some(TabCommand::Navigate(NavigateTabCommand {
                    url: url.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
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
        self.click_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn click_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ClickResult> {
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
                    retry_options: command_retry_options(options.timeout_ms),
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

    pub async fn count(&self, css_selector: impl Into<String>) -> Result<CountResult> {
        self.count_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn count_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<CountResult> {
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
                command: Some(TabCommand::CountElements(CountElementsCommand {
                    css_selector: css_selector.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send CountElementsCommand"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for count result")
                })?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementCounted(counted)) => {
                    return Ok(count_result_from_event(counted));
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while counting elements: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for count result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn highlight(&self, css_selector: impl Into<String>) -> Result<HighlightResult> {
        self.highlight_with_options(css_selector, HighlightOptions::default())
            .await
    }

    pub async fn highlight_with_options(
        &self,
        css_selector: impl Into<String>,
        options: HighlightOptions,
    ) -> Result<HighlightResult> {
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
                command: Some(TabCommand::HighlightElements(HighlightElementsCommand {
                    css_selector: css_selector.into(),
                    duration_ms: options.duration_ms,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send HighlightElementsCommand"))?;

        loop {
            let event = handle.events.message().await?.ok_or_else(|| {
                Error::new("tab session closed while waiting for highlight result")
            })?;

            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementsHighlighted(highlighted)) => {
                    return Ok(highlight_result_from_event(highlighted));
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while highlighting elements: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for highlight result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn focus(&self, css_selector: impl Into<String>) -> Result<ElementResult> {
        self.focus_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn focus_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ElementResult> {
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
                command: Some(TabCommand::FocusElement(FocusElementCommand {
                    css_selector: css_selector.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send FocusElementCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for focus result")
                })?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementFocused(focused)) => {
                    return Ok(ElementResult {
                        selector: focused.css_selector,
                        note: focused.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while focusing: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for focus result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn fill(
        &self,
        css_selector: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<FillResult> {
        self.fill_with_options(css_selector, value, CommandOptions::default())
            .await
    }

    pub async fn fill_with_options(
        &self,
        css_selector: impl Into<String>,
        value: impl Into<String>,
        options: CommandOptions,
    ) -> Result<FillResult> {
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
                command: Some(TabCommand::FillElement(FillElementCommand {
                    css_selector: css_selector.into(),
                    value: value.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send FillElementCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for fill result")
                })?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementFilled(filled)) => {
                    return Ok(FillResult {
                        selector: filled.css_selector,
                        value: filled.value,
                        note: filled.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while filling: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for fill result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn hover(&self, css_selector: impl Into<String>) -> Result<ElementResult> {
        self.hover_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn hover_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ElementResult> {
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
                command: Some(TabCommand::HoverElement(HoverElementCommand {
                    css_selector: css_selector.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send HoverElementCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for hover result")
                })?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::ElementHovered(hovered)) => {
                    return Ok(ElementResult {
                        selector: hovered.css_selector,
                        note: hovered.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while hovering: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for hover result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn press(
        &self,
        css_selector: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<PressResult> {
        self.press_with_options(css_selector, key, PressOptions::default())
            .await
    }

    pub async fn press_with_options(
        &self,
        css_selector: impl Into<String>,
        key: impl Into<String>,
        options: PressOptions,
    ) -> Result<PressResult> {
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
                command: Some(TabCommand::PressKey(PressKeyCommand {
                    css_selector: css_selector.into(),
                    key: key.into(),
                    text: options.text,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send PressKeyCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for press result")
                })?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::KeyPressed(pressed)) => {
                    return Ok(PressResult {
                        selector: pressed.css_selector,
                        key: pressed.key,
                        note: pressed.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while pressing key: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for press result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn text_content(&self, css_selector: impl Into<String>) -> Result<TextResult> {
        self.text_content_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn text_content_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<TextResult> {
        self.read_text(css_selector.into(), options, true).await
    }

    pub async fn inner_text(&self, css_selector: impl Into<String>) -> Result<TextResult> {
        self.inner_text_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn inner_text_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<TextResult> {
        self.read_text(css_selector.into(), options, false).await
    }

    pub async fn wait_for_selector(
        &self,
        css_selector: impl Into<String>,
    ) -> Result<WaitForSelectorResult> {
        self.wait_for_selector_with_options(css_selector, WaitForSelectorOptions::default())
            .await
    }

    pub async fn wait_for_selector_with_options(
        &self,
        css_selector: impl Into<String>,
        options: WaitForSelectorOptions,
    ) -> Result<WaitForSelectorResult> {
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
                command: Some(TabCommand::WaitForSelector(WaitForSelectorCommand {
                    css_selector: css_selector.into(),
                    visible: options.visible,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send WaitForSelectorCommand"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for selector"))?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::SelectorWaitSatisfied(waited)) => {
                    return Ok(WaitForSelectorResult {
                        selector: waited.css_selector,
                        visible: waited.visible,
                        note: waited.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while waiting for selector: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for selector result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    async fn read_text(
        &self,
        css_selector: String,
        options: CommandOptions,
        text_content: bool,
    ) -> Result<TextResult> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        if handle.closed {
            return Err(Error::new(format!(
                "tab session {} is closed",
                self.inner.session_id
            )));
        }
        let command = if text_content {
            TabCommand::GetTextContent(GetTextContentCommand {
                css_selector,
                retry_options: command_retry_options(options.timeout_ms),
            })
        } else {
            TabCommand::GetInnerText(GetInnerTextCommand {
                css_selector,
                retry_options: command_retry_options(options.timeout_ms),
            })
        };
        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(command),
            })
            .await
            .map_err(|_| Error::new("failed to send text command"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for text result")
                })?;
            match event.event {
                Some(TabEvent::Attached(_)) => {}
                Some(TabEvent::TextContentResolved(text)) => {
                    return Ok(TextResult {
                        selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    });
                }
                Some(TabEvent::InnerTextResolved(text)) => {
                    return Ok(TextResult {
                        selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    });
                }
                Some(TabEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while reading text: {}",
                        error.message
                    )));
                }
                Some(TabEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "tab session {} closed while waiting for text result",
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

impl BrowserType {
    pub async fn launch(&self, options: LaunchOptions) -> Result<Browser> {
        launch_browser(self.browser_kind, options).await
    }
}

impl Locator {
    pub fn page(&self) -> &Page {
        &self.page
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn locator(&self, css_selector: impl Into<String>) -> Locator {
        Locator {
            page: self.page.clone(),
            selector: format!("{} {}", self.selector, css_selector.into()),
        }
    }

    pub async fn click(&self) -> Result<ClickResult> {
        self.page.click(self.selector.clone()).await
    }

    pub async fn count(&self) -> Result<CountResult> {
        self.page.count(self.selector.clone()).await
    }

    pub async fn highlight(&self) -> Result<HighlightResult> {
        self.page.highlight(self.selector.clone()).await
    }

    pub async fn focus(&self) -> Result<ElementResult> {
        self.page.focus(self.selector.clone()).await
    }

    pub async fn fill(&self, value: impl Into<String>) -> Result<FillResult> {
        self.page.fill(self.selector.clone(), value.into()).await
    }

    pub async fn hover(&self) -> Result<ElementResult> {
        self.page.hover(self.selector.clone()).await
    }

    pub async fn press(&self, key: impl Into<String>) -> Result<PressResult> {
        self.page.press(self.selector.clone(), key.into()).await
    }

    pub async fn text_content(&self) -> Result<TextResult> {
        self.page.text_content(self.selector.clone()).await
    }

    pub async fn inner_text(&self) -> Result<TextResult> {
        self.page.inner_text(self.selector.clone()).await
    }

    pub async fn wait_for(&self) -> Result<WaitForSelectorResult> {
        self.page.wait_for_selector(self.selector.clone()).await
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

fn merge_launch_options(
    base: Option<LaunchOptions>,
    override_options: Option<LaunchOptions>,
) -> LaunchOptions {
    let mut merged = base.unwrap_or_default();
    if let Some(override_options) = override_options {
        if override_options.browser_binary.is_some() {
            merged.browser_binary = override_options.browser_binary;
        }
        if override_options.timeout_ms.is_some() {
            merged.timeout_ms = override_options.timeout_ms;
        }
    }
    merged
}

fn merge_retry_config(
    base: Option<RetryConfig>,
    override_config: Option<RetryConfig>,
) -> RetryConfig {
    let mut merged = base.unwrap_or_default();
    if let Some(override_config) = override_config {
        if override_config.timeout_ms.is_some() {
            merged.timeout_ms = override_config.timeout_ms;
        }
        if override_config.interval_ms.is_some() {
            merged.interval_ms = override_config.interval_ms;
        }
    }
    merged
}

fn validate_config_shape(config: &AllwrightConfig, source: &Path) -> Result<()> {
    if let Some(schema_version) = config.schema_version {
        if schema_version != 1 {
            return Err(Error::new(format!(
                "allwright config {} has unsupported schemaVersion {}; expected 1",
                source.display(),
                schema_version
            )));
        }
    }
    Ok(())
}

fn normalize_server_addr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

fn command_retry_options(timeout_ms: Option<u32>) -> Option<CommandRetryOptions> {
    timeout_ms.map(|timeout_ms| CommandRetryOptions {
        timeout_ms: Some(timeout_ms),
        retry_interval_ms: None,
    })
}

fn count_result_from_event(event: ElementCountedEvent) -> CountResult {
    CountResult {
        selector: event.css_selector,
        count: event.count,
        note: event.note,
    }
}

fn highlight_result_from_event(event: ElementsHighlightedEvent) -> HighlightResult {
    HighlightResult {
        selector: event.css_selector,
        count: event.count,
        note: event.note,
    }
}
