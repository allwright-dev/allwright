use std::sync::Arc;

use crate::proto::surface_session_command::Command as SurfaceCommand;
use crate::proto::surface_session_event::Event as SurfaceEvent;
use crate::proto::{
    CloseSurfaceSessionCommand, OpenContextCommand, SessionPingCommand, SurfaceSessionCommand,
};
use tokio::sync::Mutex as AsyncMutex;

use super::command::command_retry_options;
use super::types::{
    Browser, BrowserState, CommandOptions, Error, Page, Result, Tab, TabInner, TabState,
};

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
        ensure_browser_open(&state, &self.inner.session_id)?;

        state
            .command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::OpenContext(OpenContextCommand {
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send OpenContextCommand to browser session"))?;

        loop {
            let event =
                state.events.message().await?.ok_or_else(|| {
                    Error::new("browser session closed while waiting for new tab")
                })?;

            match event.event {
                Some(SurfaceEvent::ContextOpened(opened)) => {
                    return Ok(Tab {
                        inner: Arc::new(TabInner {
                            runtime: Arc::clone(&self.inner.runtime),
                            surface_session_id: self.inner.session_id.clone(),
                            session_id: opened.context_session_id,
                            state: AsyncMutex::new(TabState::default()),
                        }),
                    });
                }
                Some(SurfaceEvent::Error(error)) => {
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
        ensure_browser_open(&state, &self.inner.session_id)?;

        state
            .command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::Ping(SessionPingCommand {
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
                Some(SurfaceEvent::Pong(pong)) => return Ok(pong.message),
                Some(SurfaceEvent::Error(error)) => {
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
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::Close(CloseSurfaceSessionCommand {})),
            })
            .await
            .map_err(|_| Error::new("failed to send CloseSurfaceSessionCommand"))?;

        loop {
            let event =
                state.events.message().await?.ok_or_else(|| {
                    Error::new("browser session closed before close confirmation")
                })?;

            match event.event {
                Some(SurfaceEvent::Closed(_)) => {
                    state.closed = true;
                    return Ok(());
                }
                Some(SurfaceEvent::Error(error)) => {
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

fn ensure_browser_open(state: &BrowserState, session_id: &str) -> Result<()> {
    if state.closed {
        return Err(Error::new(format!(
            "browser session {} is closed",
            session_id
        )));
    }
    Ok(())
}
