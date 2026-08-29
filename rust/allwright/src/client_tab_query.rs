use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::{
    CountElementsCommand, GetInnerTextCommand, GetTextContentCommand, HighlightElementsCommand,
    ContextSessionCommand, WaitForSelectorCommand,
};

use super::command::{command_retry_options, count_result_from_event, highlight_result_from_event};
use super::selectors::normalize_selector_for_transport;
use super::tab::ensure_tab_open;
use super::types::{
    CommandOptions, Error, HighlightOptions, HighlightResult, Result, Tab, TextResult,
    WaitForSelectorOptions, WaitForSelectorResult,
};

impl Tab {
    pub async fn count(
        &self,
        css_selector: impl Into<String>,
    ) -> Result<super::types::CountResult> {
        self.count_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn count_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<super::types::CountResult> {
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::CountElements(CountElementsCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementCounted(counted)) => {
                    return Ok(count_result_from_event(counted));
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while counting locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;

        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::HighlightElements(HighlightElementsCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::ElementsHighlighted(highlighted)) => {
                    return Ok(highlight_result_from_event(highlighted));
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while highlighting locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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

    pub async fn text_content(&self, css_selector: impl Into<String>) -> Result<TextResult> {
        self.text_content_with_options(css_selector, CommandOptions::default())
            .await
    }

    pub async fn text_content_with_options(
        &self,
        css_selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<TextResult> {
        self.read_text(
            normalize_selector_for_transport(&css_selector.into()),
            options,
            true,
        )
        .await
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
        self.read_text(
            normalize_selector_for_transport(&css_selector.into()),
            options,
            false,
        )
        .await
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
        let css_selector = normalize_selector_for_transport(&css_selector.into());
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::WaitForSelector(WaitForSelectorCommand {
                    css_selector: css_selector.clone(),
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
                Some(ContextEvent::Attached(_)) => {}
                Some(ContextEvent::SelectorWaitSatisfied(waited)) => {
                    return Ok(WaitForSelectorResult {
                        selector: waited.css_selector,
                        visible: waited.visible,
                        note: waited.note,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "tab session error while waiting for locator {:?}: {}",
                        css_selector, error.message,
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
        ensure_tab_open(handle, &self.inner.session_id)?;
        let command = if text_content {
            ContextCommand::GetTextContent(GetTextContentCommand {
                css_selector,
                retry_options: command_retry_options(options.timeout_ms),
            })
        } else {
            ContextCommand::GetInnerText(GetInnerTextCommand {
                css_selector,
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
            .map_err(|_| Error::new("failed to send text command"))?;
        loop {
            let event =
                handle.events.message().await?.ok_or_else(|| {
                    Error::new("tab session closed while waiting for text result")
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
                        "tab session error while reading text: {}",
                        error.message
                    )));
                }
                Some(ContextEvent::Closed(_)) => {
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
}
