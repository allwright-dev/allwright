use std::marker::PhantomData;
use std::sync::Arc;

use crate::proto::hook_completed_event::Result as HookCompletionResult;
use crate::proto::register_hook_command::Hook as RegisterHook;
use crate::proto::surface_session_command::Command as SurfaceCommand;
use crate::proto::surface_session_event::Event as SurfaceEvent;
use crate::proto::{
    RegisterHookCommand, RegisterNewPageHook, SurfaceSessionCommand, WaitForHookCommand,
};
use tokio::sync::Mutex as AsyncMutex;

use super::command::command_retry_options;
use super::types::{Browser, CommandOptions, Error, Result, Tab, TabInner, TabState};

pub trait HookType: private::Sealed + Clone + Send + Sync + 'static {
    type Output;
}

mod private {
    use super::*;

    pub trait Sealed {
        fn name() -> &'static str;
        fn decode(
            browser: &Browser,
            result: HookCompletionResult,
        ) -> Result<<Self as HookType>::Output>
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

    fn decode(
        browser: &Browser,
        result: HookCompletionResult,
    ) -> Result<<Self as HookType>::Output> {
        let HookCompletionResult::NewPage(page) = result;
        Ok(Tab {
            inner: Arc::new(TabInner {
                runtime: Arc::clone(&browser.inner.runtime),
                surface_session_id: browser.inner.session_id.clone(),
                session_id: page.context_session_id,
                state: AsyncMutex::new(TabState::default()),
            }),
        })
    }
}

pub struct Hook<T: HookType> {
    browser: Browser,
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
        let mut state = self.browser.inner.state.lock().await;
        if state.closed {
            return Err(Error::new(format!(
                "browser session {} is closed",
                self.browser.inner.session_id
            )));
        }
        state
            .command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::WaitForHook(WaitForHookCommand {
                    hook_id: self.id.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send WaitForHookCommand"))?;

        loop {
            let event = state
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("browser session closed while waiting for hook"))?;
            match event.event {
                Some(SurfaceEvent::HookCompleted(completed)) if completed.hook_id == self.id => {
                    let result = completed
                        .result
                        .ok_or_else(|| Error::new("hook completed without a result"))?;
                    return T::decode(&self.browser, result);
                }
                Some(SurfaceEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "browser session error while waiting for hook: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}

impl Browser {
    pub async fn register_hook<T: HookType>(&self, _hook_type: T) -> Result<Hook<T>> {
        let mut state = self.inner.state.lock().await;
        if state.closed {
            return Err(Error::new(format!(
                "browser session {} is closed",
                self.inner.session_id
            )));
        }
        state
            .command_tx
            .send(SurfaceSessionCommand {
                command: Some(SurfaceCommand::RegisterHook(RegisterHookCommand {
                    hook: match T::name() {
                        "new_page" => Some(RegisterHook::NewPage(RegisterNewPageHook {})),
                        _ => return Err(Error::new("unsupported hook type")),
                    },
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send RegisterHookCommand"))?;

        loop {
            let event = state
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("browser session closed while registering hook"))?;
            match event.event {
                Some(SurfaceEvent::HookRegistered(registered)) => {
                    return Ok(Hook {
                        browser: self.clone(),
                        id: registered.hook_id,
                        _type: PhantomData,
                    });
                }
                Some(SurfaceEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "browser session error while registering hook: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}
