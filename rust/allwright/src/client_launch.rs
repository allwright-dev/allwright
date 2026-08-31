use std::sync::Arc;

use crate::proto::surface_session_command::Command as SurfaceCommand;
use crate::proto::surface_session_event::Event as SurfaceEvent;
use crate::proto::{
    BrowserKind as ProtoBrowserKind, BrowserLaunchedEvent, LaunchBrowserCommand,
    SurfaceSessionCommand,
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use super::command::command_retry_options;
use super::runtime::get_runtime;
use super::types::{
    Browser, BrowserInner, BrowserKind, BrowserState, BrowserType, Error, LaunchOptions, Result,
    Tab, TabInner, TabState,
};

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
        .surface_session(tonic::Request::new(ReceiverStream::new(command_rx)))
        .await?;
    let mut events = response.into_inner();

    command_tx
        .send(SurfaceSessionCommand {
            command: Some(SurfaceCommand::LaunchBrowser(LaunchBrowserCommand {
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
            Some(SurfaceEvent::BrowserLaunched(BrowserLaunchedEvent {
                browser,
                note,
                user_data_dir,
                initial_page_session_id,
                ..
            })) => {
                return Ok(build_browser(
                    runtime,
                    command_tx,
                    events,
                    event.session_id,
                    browser,
                    note,
                    String::new(),
                    user_data_dir,
                    initial_page_session_id,
                ));
            }
            Some(SurfaceEvent::ChromeLaunched(launched)) => {
                return Ok(build_browser(
                    runtime,
                    command_tx,
                    events,
                    event.session_id,
                    launched.browser,
                    launched.note,
                    launched.cdp_websocket_url,
                    launched.user_data_dir,
                    launched.initial_page_session_id,
                ));
            }
            Some(SurfaceEvent::Error(error)) => {
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

impl BrowserType {
    pub async fn launch(&self, options: LaunchOptions) -> Result<Browser> {
        launch_browser(self.browser_kind, options).await
    }
}

fn build_browser(
    runtime: Arc<super::types::RuntimeClient>,
    command_tx: mpsc::Sender<SurfaceSessionCommand>,
    events: tonic::Streaming<crate::proto::SurfaceSessionEvent>,
    surface_session_id: String,
    browser: String,
    note: String,
    cdp_websocket_url: String,
    user_data_dir: String,
    initial_page_session_id: String,
) -> Browser {
    let initial_tab = Tab {
        inner: Arc::new(TabInner {
            runtime: Arc::clone(&runtime),
            surface_session_id: surface_session_id.clone(),
            session_id: initial_page_session_id,
            state: AsyncMutex::new(TabState::default()),
        }),
    };
    Browser {
        inner: Arc::new(BrowserInner {
            runtime,
            state: AsyncMutex::new(BrowserState {
                command_tx,
                events,
                closed: false,
            }),
            session_id: surface_session_id,
            browser_name: browser,
            launch_note: note,
            cdp_websocket_url,
            user_data_dir,
            initial_tab,
        }),
    }
}
