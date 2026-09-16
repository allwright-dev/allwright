use std::marker::PhantomData;
use std::sync::Arc;

use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::hook_completed_event::Result as HookCompletionResult;
use crate::proto::register_hook_command::Hook as RegisterHook;
use crate::proto::{
    ContextSessionCommand, RegisterHookCommand, RegisterNewPageHook, WaitForHookCommand,
};
use tokio::sync::Mutex as AsyncMutex;

use super::command::command_retry_options;
use super::tab::ensure_tab_open;
use super::types::{CommandOptions, Error, Result, Tab, TabInner, TabState};

pub trait HookType: private::Sealed + Clone + Send + Sync + 'static {
    type Output;
}

mod private {
    use super::*;

    pub trait Sealed {
        fn name() -> &'static str;
        fn decode(page: &Tab, result: HookCompletionResult) -> Result<<Self as HookType>::Output>
        where
            Self: HookType;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NewPage;

pub const NEW_PAGE: NewPage = NewPage;

impl HookType for NewPage {
    type Output = Tab;
}

impl private::Sealed for NewPage {
    fn name() -> &'static str {
        "new_page"
    }

    fn decode(page: &Tab, result: HookCompletionResult) -> Result<<Self as HookType>::Output> {
        let HookCompletionResult::NewPage(new_page) = result;
        Ok(Tab {
            inner: Arc::new(TabInner {
                runtime: Arc::clone(&page.inner.runtime),
                surface_session_id: page.inner.surface_session_id.clone(),
                session_id: new_page.context_session_id,
                state: AsyncMutex::new(TabState::default()),
            }),
        })
    }
}

pub struct Hook<T: HookType> {
    page: Tab,
    id: String,
    _type: PhantomData<T>,
}

impl<T: HookType> Hook<T> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn wait(&self) -> Result<T::Output> {
        self.wait_with_options(CommandOptions::default()).await
    }

    pub async fn wait_with_options(&self, options: CommandOptions) -> Result<T::Output> {
        let mut state = self.page.inner.state.lock().await;
        let handle = self.page.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.page.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.page.inner.surface_session_id.clone(),
                context_session_id: self.page.inner.session_id.clone(),
                command: Some(ContextCommand::WaitForHook(WaitForHookCommand {
                    hook_id: self.id.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send WaitForHookCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("page session closed while waiting for hook"))?;
            match event.event {
                Some(ContextEvent::HookCompleted(completed)) if completed.hook_id == self.id => {
                    let result = completed
                        .result
                        .ok_or_else(|| Error::new("hook completed without a result"))?;
                    return T::decode(&self.page, result);
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "page session error while waiting for hook: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}

impl Tab {
    pub async fn register_hook<T: HookType>(&self, _hook_type: T) -> Result<Hook<T>> {
        let mut state = self.inner.state.lock().await;
        let handle = self.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.inner.surface_session_id.clone(),
                context_session_id: self.inner.session_id.clone(),
                command: Some(ContextCommand::RegisterHook(RegisterHookCommand {
                    hook: match T::name() {
                        "new_page" => Some(RegisterHook::NewPage(RegisterNewPageHook {})),
                        _ => return Err(Error::new("unsupported hook type")),
                    },
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send RegisterHookCommand"))?;

        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("page session closed while registering hook"))?;
            match event.event {
                Some(ContextEvent::HookRegistered(registered)) => {
                    return Ok(Hook {
                        page: self.clone(),
                        id: registered.hook_id,
                        _type: PhantomData,
                    });
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "page session error while registering hook: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}
