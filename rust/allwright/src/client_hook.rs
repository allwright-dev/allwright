use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs,
    io::{Read, Write},
};

use crate::proto::context_session_command::Command as ContextCommand;
use crate::proto::context_session_event::Event as ContextEvent;
use crate::proto::hook_completed_event::Result as HookCompletionResult;
use crate::proto::register_hook_command::Hook as RegisterHook;
use crate::proto::{
    ContextSessionCommand, ReadFileChunkCommand, RegisterDownloadHook, RegisterFileChooserHook,
    RegisterHookCommand, RegisterNewPageHook, SaveDownloadCommand, SetFileChooserFilesCommand,
    UploadFileChunkCommand, WaitForHookCommand,
};
use std::path::Path;
use tokio::sync::Mutex as AsyncMutex;

use super::command::command_retry_options;
use super::tab::ensure_tab_open;
use super::types::{CommandOptions, Error, Result, Tab, TabInner, TabState};

static TRANSFER_COUNTER: AtomicU64 = AtomicU64::new(1);

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
        let HookCompletionResult::NewPage(new_page) = result else {
            return Err(Error::new("new page hook completed with an invalid result"));
        };
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

#[derive(Debug, Clone, Copy, Default)]
pub struct FileChooserHook;

pub const FILE_CHOOSER: FileChooserHook = FileChooserHook;

impl HookType for FileChooserHook {
    type Output = FileChooser;
}

impl private::Sealed for FileChooserHook {
    fn name() -> &'static str {
        "file_chooser"
    }

    fn decode(page: &Tab, result: HookCompletionResult) -> Result<<Self as HookType>::Output> {
        let HookCompletionResult::FileChooser(file_chooser) = result else {
            return Err(Error::new(
                "file chooser hook completed with an invalid result",
            ));
        };
        Ok(FileChooser {
            page: page.clone(),
            id: file_chooser.file_chooser_id,
            is_multiple: file_chooser.is_multiple,
        })
    }
}

#[derive(Clone)]
pub struct FileChooser {
    page: Tab,
    id: String,
    is_multiple: bool,
}

impl FileChooser {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_multiple(&self) -> bool {
        self.is_multiple
    }

    pub fn page(&self) -> &Tab {
        &self.page
    }

    pub async fn set_file(&self, file: impl AsRef<Path>) -> Result<()> {
        self.set_files([file]).await
    }

