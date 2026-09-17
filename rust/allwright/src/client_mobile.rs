use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::{Read, Write},
    marker::PhantomData,
    path::Path,
};

use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::hook_completed_event::Result as HookCompletionResult;
use crate::proto::register_hook_command::Hook as RegisterHook;
use crate::proto::surface_session_command::Command as SurfaceCommand;
use crate::proto::surface_session_event::Event as SurfaceEvent;
use crate::proto::{
    AccessibilitySnapshotCommand, AppLaunchedEvent, ClickElementCommand, ConnectMobileCommand,
    ContextSessionCommand, CountElementsCommand, FillElementCommand, FocusElementCommand,
    GetInnerTextCommand, GetTextContentCommand, LaunchAppCommand, MobileConnectedEvent,
    MobilePlatform as ProtoMobilePlatform, PressKeyCommand, ReadFileChunkCommand,
    RegisterHookCommand, RegisterMobileDownloadHook, RegisterMobileFileChooserHook,
    SaveMobileDownloadCommand, ScreenshotCommand, SetMobileFileChooserFilesCommand,
    SurfaceSessionCommand, UploadFileChunkCommand, WaitForHookCommand, WaitForSelectorCommand,
};

use super::hook::{DownloadHook, FileChooserHook};

static MOBILE_TRANSFER_COUNTER: AtomicU64 = AtomicU64::new(1);

pub trait AndroidHookType: private::Sealed + Clone + Send + Sync + 'static {
    type Output;
}

mod private {
    use super::*;
    pub trait Sealed {
        fn name() -> &'static str;
        fn decode(
            app: &AndroidApp,
            result: HookCompletionResult,
        ) -> Result<<Self as AndroidHookType>::Output>
        where
            Self: AndroidHookType;
    }
}

impl AndroidHookType for FileChooserHook {
    type Output = AndroidFileChooser;
}
impl private::Sealed for FileChooserHook {
    fn name() -> &'static str {
        "file_chooser"
    }
    fn decode(app: &AndroidApp, result: HookCompletionResult) -> Result<AndroidFileChooser> {
        let HookCompletionResult::MobileFileChooser(result) = result else {
            return Err(Error::new(
                "Android file chooser hook returned an invalid result",
            ));
        };
        Ok(AndroidFileChooser {
            app: app.clone(),
            id: result.file_chooser_id,
            is_multiple: result.is_multiple,
        })
    }
}
impl AndroidHookType for DownloadHook {
    type Output = AndroidDownload;
}
impl private::Sealed for DownloadHook {
    fn name() -> &'static str {
        "download"
    }
    fn decode(app: &AndroidApp, result: HookCompletionResult) -> Result<AndroidDownload> {
        let HookCompletionResult::MobileDownload(result) = result else {
            return Err(Error::new(
                "Android download hook returned an invalid result",
            ));
        };
        Ok(AndroidDownload {
            app: app.clone(),
            id: result.download_id,
            suggested_filename: result.suggested_filename,
        })
    }
}

pub struct AndroidHook<T: AndroidHookType> {
    app: AndroidApp,
    id: String,
    _type: PhantomData<T>,
}

#[derive(Clone)]
pub struct AndroidFileChooser {
    app: AndroidApp,
    id: String,
    is_multiple: bool,
}

