use std::sync::{Arc, Mutex};

use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::surface_session_command::Command as SurfaceCommand;
use crate::proto::surface_session_event::Event as SurfaceEvent;
use crate::proto::{
    AppLaunchedEvent, ClickElementCommand, ConnectMobileCommand, ContextSessionCommand,
    FillElementCommand, LaunchAppCommand, MobileConnectedEvent, MobilePlatform as ProtoMobilePlatform,
    SurfaceSessionCommand,
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use super::command::command_retry_options;
use super::runtime::get_runtime;
use super::types::{ClickResult, CommandOptions, Error, FillResult, Result, RuntimeClient};

#[derive(Debug, Clone, Default)]
pub struct MobileAndroidConnectOptions {
    pub device: Option<String>,
    pub adb_endpoint: Option<String>,
    pub preserve_app_state: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct MobileAndroidLaunchOptions {
    pub apk_path: Option<String>,
    pub app_id: Option<String>,
    pub launch_activity: Option<String>,
    pub stop_before_launch: bool,
    pub timeout_ms: Option<u32>,
}

#[derive(Clone)]
pub struct AndroidLocator {
    page: AndroidApp,
    selector: String,
}

#[derive(Clone)]
pub struct AndroidApp {
    inner: Arc<AndroidAppInner>,
}

#[derive(Clone)]
pub struct AndroidDevice {
    inner: Arc<AndroidDeviceInner>,
}

struct AndroidDeviceInner {
    runtime: Arc<RuntimeClient>,
    state: AsyncMutex<AndroidDeviceState>,
    session_id: String,
    initial_app: AndroidApp,
    current_app: Mutex<AndroidApp>,
}

struct AndroidDeviceState {
    command_tx: mpsc::Sender<SurfaceSessionCommand>,
    events: tonic::Streaming<crate::proto::SurfaceSessionEvent>,
    closed: bool,
}

struct AndroidAppInner {
    runtime: Arc<RuntimeClient>,
    surface_session_id: String,
    session_id: String,
    state: AsyncMutex<AndroidAppState>,
}

#[derive(Default)]
struct AndroidAppState {
    handle: Option<AndroidTabHandle>,
}

struct AndroidTabHandle {
    command_tx: mpsc::Sender<crate::proto::ContextSessionCommand>,
    events: tonic::Streaming<crate::proto::ContextSessionEvent>,
    closed: bool,
}

pub mod android {
    use super::*;

    pub async fn connect(options: MobileAndroidConnectOptions) -> Result<AndroidDevice> {
        let runtime = get_runtime().await?;
        let mut engine = runtime.engine.clone();
        let (command_tx, command_rx) = mpsc::channel(16);
        let response = engine
            .surface_session(tonic::Request::new(ReceiverStream::new(command_rx)))
            .await?;
        let mut events = response.into_inner();

        command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::ConnectMobile(ConnectMobileCommand {
                    platform: ProtoMobilePlatform::Android as i32,
                    device: options.device,
                    adb_endpoint: options.adb_endpoint,
                    preserve_app_state: options.preserve_app_state,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send ConnectMobileCommand"))?;

        loop {
            let event = events
                .message()
                .await?
                .ok_or_else(|| Error::new("surface session closed before mobile connect response"))?;

            match event.event {
                Some(SurfaceEvent::MobileConnected(MobileConnectedEvent {
                    initial_app_session_id,
                    device_session_id,
                    ..
                })) => {
                    let initial_app = AndroidApp {
                        inner: Arc::new(AndroidAppInner {
                            runtime: Arc::clone(&runtime),
                            surface_session_id: event.session_id.clone(),
                            session_id: initial_app_session_id,
                            state: AsyncMutex::new(AndroidAppState::default()),
                        }),
                    };
                    return Ok(AndroidDevice {
                        inner: Arc::new(AndroidDeviceInner {
                            runtime,
                            state: AsyncMutex::new(AndroidDeviceState {
                                command_tx,
                                events,
                                closed: false,
                            }),
                            session_id: if device_session_id.is_empty() {
                                event.session_id
                            } else {
                                device_session_id
                            },
                            initial_app: initial_app.clone(),
                            current_app: Mutex::new(initial_app),
                        }),
                    });
                }
                Some(SurfaceEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "surface session error during mobile connect: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}

impl AndroidDevice {
    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    pub fn app(&self) -> AndroidApp {
        self.inner
            .current_app
            .lock()
            .map(|app| app.clone())
            .unwrap_or_else(|_| self.inner.initial_app.clone())
    }

    pub fn initial_app(&self) -> AndroidApp {
        self.inner.initial_app.clone()
    }

    pub async fn launch(&self, options: MobileAndroidLaunchOptions) -> Result<AndroidApp> {
        let mut state = self.inner.state.lock().await;
        ensure_android_device_open(&state, &self.inner.session_id)?;

        state
            .command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::LaunchApp(LaunchAppCommand {
                    apk_path: options.apk_path,
                    app_id: options.app_id,
                    launch_activity: options.launch_activity,
                    stop_before_launch: options.stop_before_launch,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send LaunchAppCommand"))?;

        loop {
            let event = state
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("surface session closed before app launch response"))?;

            match event.event {
                Some(SurfaceEvent::AppLaunched(AppLaunchedEvent {
                    app_session_id, ..
                })) => {
                    let app = AndroidApp {
                        inner: Arc::new(AndroidAppInner {
                            runtime: Arc::clone(&self.inner.runtime),
                            surface_session_id: event.session_id,
                            session_id: app_session_id,
                            state: AsyncMutex::new(AndroidAppState::default()),
                        }),
                    };
                    if let Ok(mut current_app) = self.inner.current_app.lock() {
                        *current_app = app.clone();
                    }
                    return Ok(app);
                }
                Some(SurfaceEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "surface session error while launching Android app: {}",
                        error.message
                    )));
                }
                Some(SurfaceEvent::Closed(_)) => {
                    state.closed = true;
                    return Err(Error::new(
                        "surface session closed while waiting for Android app launch",
                    ));
                }
                _ => {}
            }
        }
    }
}

impl AndroidApp {
    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    pub fn locator(&self, selector: impl Into<String>) -> AndroidLocator {
        AndroidLocator {
            page: self.clone(),
            selector: normalize_mobile_selector_for_transport(&selector.into()),
        }
    }

    pub async fn click(&self, selector: &str, options: CommandOptions) -> Result<ClickResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::ClickElement(ClickElementCommand {
                    css_selector: selector.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send ClickElementCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("app session closed while waiting for click result"))?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementClicked(clicked)) => {
                    return Ok(ClickResult {
                        selector: clicked.css_selector,
                        note: clicked.note,
                        bidi_session_id: clicked.bidi_session_id,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while clicking Android locator {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for click result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn fill(
        &self,
        selector: &str,
        value: &str,
        options: CommandOptions,
    ) -> Result<FillResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::FillElement(FillElementCommand {
                    css_selector: selector.clone(),
                    value: value.to_string(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send FillElementCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("app session closed while waiting for fill result"))?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementFilled(filled)) => {
                    return Ok(FillResult {
                        selector: filled.css_selector,
                        value: filled.value,
                        note: filled.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while filling Android locator {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for fill result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    async fn ensure_handle<'a>(
        &self,
        state: &'a mut AndroidAppState,
    ) -> Result<&'a mut AndroidTabHandle> {
        if state.handle.is_none() {
            let mut engine = self.inner.runtime.engine.clone();
            let (command_tx, command_rx) = mpsc::channel(16);
            let response = engine
                .context_session(tonic::Request::new(ReceiverStream::new(command_rx)))
                .await?;
            state.handle = Some(AndroidTabHandle {
                command_tx,
                events: response.into_inner(),
                closed: false,
            });
        }

        state
            .handle
            .as_mut()
            .ok_or_else(|| Error::new("android app session handle was not initialized"))
    }
}

impl AndroidLocator {
    pub fn app(&self) -> &AndroidApp {
        &self.page
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn locator(&self, selector: impl Into<String>) -> AndroidLocator {
        AndroidLocator {
            page: self.page.clone(),
            selector: chain_mobile_selector_for_transport(&self.selector, &selector.into()),
        }
    }

    pub async fn click(&self, options: CommandOptions) -> Result<ClickResult> {
        self.page.click(&self.selector, options).await
    }

    pub async fn fill(&self, value: &str, options: CommandOptions) -> Result<FillResult> {
        self.page.fill(&self.selector, value, options).await
    }
}

fn ensure_android_device_open(state: &AndroidDeviceState, session_id: &str) -> Result<()> {
    if state.closed {
        return Err(Error::new(format!(
            "android device session {} is closed",
            session_id
        )));
    }
    Ok(())
}

fn ensure_android_app_open(handle: &AndroidTabHandle, session_id: &str) -> Result<()> {
    if handle.closed {
        return Err(Error::new(format!(
            "android app session {} is closed",
            session_id
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MobileSelectorFlavor {
    Css,
    XPath,
    UiAutomator,
}

impl MobileSelectorFlavor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::XPath => "xpath",
            Self::UiAutomator => "uia",
        }
    }
}

const UIAUTOMATOR_SELECTOR_KEYS: &[&str] = &[
    "text",
    "textcontains",
    "textmatches",
    "textstartswith",
    "classname",
    "classnamematches",
    "description",
    "desc",
    "descriptioncontains",
    "desccontains",
    "descriptionmatches",
    "descmatches",
    "descriptionstartswith",
    "descstartswith",
    "checkable",
    "checked",
    "clickable",
    "longclickable",
    "scrollable",
    "enabled",
    "focusable",
    "focused",
    "selected",
    "packagename",
    "package",
    "packagenamematches",
    "resourceid",
    "resourceidmatches",
    "index",
    "instance",
];

fn parse_explicit_mobile_selector_prefix(selector: &str) -> Option<(MobileSelectorFlavor, usize)> {
    let lowered = selector.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return Some((MobileSelectorFlavor::XPath, 6));
    }
    if lowered.starts_with("uia=") || lowered.starts_with("uia:") {
        return Some((MobileSelectorFlavor::UiAutomator, 4));
    }
    if let Some(prefix_len) = parse_ui_automator_selector_prefix(&lowered) {
        return Some((MobileSelectorFlavor::UiAutomator, prefix_len));
    }
    if lowered.starts_with("text=") || lowered.starts_with("text:") {
        return Some((MobileSelectorFlavor::UiAutomator, 5));
    }
    if lowered.starts_with("id=") || lowered.starts_with("id:") {
        return Some((MobileSelectorFlavor::Css, 3));
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return Some((MobileSelectorFlavor::Css, 4));
    }
    None
}

fn parse_ui_automator_selector_prefix(selector: &str) -> Option<usize> {
    UIAUTOMATOR_SELECTOR_KEYS.iter().find_map(|key| {
        if selector.starts_with(key) {
            let separator = selector.as_bytes().get(key.len()).copied()?;
            if separator == b'=' || separator == b':' {
                return Some(key.len() + 1);
            }
        }
        None
    })
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

fn is_normalized_mobile_transport_selector(selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut index = 0usize;
    while index < trimmed.len() {
        let Some((_, prefix_len)) = parse_explicit_mobile_selector_prefix(&trimmed[index..]) else {
            return false;
        };
        index += prefix_len;

        let remainder = &trimmed[index..];
        let Some(json_end) = find_json_string_end(remainder) else {
            return false;
        };
        index += json_end;

        if index == trimmed.len() {
            return true;
        }

        let whitespace_len = trimmed[index..]
            .chars()
            .take_while(|char| char.is_ascii_whitespace())
            .count();
        if whitespace_len == 0 {
            return false;
        }
        index += whitespace_len;

        if parse_explicit_mobile_selector_prefix(&trimmed[index..]).is_none() {
            return false;
        }
    }

    true
}

fn decode_selector_body(body: &str) -> String {
    let candidate = body.trim();
    if candidate.len() >= 2 && candidate.starts_with('"') && candidate.ends_with('"') {
        if let Ok(decoded) = serde_json::from_str::<String>(candidate) {
            return unescape_shell_escaped_selector(&decoded);
        }
    }
    unescape_shell_escaped_selector(candidate)
}

fn unescape_shell_escaped_selector(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('_' | ' ' | '#' | ':' | '[' | ']' | '(' | ')' | '"' | '\'') => {
                    result.push(chars.next().expect("peeked char should exist"));
                    continue;
                }
                _ => {}
            }
        }
        result.push(ch);
    }
    result
}

fn parse_mobile_selector_for_transport(selector: &str) -> (MobileSelectorFlavor, String) {
    let trimmed = selector.trim();
    if let Some((flavor, prefix_len)) = parse_explicit_mobile_selector_prefix(trimmed) {
        let body = decode_selector_body(&trimmed[prefix_len..]);
        return match flavor {
            MobileSelectorFlavor::Css if prefix_len == 3 => {
                let normalized = if body.starts_with('#') {
                    body
                } else {
                    format!("#{body}")
                };
                (MobileSelectorFlavor::Css, normalized)
            }
            MobileSelectorFlavor::UiAutomator
                if prefix_len != 4 && !trimmed[..prefix_len].eq_ignore_ascii_case("text=") =>
            {
                (
                    MobileSelectorFlavor::UiAutomator,
                    format!("{}={body}", &trimmed[..prefix_len - 1]),
                )
            }
            MobileSelectorFlavor::UiAutomator if prefix_len == 5 => {
                (MobileSelectorFlavor::UiAutomator, format!("text={body}"))
            }
            _ => (flavor, body),
        };
    }

    if trimmed.starts_with("//")
        || trimmed.starts_with(".//")
        || trimmed.starts_with("../")
        || trimmed.starts_with('/')
        || trimmed.starts_with('(')
    {
        return (MobileSelectorFlavor::XPath, trimmed.to_string());
    }

    (MobileSelectorFlavor::Css, trimmed.to_string())
}

fn normalize_mobile_selector_for_transport(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if is_normalized_mobile_transport_selector(trimmed) {
        return trimmed.to_string();
    }
    let (flavor, body) = parse_mobile_selector_for_transport(selector);
    format!(
        "{}={}",
        flavor.as_str(),
        serde_json::to_string(&body).unwrap_or_else(|_| format!("{body:?}"))
    )
}

fn chain_mobile_selector_for_transport(parent: &str, child: &str) -> String {
    let parent = if parent.trim().is_empty() {
        String::new()
    } else {
        normalize_mobile_selector_for_transport(parent)
    };
    let child = if child.trim().is_empty() {
        String::new()
    } else {
        normalize_mobile_selector_for_transport(child)
    };
    if parent.is_empty() {
        return child;
    }
    if child.is_empty() {
        return parent;
    }
    format!("{parent} {child}")
}
