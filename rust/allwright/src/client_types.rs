use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use crate::proto::engine_service_client::EngineServiceClient;
use serde::Deserialize;
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tonic::transport::Channel;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self::new(format!("transport error: {value}"))
    }
}

impl From<tonic::Status> for Error {
    fn from(value: tonic::Status) -> Self {
        Self::new(format!("grpc status error: {value}"))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    pub browser_binary: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    pub timeout_ms: Option<u32>,
    pub interval_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chromium,
    Firefox,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigServer {
    pub(crate) addr: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigBrowser {
    pub(crate) name: Option<BrowserKind>,
    pub(crate) binary: Option<String>,
    pub(crate) launch_options: Option<LaunchOptions>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigWeb {
    pub(crate) browser: Option<ConfigBrowser>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigApp {
    pub(crate) id: Option<String>,
    pub(crate) binary: Option<String>,
    pub(crate) activity: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigMobileTarget {
    pub(crate) device: Option<String>,
    pub(crate) app: Option<ConfigApp>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigMobile {
    pub(crate) android: Option<ConfigMobileTarget>,
    pub(crate) ios: Option<ConfigMobileTarget>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigDesktopTarget {
    pub(crate) app: Option<ConfigApp>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigDesktop {
    pub(crate) mac: Option<ConfigDesktopTarget>,
    pub(crate) windows: Option<ConfigDesktopTarget>,
    pub(crate) linux: Option<ConfigDesktopTarget>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SuiteConfig {
    pub(crate) server: Option<ConfigServer>,
    pub(crate) web: Option<ConfigWeb>,
    pub(crate) mobile: Option<ConfigMobile>,
    pub(crate) desktop: Option<ConfigDesktop>,
    pub(crate) expect: Option<RetryConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllwrightConfig {
    pub(crate) schema_version: Option<u32>,
    pub(crate) server: Option<ConfigServer>,
    pub(crate) web: Option<ConfigWeb>,
    pub(crate) mobile: Option<ConfigMobile>,
    pub(crate) desktop: Option<ConfigDesktop>,
    pub(crate) expect: Option<RetryConfig>,
    pub(crate) suites: Option<std::collections::BTreeMap<String, SuiteConfig>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAppConfig {
    pub id: Option<String>,
    pub binary: Option<String>,
    pub activity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedMobileTargetConfig {
    pub device: Option<String>,
    pub app: Option<ResolvedAppConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedMobileConfig {
    pub android: Option<ResolvedMobileTargetConfig>,
    pub ios: Option<ResolvedMobileTargetConfig>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDesktopTargetConfig {
    pub app: Option<ResolvedAppConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedDesktopConfig {
    pub mac: Option<ResolvedDesktopTargetConfig>,
    pub windows: Option<ResolvedDesktopTargetConfig>,
    pub linux: Option<ResolvedDesktopTargetConfig>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub config_file_path: Option<PathBuf>,
    pub suite_name: Option<String>,
    pub server_addr: Option<String>,
    pub browser_name: Option<BrowserKind>,
    pub browser_binary: Option<String>,
    pub launch_options: LaunchOptions,
    pub expect: RetryConfig,
    pub mobile: ResolvedMobileConfig,
    pub desktop: ResolvedDesktopConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ResolveConfigOptions {
    pub cwd: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub suite: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserType {
    pub(crate) browser_kind: BrowserKind,
}

#[derive(Debug, Clone, Default)]
pub struct CommandOptions {
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NavigateResult {
    pub url: String,
    pub note: String,
    pub bidi_session_id: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
    pub package_version: String,
}

#[derive(Debug, Clone)]
pub struct ClickResult {
    pub selector: String,
    pub note: String,
    pub bidi_session_id: String,
}

#[derive(Debug, Clone)]
pub struct CountResult {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct HighlightOptions {
    pub timeout_ms: Option<u32>,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HighlightResult {
    pub selector: String,
    pub count: u32,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ElementResult {
    pub selector: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct FillResult {
    pub selector: String,
    pub value: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct PressResult {
    pub selector: String,
    pub key: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct TextResult {
    pub selector: String,
    pub text: String,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct PressOptions {
    pub timeout_ms: Option<u32>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WaitForSelectorOptions {
    pub timeout_ms: Option<u32>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct WaitForSelectorResult {
    pub selector: String,
    pub visible: bool,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub png_data: Vec<u8>,
    pub note: String,
}

#[derive(Clone)]
pub struct Browser {
    pub(crate) inner: Arc<BrowserInner>,
}

#[derive(Clone)]
pub struct Tab {
    pub(crate) inner: Arc<TabInner>,
}

pub type Page = Tab;

#[derive(Clone)]
pub struct Locator {
    pub(crate) page: Tab,
    pub(crate) selector: String,
}

#[derive(Clone)]
pub(crate) struct RuntimeClient {
    pub(crate) engine: EngineServiceClient<Channel>,
}

pub(crate) struct BrowserInner {
    pub(crate) runtime: Arc<RuntimeClient>,
    pub(crate) state: AsyncMutex<BrowserState>,
    pub(crate) session_id: String,
    pub(crate) browser_name: String,
    pub(crate) launch_note: String,
    pub(crate) cdp_websocket_url: String,
    pub(crate) user_data_dir: String,
    pub(crate) initial_tab: Tab,
}

pub(crate) struct BrowserState {
    pub(crate) command_tx: mpsc::Sender<crate::proto::SurfaceSessionCommand>,
    pub(crate) events: tonic::Streaming<crate::proto::SurfaceSessionEvent>,
    pub(crate) closed: bool,
}

pub(crate) struct TabInner {
    pub(crate) runtime: Arc<RuntimeClient>,
    pub(crate) surface_session_id: String,
    pub(crate) session_id: String,
    pub(crate) state: AsyncMutex<TabState>,
}

#[derive(Default)]
pub(crate) struct TabState {
    pub(crate) handle: Option<TabHandle>,
}

pub(crate) struct TabHandle {
    pub(crate) command_tx: mpsc::Sender<crate::proto::ContextSessionCommand>,
    pub(crate) events: tonic::Streaming<crate::proto::ContextSessionEvent>,
    pub(crate) closed: bool,
}
