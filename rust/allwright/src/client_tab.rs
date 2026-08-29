use crate::proto::tab_session_command::Command as TabCommand;
use crate::proto::tab_session_event::Event as TabEvent;
use crate::proto::{
    CloseTabSessionCommand, NavigateTabCommand, TabSessionCommand, TabSessionPingCommand,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::command::command_retry_options;
use super::selectors::normalize_selector_for_transport;
use super::types::{CommandOptions, Error, NavigateResult, Result, Tab, TabHandle, TabState};

impl Tab {
    pub fn locator(&self, css_selector: impl Into<String>) -> super::types::Locator {
        super::types::Locator {
            page: self.clone(),
            selector: normalize_selector_for_transport(&css_selector.into()),
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
        ensure_tab_open(handle, &self.inner.session_id)?;

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
        ensure_tab_open(handle, &self.inner.session_id)?;

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

            if navigated.is_some() && injection.is_some() {
                let navigated_event = navigated
                    .take()
                    .ok_or_else(|| Error::new("navigation event disappeared unexpectedly"))?;
                let injection_event = injection
                    .take()
                    .ok_or_else(|| Error::new("bidi injection event disappeared unexpectedly"))?;
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

    pub(crate) async fn ensure_handle<'a>(
        &self,
        state: &'a mut TabState,
    ) -> Result<&'a mut TabHandle> {
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

pub(crate) fn ensure_tab_open(handle: &TabHandle, session_id: &str) -> Result<()> {
    if handle.closed {
        return Err(Error::new(format!("tab session {} is closed", session_id)));
    }
    Ok(())
}