    pub async fn set_files<I, P>(&self, files: I) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let files = files
            .into_iter()
            .map(|file| file.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        let mut state = self.page.inner.state.lock().await;
        let handle = self.page.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.page.inner.session_id)?;
        let mut file_ids = Vec::with_capacity(files.len());
        for path in files {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| Error::new("upload path requires a valid file name"))?
                .to_string();
            let transfer_id = format!(
                "rust-upload-{}-{}",
                std::process::id(),
                TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let mut source = fs::File::open(&path)
                .map_err(|error| Error::new(format!("open upload {}: {error}", path.display())))?;
            let size = source
                .metadata()
                .map_err(|error| Error::new(format!("inspect upload {}: {error}", path.display())))?
                .len();
            let mut offset = 0_u64;
            loop {
                let mut data = vec![0; 256 * 1024];
                let count = source.read(&mut data).map_err(|error| {
                    Error::new(format!("read upload {}: {error}", path.display()))
                })?;
                data.truncate(count);
                let last = offset + count as u64 >= size;
                handle
                    .command_tx
                    .send(ContextSessionCommand {
                        surface_session_id: self.page.inner.surface_session_id.clone(),
                        context_session_id: self.page.inner.session_id.clone(),
                        command: Some(ContextCommand::UploadFileChunk(UploadFileChunkCommand {
                            transfer_id: transfer_id.clone(),
                            name: name.clone(),
                            offset,
                            data,
                            last,
                        })),
                    })
                    .await
                    .map_err(|_| Error::new("failed to send UploadFileChunkCommand"))?;
                offset += count as u64;
                if last {
                    break;
                }
            }
            loop {
                let event = handle.events.message().await?.ok_or_else(|| {
                    Error::new("page session closed while uploading chooser file")
                })?;
                match event.event {
                    Some(ContextEvent::FileUploaded(result))
                        if result.transfer_id == transfer_id =>
                    {
                        file_ids.push(result.file_id);
                        break;
                    }
                    Some(ContextEvent::Error(error)) => {
                        return Err(Error::new(format!(
                            "page session error while uploading chooser file: {}",
                            error.message
                        )));
                    }
                    _ => {}
                }
            }
        }
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.page.inner.surface_session_id.clone(),
                context_session_id: self.page.inner.session_id.clone(),
                command: Some(ContextCommand::SetFileChooserFiles(
                    SetFileChooserFilesCommand {
                        file_chooser_id: self.id.clone(),
                        file_ids,
                        retry_options: None,
                    },
                )),
            })
            .await
            .map_err(|_| Error::new("failed to send SetFileChooserFilesCommand"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("page session closed while setting chooser files"))?;
            match event.event {
                Some(ContextEvent::FileChooserFilesSet(result))
                    if result.file_chooser_id == self.id =>
                {
                    return Ok(());
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "page session error while setting chooser files: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadHook;

pub const DOWNLOAD: DownloadHook = DownloadHook;

impl HookType for DownloadHook {
    type Output = Download;
}

impl private::Sealed for DownloadHook {
    fn name() -> &'static str {
        "download"
    }

    fn decode(page: &Tab, result: HookCompletionResult) -> Result<<Self as HookType>::Output> {
        let HookCompletionResult::Download(download) = result else {
            return Err(Error::new("download hook completed with an invalid result"));
        };
        Ok(Download {
            page: page.clone(),
            id: download.download_id,
            url: download.url,
            suggested_filename: download.suggested_filename,
        })
    }
}

#[derive(Clone)]
pub struct Download {
    page: Tab,
    id: String,
    url: String,
    suggested_filename: String,
}

impl Download {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn page(&self) -> &Tab {
        &self.page
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn suggested_filename(&self) -> &str {
        &self.suggested_filename
    }

    pub async fn save_as(&self, path: impl AsRef<Path>) -> Result<()> {
        self.save_as_with_options(path, CommandOptions::default())
            .await
    }

    pub async fn save_as_with_options(
        &self,
        path: impl AsRef<Path>,
        options: CommandOptions,
    ) -> Result<()> {
        let mut state = self.page.inner.state.lock().await;
        let handle = self.page.ensure_handle(&mut state).await?;
        ensure_tab_open(handle, &self.page.inner.session_id)?;
        handle
            .command_tx
            .send(ContextSessionCommand {
                surface_session_id: self.page.inner.surface_session_id.clone(),
                context_session_id: self.page.inner.session_id.clone(),
                command: Some(ContextCommand::SaveDownload(SaveDownloadCommand {
                    download_id: self.id.clone(),
                    retry_options: command_retry_options(options.timeout_ms),
                })),
            })
            .await
            .map_err(|_| Error::new("failed to send SaveDownloadCommand"))?;
        loop {
            let event = handle
                .events
                .message()
                .await?
                .ok_or_else(|| Error::new("page session closed while saving download"))?;
            match event.event {
                Some(ContextEvent::DownloadSaved(result)) if result.download_id == self.id => {
                    let destination = path.as_ref();
                    let temporary = destination.with_extension(format!(
                        "allwright-{}.tmp",
                        TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed)
                    ));
                    let mut output = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&temporary)
                        .map_err(|error| Error::new(format!("create download file: {error}")))?;
                    let mut offset = 0_u64;
                    loop {
                        handle
                            .command_tx
                            .send(ContextSessionCommand {
                                surface_session_id: self.page.inner.surface_session_id.clone(),
                                context_session_id: self.page.inner.session_id.clone(),
                                command: Some(ContextCommand::ReadFileChunk(
                                    ReadFileChunkCommand {
                                        file_id: result.file_id.clone(),
                                        offset,
                                        max_bytes: 256 * 1024,
                                    },
                                )),
                            })
                            .await
                            .map_err(|_| Error::new("failed to send ReadFileChunkCommand"))?;
                        let chunk = handle.events.message().await?.ok_or_else(|| {
                            Error::new("page session closed while downloading file")
                        })?;
                        match chunk.event {
                            Some(ContextEvent::FileChunk(chunk))
                                if chunk.file_id == result.file_id && chunk.offset == offset =>
                            {
                                output.write_all(&chunk.data).map_err(|error| {
                                    Error::new(format!("write download file: {error}"))
                                })?;
                                offset += chunk.data.len() as u64;
                                if chunk.last {
                                    drop(output);
                                    fs::rename(&temporary, destination).map_err(|error| {
                                        Error::new(format!("finish download file: {error}"))
                                    })?;
                                    return Ok(());
                                }
                            }
                            Some(ContextEvent::Error(error)) => {
                                let _ = fs::remove_file(&temporary);
                                return Err(Error::new(format!(
                                    "page session error while downloading file: {}",
                                    error.message
                                )));
                            }
                            _ => {}
                        }
                    }
                }
                Some(ContextEvent::Error(error)) => {
                    return Err(Error::new(format!(
                        "page session error while saving download: {}",
                        error.message
                    )));
                }
                _ => {}
            }
        }
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
                        "file_chooser" => {
                            Some(RegisterHook::FileChooser(RegisterFileChooserHook {}))
                        }
                        "download" => Some(RegisterHook::Download(RegisterDownloadHook {})),
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
