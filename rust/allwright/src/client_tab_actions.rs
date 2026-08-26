use crate::proto::tab_session_command::Command as TabCommand;
use crate::proto::tab_session_event::Event as TabEvent;
use crate::proto::{
    ClickElementCommand, FillElementCommand, FocusElementCommand, HoverElementCommand,
    PressKeyCommand, TabSessionCommand,
};

use super::command::command_retry_options;
use super::selectors::normalize_selector_for_transport;
use super::tab::ensure_tab_open;
use super::types::{
    ClickResult, CommandOptions, ElementResult, Error, FillResult, PressOptions, PressResult,
    Result, Tab,
};

impl Tab {
    pub async fn click(&self, css_selector: impl Into<String>) -> Result<ClickResult> {
        self.click_with_options(css_selector, CommandOptions::default()).await
    }

    pub async fn click_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ClickResult> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::ClickElement(ClickElementCommand {
                    css_selector,
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
                .ok_or_else(|| Error::new("tab session closed while waiting for click result"))?;

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

    pub async fn focus(&self, css_selector: impl Into<String>) -> Result<ElementResult> {
        self.focus_with_options(css_selector, CommandOptions::default()).await
    }

    pub async fn focus_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ElementResult> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::FocusElement(FocusElementCommand {
                    css_selector,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send FocusElementCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for focus result"))?;
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
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::FillElement(FillElementCommand {
                    css_selector,
                    value: value.into(),
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
                .ok_or_else(|| Error::new("tab session closed while waiting for fill result"))?;
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
        self.hover_with_options(css_selector, CommandOptions::default()).await
    }

    pub async fn hover_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<ElementResult> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::HoverElement(HoverElementCommand {
                    css_selector,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send HoverElementCommand"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for hover result"))?;
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
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(TabSessionCommand {
                browser_session_id: self.inner.browser_session_id.clone(),
                tab_session_id: self.inner.session_id.clone(),
                command: Some(TabCommand::PressKey(PressKeyCommand {
                    css_selector,
                    key: key.into(),
                    text: options.text,
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send PressKeyCommand"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("tab session closed while waiting for press result"))?;
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
}
