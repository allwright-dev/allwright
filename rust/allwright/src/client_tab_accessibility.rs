use super::command::command_retry_options;
use super::tab::ensure_tab_open;
use super::types::{AccessibilitySnapshotFormat, AccessibilitySnapshotOptions, Error, Result, Tab};
use crate::proto::context_session_command::Command;
use crate::proto::context_session_event::Event;
use crate::proto::{AccessibilitySnapshotCommand, ContextSessionCommand};

impl Tab {
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
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(Command::AccessibilitySnapshot(
                    AccessibilitySnapshotCommand {
                        format: match options.format {
                            AccessibilitySnapshotFormat::Json => "json",
                            AccessibilitySnapshotFormat::Yaml => "yaml",
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
                Error::new("page session closed while capturing accessibility snapshot")
            })?;
            match event.event {
                Some(Event::AccessibilitySnapshotCaptured(result)) => return Ok(result.snapshot),
                Some(Event::Error(error)) => return Err(Error::new(error.message)),
                Some(Event::Closed(_)) => {
                    handle.closed = true;
                    return Err(Error::new(
                        "page session closed while capturing accessibility snapshot",
                    ));
                }
                _ => {}
            }
        }
    }
}