#[derive(Clone)]
pub struct AndroidDownload {
    app: AndroidApp,
    id: String,
    suggested_filename: String,
}
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use super::command::command_retry_options;
use super::runtime::get_runtime;
use super::types::{
    AccessibilitySnapshotFormat, AccessibilitySnapshotMode, AccessibilitySnapshotOptions,
    ClickResult, CommandOptions, CountResult, ElementResult, Error, FillResult, PressOptions,
    PressResult, Result, RuntimeClient, ScreenshotOptions, ScreenshotResult, TextResult,
    WaitForSelectorOptions, WaitForSelectorResult,
};

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
            let event = events.message().await?.ok_or_else(|| {
                Error::new("surface session closed before mobile connect response")
            })?;

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
            let event =
                state.events.message().await?.ok_or_else(|| {
                    Error::new("surface session closed before app launch response")
                })?;

            match event.event {
                Some(SurfaceEvent::AppLaunched(AppLaunchedEvent { app_session_id, .. })) => {
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

    pub async fn register_hook<T: AndroidHookType>(&self, _hook_type: T) -> Result<AndroidHook<T>> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;
        let hook = match T::name() {
            "file_chooser" => Some(RegisterHook::MobileFileChooser(
                RegisterMobileFileChooserHook {},
            )),
            "download" => Some(RegisterHook::MobileDownload(RegisterMobileDownloadHook {})),
            _ => return Err(Error::new("hook type is not supported by Android apps")),
        };
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::RegisterHook(RegisterHookCommand { hook })),
            })
            .await
            .map_err(|_| Error::new("failed to send Android RegisterHookCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("Android app session closed while registering hook")
                })?;
            match event.event {
                Some(ContextEvent::HookRegistered(event)) => {
                    return Ok(AndroidHook {
                        app: self.clone(),
                        id: event.hook_id,
                        _type: PhantomData,
                    });
                }
                Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
                _ => {}
            }
        }
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
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for click result")
                })?;

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
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for fill result")
                })?;

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

    pub async fn count(&self, selector: &str, options: CommandOptions) -> Result<CountResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::CountElements(CountElementsCommand {
                    css_selector: selector.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send CountElementsCommand"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for count result")
                })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementCounted(counted)) => {
                    return Ok(CountResult {
                        selector: counted.css_selector,
                        count: counted.count,
                        note: counted.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while counting Android locator {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for count result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn focus(&self, selector: &str, options: CommandOptions) -> Result<ElementResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::FocusElement(FocusElementCommand {
                    css_selector: selector.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send FocusElementCommand"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for focus result")
                })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementFocused(focused)) => {
                    return Ok(ElementResult {
                        selector: focused.css_selector,
                        note: focused.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while focusing Android locator {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for focus result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn press(
        &self,
        selector: &str,
        key: &str,
        options: PressOptions,
    ) -> Result<PressResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::PressKey(PressKeyCommand {
                    css_selector: selector.clone(),
                    key: key.to_string(),
                    text: options.text,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send PressKeyCommand"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for press result")
                })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::KeyPressed(pressed)) => {
                    return Ok(PressResult {
                        selector: pressed.css_selector,
                        key: pressed.key,
                        note: pressed.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while pressing Android key on {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for press result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn text_content(
        &self,
        selector: &str,
        options: CommandOptions,
    ) -> Result<TextResult> {
        self.read_text(selector, options, true).await
    }

    pub async fn inner_text(&self, selector: &str, options: CommandOptions) -> Result<TextResult> {
        self.read_text(selector, options, false).await
    }

    pub async fn wait_for_selector(
        &self,
        selector: &str,
        options: WaitForSelectorOptions,
    ) -> Result<WaitForSelectorResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::WaitForSelector(WaitForSelectorCommand {
                    css_selector: selector.clone(),
                    visible: options.visible,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send WaitForSelectorCommand"))?;

        loop {
            let event = handle.events.message().await?.ok_or_else(|| {
                Error::new("app session closed while waiting for selector result")
            })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::SelectorWaitSatisfied(wait)) => {
                    return Ok(WaitForSelectorResult {
                        selector: wait.css_selector,
                        visible: wait.visible,
                        note: wait.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while waiting for Android locator {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for selector result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }

    pub async fn screenshot(&self) -> Result<ScreenshotResult> {
        self.screenshot_with_options(ScreenshotOptions::default())
            .await
    }

    pub async fn screenshot_with_options(
        &self,
        options: ScreenshotOptions,
    ) -> Result<ScreenshotResult> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::Screenshot(ScreenshotCommand {
                    retry_options: command_retry_options(options.timeout_ms),
                    full_page: Some(options.full_page),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send ScreenshotCommand"))?;

        loop {
            let event = handle.events.message().await?.ok_or_else(|| {
                Error::new("app session closed while waiting for screenshot result")
            })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ScreenshotCaptured(screenshot)) => {
                    let result = ScreenshotResult {
                        png_data: screenshot.png_data,
                        note: screenshot.note,
                    };
                    if let Some(path) = options.path.as_ref() {
                        std::fs::write(path, &result.png_data).map_err(|error| {
                            Error::new(format!("write screenshot to {}: {error}", path.display()))
                        })?;
                    }
                    return Ok(result);
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while capturing Android screenshot: {}",
                        error.message
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for screenshot result",
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

    async fn read_text(
        &self,
        selector: &str,
        options: CommandOptions,
        text_content: bool,
    ) -> Result<TextResult> {
        let selector = normalize_mobile_selector_for_transport(selector);
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;

        let command = if text_content {
            ContextCommand::GetTextContent(GetTextContentCommand {
                css_selector: selector.clone(),
                retry_options: command_retry_options(options.timeout_ms),
            })
        } else {
            ContextCommand::GetInnerText(GetInnerTextCommand {
                css_selector: selector.clone(),
                retry_options: command_retry_options(options.timeout_ms),
            })
        };

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(command),
            })
            .await
            .map_err(|_| Error::new("failed to send text read command"))?;

        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("app session closed while waiting for text result")
                })?;

            match event.event {
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::TextContentResolved(text)) => {
                    return Ok(TextResult {
                        selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    });
                }
                Some(ContextEvent::InnerTextResolved(text)) => {
                    return Ok(TextResult {
                        selector: text.css_selector,
                        text: text.text,
                        note: text.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "app session error while reading Android text for {:?}: {}",
                        selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(format!(
                        "app session {} closed while waiting for text result",
                        self.inner.session_id
                    )));
                }
                _ => {}
            }
        }
    }
}

impl<T: AndroidHookType> AndroidHook<T> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn wait(&self) -> Result<T::Output> {
        self.wait_with_options(CommandOptions::default()).await
    }

    pub async fn wait_with_options(&self, options: CommandOptions) -> Result<T::Output> {
        let mut state = self.app.inner.state.lock().await;
        let handle = self.app.ensure_handle(&mut state).await?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.app.inner.surface_session_id.clone(),
                context_session_id: self.app.inner.session_id.clone(),
                command: Some(ContextCommand::WaitForHook(WaitForHookCommand {
                    hook_id: self.id.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send Android WaitForHookCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("Android app session closed while waiting for hook")
                })?;
            match event.event {
                Some(ContextEvent::HookCompleted(event)) if event.hook_id == self.id => {
                    return T::decode(
                        &self.app,
                        event
                            .result
                            .ok_or_else(|| Error::new("Android hook completed without a result"))?,
                    );
                }
                Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
                _ => {}
            }
        }
    }
}

impl AndroidFileChooser {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn app(&self) -> &AndroidApp {
        &self.app
    }
    pub fn is_multiple(&self) -> bool {
        self.is_multiple
    }
    pub async fn set_file(&self, path: impl AsRef<Path>) -> Result<()> {
        self.set_files([path]).await
    }
    pub async fn set_files<I, P>(&self, paths: I) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        if !self.is_multiple && paths.len() > 1 {
            return Err(Error::new(
                "Android file chooser does not accept multiple files",
            ));
        }
        let mut state = self.app.inner.state.lock().await;
        let handle = self.app.ensure_handle(&mut state).await?;
        let mut file_ids = Vec::with_capacity(paths.len());
        for path in paths {
            file_ids.push(upload_android_client_file(&self.app, handle, &path).await?);
        }
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.app.inner.surface_session_id.clone(),
                context_session_id: self.app.inner.session_id.clone(),
                command: Some(ContextCommand::SetMobileFileChooserFiles(
                    SetMobileFileChooserFilesCommand {
                        file_chooser_id: self.id.clone(),
                        file_ids,
                        retry_options: None,
                    },
                )),
            })
            .await
            .map_err(|_| Error::new("failed to send SetMobileFileChooserFilesCommand"))?;
        loop {
            let event = handle.events.message().await?.ok_or_else(|| {
                Error::new("Android app session closed while setting chooser files")
            })?;
            match event.event {
                Some(ContextEvent::MobileFileChooserFilesSet(result))
                    if result.file_chooser_id == self.id =>
                {
                    return Ok(());
                }
                Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
                _ => {}
            }
        }
    }
}

impl AndroidDownload {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn app(&self) -> &AndroidApp {
        &self.app
    }
    pub fn suggested_filename(&self) -> &str {
        &self.suggested_filename
    }
    pub async fn save_as(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut state = self.app.inner.state.lock().await;
        let handle = self.app.ensure_handle(&mut state).await?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.app.inner.surface_session_id.clone(),
                context_session_id: self.app.inner.session_id.clone(),
                command: Some(ContextCommand::SaveMobileDownload(
                    SaveMobileDownloadCommand {
                        download_id: self.id.clone(),
                        retry_options: None,
                    },
                )),
            })
            .await
            .map_err(|_| Error::new("failed to send SaveMobileDownloadCommand"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("Android app session closed while saving download")
                })?;
            match event.event {
                Some(ContextEvent::MobileDownloadSaved(result))
                    if result.download_id == self.id =>
                {
                    return download_android_client_file(
                        &self.app,
                        handle,
                        &result.file_id,
                        path.as_ref(),
                    )
                    .await;
                }
                Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
                _ => {}
            }
        }
    }
}

async fn upload_android_client_file(
    app: &AndroidApp,
    handle: &mut AndroidTabHandle,
    path: &Path,
) -> Result<String> {
    let mut source = fs::File::open(path)
        .map_err(|error| Error::new(format!("open upload {}: {error}", path.display())))?;
    let size = source
        .metadata()
        .map_err(|error| Error::new(error.to_string()))?
        .len();
    let transfer_id = format!(
        "rust-mobile-upload-{}-{}",
        std::process::id(),
        MOBILE_TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new("upload requires a valid file name"))?
        .to_string();
    let mut offset = 0_u64;
    loop {
        let mut data = vec![0; 256 * 1024];
        let count = source
            .read(&mut data)
            .map_err(|error| Error::new(error.to_string()))?;
        data.truncate(count);
        let last = offset + count as u64 >= size;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: app.inner.surface_session_id.clone(),
                context_session_id: app.inner.session_id.clone(),
                command: Some(ContextCommand::UploadFileChunk(UploadFileChunkCommand {
                    transfer_id: transfer_id.clone(),
                    name: name.clone(),
                    offset,
                    data,
                    last,
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send UploadFileChunkCommand"))?;
        offset += count as u64;
        if last {
            break;
        }
    }
    loop {
        let event = handle
            .events
            .message()
            .await?
            .ok_or_else(|| Error::new("Android app session closed while uploading file"))?;
        match event.event {
            Some(ContextEvent::FileUploaded(result)) if result.transfer_id == transfer_id => {
                return Ok(result.file_id);
            }
            Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
            _ => {}
        }
    }
}

async fn download_android_client_file(
    app: &AndroidApp,
    handle: &mut AndroidTabHandle,
    file_id: &str,
    path: &Path,
) -> Result<()> {
    let temporary = path.with_extension(format!(
        "allwright-{}.tmp",
        MOBILE_TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| Error::new(error.to_string()))?;
    let mut offset = 0_u64;
    loop {
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: app.inner.surface_session_id.clone(),
                context_session_id: app.inner.session_id.clone(),
                command: Some(ContextCommand::ReadFileChunk(ReadFileChunkCommand {
                    file_id: file_id.to_string(),
                    offset,
                    max_bytes: 256 * 1024,
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send ReadFileChunkCommand"))?;
        let event = handle
            .events
            .message()
            .await?
            .ok_or_else(|| Error::new("Android app session closed while downloading file"))?;
        match event.event {
            Some(ContextEvent::FileChunk(chunk))
                if chunk.file_id == file_id && chunk.offset == offset =>
            {
                output
                    .write_all(&chunk.data)
                    .map_err(|error| Error::new(error.to_string()))?;
                offset += chunk.data.len() as u64;
                if chunk.last {
                    drop(output);
                    fs::rename(&temporary, path).map_err(|error| Error::new(error.to_string()))?;
                    return Ok(());
                }
            }
            Some(ContextEvent::Error(error)) => {
                let _ = fs::remove_file(&temporary);
                return Err(Error::new(error.message));
            }
            _ => {}
        }
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

    pub async fn count(&self, options: CommandOptions) -> Result<CountResult> {
        self.page.count(&self.selector, options).await
    }

    pub async fn focus(&self, options: CommandOptions) -> Result<ElementResult> {
        self.page.focus(&self.selector, options).await
    }

    pub async fn fill(&self, value: &str, options: CommandOptions) -> Result<FillResult> {
        self.page.fill(&self.selector, value, options).await
    }

    pub async fn press(&self, key: &str, options: PressOptions) -> Result<PressResult> {
        self.page.press(&self.selector, key, options).await
    }

    pub async fn text_content(&self, options: CommandOptions) -> Result<TextResult> {
        self.page.text_content(&self.selector, options).await
    }

    pub async fn inner_text(&self, options: CommandOptions) -> Result<TextResult> {
        self.page.inner_text(&self.selector, options).await
    }

    pub async fn wait_for(&self, options: WaitForSelectorOptions) -> Result<WaitForSelectorResult> {
        self.page.wait_for_selector(&self.selector, options).await
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

impl AndroidApp {
    pub async fn accessibility_snapshot(&self) -> Result<String> {
        self.accessibility_snapshot_with_options(AccessibilitySnapshotOptions::default())
            .await
    }

    pub async fn accessibility_snapshot_with_options(
        &self,
        options: AccessibilitySnapshotOptions,
    ) -> Result<String> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_android_app_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::AccessibilitySnapshot(
                    AccessibilitySnapshotCommand {
                        format: match options.format {
                            AccessibilitySnapshotFormat::Json => "json",
                            AccessibilitySnapshotFormat::Yaml => "yaml",
                        }
                        .into(),
                        mode: match options.mode {
                            AccessibilitySnapshotMode::Default => "default",
                            AccessibilitySnapshotMode::Ai => "ai",
                            AccessibilitySnapshotMode::Autoexpect => "autoexpect",
                            AccessibilitySnapshotMode::Codegen => "codegen",
                        }
                        .into(),
                        retry_options: command_retry_options(options.timeout_ms),
                    },
                )),
            })
            .await
            .map_err(|_| Error::new("failed to send AccessibilitySnapshotCommand"))?;
        loop {
            let event = handle.events.message().await?.ok_or_else(|| {
                Error::new("android app session closed while capturing accessibility snapshot")
            })?;
            match event.event {
                Some(ContextEvent::AccessibilitySnapshotCaptured(result)) => {
                    return Ok(result.snapshot);
                }
                Some(ContextEvent::Error(error)) => return Err(Error::new(error.message)),
                Some(ContextEvent::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(
                        "android app session closed while capturing accessibility snapshot",
                    ));
                }
                _ => {}
            }
        }
    }
}
