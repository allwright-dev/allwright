use super::command::command_retry_options;
use super::selectors::normalize_selector_for_transport;
use super::tab::ensure_tab_open;
use super::types::*;
use crate::proto::{self, context_session_command::Command, context_session_event::Event};

impl Tab {
    async fn capture(
        &self,
        kind: proto::CaptureKind,
        selector: &str,
        attribute_name: &str,
        options: CommandOptions,
    ) -> Result<proto::CaptureResolvedEvent> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(proto::ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(Command::Capture(proto::CaptureCommand {
                    kind: kind as i32,
                    css_selector: if selector.is_empty() {
                        String::new()
                    } else {
                        normalize_selector_for_transport(selector)
                    },
                    attribute_name: attribute_name.into(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send capture command"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("page session closed while capturing"))?;
            match event.event {
                Some(Event::CaptureResolved(result)) => return Ok(result),
                Some(Event::Error(error)) => return Err(Error::new(error.message)),
                Some(Event::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new("page session closed while capturing"));
                }
                _ => {}
            }
        }
    }
    pub async fn url(&self) -> Result<String> {
        self.url_with_options(CommandOptions::default()).await
    }
    pub async fn url_with_options(&self, options: CommandOptions) -> Result<String> {
        let r = self
            .capture(proto::CaptureKind::Url, "", "", options)
            .await?;
        Ok(r.value.unwrap_or_default())
    }
    pub async fn input_value(&self, selector: impl Into<String>) -> Result<String> {
        self.input_value_with_options(selector, CommandOptions::default())
            .await
    }
    pub async fn input_value_with_options(
        &self,
        selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<String> {
        let r = self
            .capture(
                proto::CaptureKind::InputValue,
                &selector.into(),
                "",
                options,
            )
            .await?;
        Ok(r.value.unwrap_or_default())
    }
    pub async fn selected_options(
        &self,
        selector: impl Into<String>,
    ) -> Result<Vec<CapturedOption>> {
        self.selected_options_with_options(selector, CommandOptions::default())
            .await
    }
    pub async fn selected_options_with_options(
        &self,
        selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<Vec<CapturedOption>> {
        let r = self
            .capture(
                proto::CaptureKind::SelectedOptions,
                &selector.into(),
                "",
                options,
            )
            .await?;
        Ok(r.selected_options
            .into_iter()
            .map(|o| CapturedOption {
                value: o.value,
                label: o.label,
                index: o.index,
            })
            .collect())
    }
    pub async fn selected_text(&self, selector: impl Into<String>) -> Result<Option<String>> {
        self.selected_text_with_options(selector, CommandOptions::default())
            .await
    }
    pub async fn selected_text_with_options(
        &self,
        selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<Option<String>> {
        let r = self
            .capture(
                proto::CaptureKind::SelectedText,
                &selector.into(),
                "",
                options,
            )
            .await?;
        Ok(r.value)
    }
    pub async fn is_checked(&self, selector: impl Into<String>) -> Result<bool> {
        self.is_checked_with_options(selector, CommandOptions::default())
            .await
    }
    pub async fn is_checked_with_options(
        &self,
        selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<bool> {
        let r = self
            .capture(proto::CaptureKind::Checked, &selector.into(), "", options)
            .await?;
        Ok(r.checked.unwrap_or(false))
    }
    pub async fn get_attribute(
        &self,
        selector: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Option<String>> {
        self.get_attribute_with_options(selector, name, CommandOptions::default())
            .await
    }
    pub async fn get_attribute_with_options(
        &self,
        selector: impl Into<String>,
        name: impl Into<String>,
        options: CommandOptions,
    ) -> Result<Option<String>> {
        let r = self
            .capture(
                proto::CaptureKind::Attribute,
                &selector.into(),
                &name.into(),
                options,
            )
            .await?;
        Ok(r.value)
    }
    pub async fn bounding_box(&self, selector: impl Into<String>) -> Result<Option<BoundingBox>> {
        self.bounding_box_with_options(selector, CommandOptions::default())
            .await
    }
    pub async fn bounding_box_with_options(
        &self,
        selector: impl Into<String>,
        options: CommandOptions,
    ) -> Result<Option<BoundingBox>> {
        let r = self
            .capture(
                proto::CaptureKind::BoundingBox,
                &selector.into(),
                "",
                options,
            )
            .await?;
        Ok(r.bounding_box.map(|b| BoundingBox {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }))
    }
}
impl Locator {
    pub async fn input_value(&self) -> Result<String> {
        self.input_value_with_options(CommandOptions::default())
            .await
    }
    pub async fn input_value_with_options(&self, options: CommandOptions) -> Result<String> {
        self.page
            .input_value_with_options(self.selector.clone(), options)
            .await
    }
    pub async fn selected_options(&self) -> Result<Vec<CapturedOption>> {
        self.selected_options_with_options(CommandOptions::default())
            .await
    }
    pub async fn selected_options_with_options(
        &self,
        options: CommandOptions,
    ) -> Result<Vec<CapturedOption>> {
        self.page
            .selected_options_with_options(self.selector.clone(), options)
            .await
    }
    pub async fn selected_text(&self) -> Result<Option<String>> {
        self.selected_text_with_options(CommandOptions::default())
            .await
    }
    pub async fn selected_text_with_options(
        &self,
        options: CommandOptions,
    ) -> Result<Option<String>> {
        self.page
            .selected_text_with_options(self.selector.clone(), options)
            .await
    }
    pub async fn is_checked(&self) -> Result<bool> {
        self.is_checked_with_options(CommandOptions::default())
            .await
    }
    pub async fn is_checked_with_options(&self, options: CommandOptions) -> Result<bool> {
        self.page
            .is_checked_with_options(self.selector.clone(), options)
            .await
    }
    pub async fn get_attribute(&self, name: impl Into<String>) -> Result<Option<String>> {
        self.get_attribute_with_options(name, CommandOptions::default())
            .await
    }
    pub async fn get_attribute_with_options(
        &self,
        name: impl Into<String>,
        options: CommandOptions,
    ) -> Result<Option<String>> {
        self.page
            .get_attribute_with_options(self.selector.clone(), name, options)
            .await
    }
    pub async fn bounding_box(&self) -> Result<Option<BoundingBox>> {
        self.bounding_box_with_options(CommandOptions::default())
            .await
    }
    pub async fn bounding_box_with_options(
        &self,
        options: CommandOptions,
    ) -> Result<Option<BoundingBox>> {
        self.page
            .bounding_box_with_options(self.selector.clone(), options)
            .await
    }
}
