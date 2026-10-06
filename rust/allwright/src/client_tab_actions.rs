use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::{
    ClickElementCommand, ContextSessionCommand, FillElementCommand, FocusElementCommand,
    HoverElementCommand, PressKeyCommand,
};

use super::command::command_retry_options;
use super::selectors::normalize_selector_for_transport;
use super::tab::ensure_tab_open;
use super::types::{ClickOptions, CommandOptions, Error, PressOptions, Result, Tab};

impl Tab {
    pub async fn click(&self, css_selector: impl Into<String>) -> Result<()> {
        self.click_with_options(css_selector, ClickOptions::default())
            .await
    }

    pub async fn click_with_options(
        &self,
        css_selector: impl Into<String>,
        options: impl Into<ClickOptions>,
    ) -> Result<()> {
        let options = options.into();
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::ClickElement(ClickElementCommand {
                    css_selector: css_selector.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                    button: Some(options.button.as_str().to_string()),
                    click_count: options.click_count,
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementClicked(_)) => return Ok(()),
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while clicking locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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

    pub async fn dblclick(&self, css_selector: impl Into<String>) -> Result<()> {
        self.dblclick_with_options(css_selector, ClickOptions::default())
            .await
    }

    pub async fn dblclick_with_options(
        &self,
        css_selector: impl Into<String>,
        mut options: ClickOptions,
    ) -> Result<()> {
        options.click_count = Some(2);
        self.click_with_options(css_selector, options).await
    }

    pub async fn focus(&self, css_selector: impl Into<String>) -> Result<()> {
        self.focus_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn focus_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<()> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::FocusElement(FocusElementCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementFocused(_)) => return Ok(()),
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while focusing locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
    ) -> Result<()> {
        self.fill_with_options(css_selector, value, CommandOptions::default())
            .await
    }

    pub async fn fill_with_options(
        &self,
        css_selector: impl Into<String>,
        value: impl Into<String>,
        options: CommandOptions,
    ) -> Result<()> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::FillElement(FillElementCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementFilled(_)) => return Ok(()),
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while filling locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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

    pub async fn hover(&self, css_selector: impl Into<String>) -> Result<()> {
        self.hover_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn hover_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<()> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::HoverElement(HoverElementCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementHovered(_)) => return Ok(()),
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while hovering locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
    ) -> Result<()> {
        self.press_with_options(css_selector, key, PressOptions::default())
            .await
    }

    pub async fn press_with_options(
        &self,
        css_selector: impl Into<String>,
        key: impl Into<String>,
        options: PressOptions,
    ) -> Result<()> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::PressKey(PressKeyCommand {
                    css_selector,
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::KeyPressed(_)) => return Ok(()),
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while pressing key: {}",
                        error.message
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
}
