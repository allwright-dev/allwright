use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeLaunchInfo {
    pub browser: String,
    pub note: String,
    pub cdp_websocket_url: String,
    pub user_data_dir: String,
    pub process_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeTabInfo {
    pub note: String,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabNavigationInfo {
    pub url: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromiumBidiMapperInfo {
    pub package_version: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidiClickInfo {
    pub css_selector: String,
    pub note: String,
    pub package_version: String,
    pub mapper_target_id: String,
    pub mapper_session_id: String,
}

const CHROMIUM_BIDI_NPM_VERSION: &str = "17.0.2";
const CHROMIUM_BIDI_MAPPER_BUNDLE: &str =
    include_str!("../third_party/chromium-bidi/17.0.2/mapperTab.js");

pub fn open_chrome_window(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    launch_chrome_for_platform(chrome_binary.filter(|value| !value.trim().is_empty()))
}

pub async fn discover_initial_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;

    for _ in 0..100 {
        let targets = cdp
            .send_command("Target.getTargets", json!({}), None)
            .await?;
        if let Some(page_target) = targets
            .get("targetInfos")
            .and_then(Value::as_array)
            .and_then(|target_infos| {
                target_infos.iter().find(|target_info| {
                    target_info.get("type").and_then(Value::as_str) == Some("page")
                })
            })
        {
            let target_id = required_string(page_target, "/targetId")?;
            return Ok(ChromeTabInfo {
                note: "tracked initial Chrome tab via CDP".to_string(),
                target_id,
            });
        }

        sleep(Duration::from_millis(50)).await;
    }

    Err("timed out waiting for the initial Chrome page target over CDP".to_string())
}

pub async fn open_chrome_tab(cdp_websocket_url: &str) -> Result<ChromeTabInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let create_target = cdp
        .send_command(
            "Target.createTarget",
            json!({
                "url": "about:blank",
                "background": false,
            }),
            None,
        )
        .await?;
    let target_id = required_string(&create_target, "/targetId")?;

    Ok(ChromeTabInfo {
        note: "opened Chrome tab via CDP".to_string(),
        target_id,
    })
}

pub fn close_browser_process(process_id: u32) -> Result<(), String> {
    close_browser_process_for_platform(process_id)
}

pub async fn close_chrome_tab(cdp_websocket_url: &str, target_id: &str) -> Result<(), String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let response = cdp
        .send_command(
            "Target.closeTarget",
            json!({
                "targetId": target_id,
            }),
            None,
        )
        .await?;
    let success = response
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        Err(format!(
            "CDP Target.closeTarget reported unsuccessful close for target {target_id}"
        ))
    }
}

pub async fn navigate_chrome_tab(
    cdp_websocket_url: &str,
    target_id: &str,
    url: &str,
) -> Result<TabNavigationInfo, String> {
    if url.trim().is_empty() {
        return Err("navigate command requires a non-empty url".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let attach = cdp
        .send_command(
            "Target.attachToTarget",
            json!({
                "targetId": target_id,
                "flatten": true,
            }),
            None,
        )
        .await?;
    let session_id = required_string(&attach, "/sessionId")?;

    cdp.send_command("Page.enable", json!({}), Some(&session_id))
        .await?;
    cdp.navigate_and_wait_for_load(&session_id, url).await?;
    cdp.send_command(
        "Target.detachFromTarget",
        json!({
            "sessionId": session_id,
        }),
        None,
    )
    .await?;

    Ok(TabNavigationInfo {
        url: url.to_string(),
        note: "navigated Chrome tab via CDP and observed Page.loadEventFired".to_string(),
    })
}

pub async fn inject_chromium_bidi_mapper(
    cdp_websocket_url: &str,
) -> Result<ChromiumBidiMapperInfo, String> {
    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let mapper_target = cdp
        .send_command(
            "Target.createTarget",
            json!({
                "url": "about:blank#MAPPER_TARGET",
                "hidden": true,
                "background": true,
            }),
            None,
        )
        .await?;
    let mapper_target_id = required_string(&mapper_target, "/targetId")?;

    let attached = cdp
        .send_command(
            "Target.attachToTarget",
            json!({
                "targetId": mapper_target_id,
                "flatten": true,
            }),
            None,
        )
        .await?;
    let mapper_session_id = required_string(&attached, "/sessionId")?;

    cdp.send_command("Runtime.enable", json!({}), Some(&mapper_session_id))
        .await?;
    cdp.send_command(
        "Target.exposeDevToolsProtocol",
        json!({
            "bindingName": "cdp",
            "targetId": mapper_target_id,
            "inheritPermissions": true,
        }),
        None,
    )
    .await?;
    cdp.send_command(
        "Runtime.addBinding",
        json!({
            "name": "sendBidiResponse",
        }),
        Some(&mapper_session_id),
    )
    .await?;
    cdp.evaluate_expression(&mapper_session_id, CHROMIUM_BIDI_MAPPER_BUNDLE, false)
        .await?;
    cdp.evaluate_expression(
        &mapper_session_id,
        &format!(
            "window.runMapperInstance('{}')",
            js_single_quote(&mapper_target_id)
        ),
        true,
    )
    .await?;

    Ok(ChromiumBidiMapperInfo {
        package_version: CHROMIUM_BIDI_NPM_VERSION.to_string(),
        mapper_target_id,
        mapper_session_id,
        note: format!(
            "injected chromium-bidi mapper from pinned published chromium-bidi@{} artifact into hidden mapper target",
            CHROMIUM_BIDI_NPM_VERSION
        ),
    })
}

pub async fn click_element_via_bidi(
    cdp_websocket_url: &str,
    existing_mapper_target_id: Option<&str>,
    context_id: &str,
    css_selector: &str,
) -> Result<BidiClickInfo, String> {
    if css_selector.trim().is_empty() {
        return Err("click_element command requires a non-empty css_selector".to_string());
    }
    if context_id.trim().is_empty() {
        return Err("click_element command requires a non-empty context_id".to_string());
    }

    let mut cdp = CdpConnection::connect(cdp_websocket_url).await?;
    let mapper = cdp
        .ensure_chromium_bidi_mapper(existing_mapper_target_id)
        .await?;

    let selector_literal = serde_json::to_string(css_selector)
        .map_err(|error| format!("failed to serialize css selector for BiDi click: {error}"))?;
    let expression = format!(
        "(() => {{ const selector = {selector_literal}; const element = document.querySelector(selector); if (!element) {{ throw new Error(`No element matches selector: ${{selector}}`); }} element.click(); return {{ clicked: true, selector }}; }})()"
    );
    let bidi_command = json!({
        "id": 1,
        "method": "script.evaluate",
        "params": {
            "expression": expression,
            "target": {
                "context": context_id,
            },
            "awaitPromise": true,
            "resultOwnership": "none",
            "userActivation": true,
        }
    });
    cdp.send_bidi_command(&mapper.mapper_session_id, &bidi_command)
        .await?;

    Ok(BidiClickInfo {
        css_selector: css_selector.to_string(),
        note: format!("clicked element over WebDriver BiDi using css selector {css_selector}"),
        package_version: mapper.package_version,
        mapper_target_id: mapper.mapper_target_id,
        mapper_session_id: mapper.mapper_session_id,
    })
}

#[cfg(target_os = "macos")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    let browser_binary = chrome_binary.map(ToOwned::to_owned).unwrap_or_else(|| {
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string()
    });
    launch_chrome_with_cdp(&browser_binary)
}

#[cfg(target_os = "linux")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    if let Some(chrome_binary) = chrome_binary {
        return launch_chrome_with_cdp(chrome_binary);
    }

    let candidates = ["google-chrome", "chromium", "chromium-browser"];

    for candidate in candidates {
        match launch_chrome_with_cdp(candidate) {
            Ok(launch_info) => return Ok(launch_info),
            Err(error) => {
                if error.contains("No such file or directory")
                    || error.contains("os error 2")
                    || error.contains("not found")
                {
                    continue;
                }
                return Err(format!("failed to launch {candidate}: {error}"));
            }
        }
    }

    Err("could not find a Chrome-compatible browser binary".to_string())
}

#[cfg(target_os = "windows")]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    if let Some(chrome_binary) = chrome_binary {
        return launch_chrome_with_cdp(chrome_binary);
    }

    launch_chrome_with_cdp("chrome")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn launch_chrome_for_platform(chrome_binary: Option<&str>) -> Result<ChromeLaunchInfo, String> {
    match chrome_binary {
        Some(chrome_binary) => launch_chrome_with_cdp(chrome_binary),
        None => Err("Chrome launching is not supported on this platform yet".to_string()),
    }
}

fn launch_chrome_with_cdp(browser_binary: &str) -> Result<ChromeLaunchInfo, String> {
    let user_data_dir = create_chrome_user_data_dir()?;
    let user_data_dir_str = user_data_dir.to_string_lossy().to_string();

    let child = Command::new(browser_binary)
        .args([
            "--new-window",
            "--remote-debugging-port=0",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-sync",
            &format!("--user-data-dir={user_data_dir_str}"),
        ])
        .spawn()
        .map_err(|error| format!("failed to launch Chrome binary {browser_binary}: {error}"))?;
    let process_id = child.id();

    let cdp_websocket_url = wait_for_cdp_endpoint(&user_data_dir)?;

    Ok(ChromeLaunchInfo {
        browser: browser_binary.to_string(),
        note: "launched Chrome window with CDP enabled".to_string(),
        cdp_websocket_url,
        user_data_dir: user_data_dir_str,
        process_id,
    })
}

fn create_chrome_user_data_dir() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute timestamp: {error}"))?
        .as_millis();
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("allwright-chrome-{pid}-{timestamp}"));
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create Chrome user data dir {}: {error}",
            dir.display()
        )
    })?;
    Ok(dir)
}

fn wait_for_cdp_endpoint(user_data_dir: &std::path::Path) -> Result<String, String> {
    let active_port_file = user_data_dir.join("DevToolsActivePort");

    for _ in 0..100 {
        if active_port_file.exists() {
            let contents = fs::read_to_string(&active_port_file).map_err(|error| {
                format!(
                    "failed to read DevToolsActivePort file {}: {error}",
                    active_port_file.display()
                )
            })?;
            return parse_devtools_active_port(&contents);
        }
        thread::sleep(StdDuration::from_millis(50));
    }

    Err(format!(
        "timed out waiting for DevToolsActivePort in {}",
        user_data_dir.display()
    ))
}

fn parse_devtools_active_port(contents: &str) -> Result<String, String> {
    let mut lines = contents.lines();
    let port = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "DevToolsActivePort missing port line".to_string())?;
    let browser_path = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "DevToolsActivePort missing websocket path line".to_string())?;

    Ok(format!("ws://127.0.0.1:{port}{browser_path}"))
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(15)).await;
    "web ready".to_string()
}

#[cfg(unix)]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    let status = Command::new("kill")
        .args(["-TERM", &process_id.to_string()])
        .status()
        .map_err(|error| format!("failed to send TERM to process {process_id}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "kill -TERM returned non-zero exit status for process {process_id}: {status}"
        ))
    }
}

#[cfg(windows)]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("failed to terminate process {process_id}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "taskkill returned non-zero exit status for process {process_id}: {status}"
        ))
    }
}

#[cfg(not(any(unix, windows)))]
fn close_browser_process_for_platform(process_id: u32) -> Result<(), String> {
    Err(format!(
        "browser process termination is not implemented for process {process_id} on this platform"
    ))
}

type CdpSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct MapperConnectionInfo {
    package_version: String,
    mapper_target_id: String,
    mapper_session_id: String,
}

struct CdpConnection {
    socket: CdpSocket,
    next_id: u64,
}

impl CdpConnection {
    async fn connect(cdp_websocket_url: &str) -> Result<Self, String> {
        let (socket, _) = connect_async(cdp_websocket_url).await.map_err(|error| {
            format!("failed to connect to CDP websocket {cdp_websocket_url}: {error}")
        })?;
        Ok(Self { socket, next_id: 1 })
    }

    async fn send_command(
        &mut self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let mut payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(session_id) = session_id {
            payload["sessionId"] = Value::String(session_id.to_string());
        }

        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send CDP command {method}: {error}"))?;

        loop {
            let message = self.next_json_message().await?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error") {
                return Err(format!("CDP command {method} failed: {error}"));
            }

            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn evaluate_expression(
        &mut self,
        session_id: &str,
        expression: &str,
        await_promise: bool,
    ) -> Result<Value, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "awaitPromise": await_promise,
                }),
                Some(session_id),
            )
            .await?;
        if let Some(exception_details) = result.get("exceptionDetails") {
            return Err(format!(
                "Runtime.evaluate failed with exception details: {exception_details}"
            ));
        }
        Ok(result)
    }

    async fn ensure_chromium_bidi_mapper(
        &mut self,
        existing_mapper_target_id: Option<&str>,
    ) -> Result<MapperConnectionInfo, String> {
        let (mapper_target_id, created_target) = match existing_mapper_target_id {
            Some(existing_mapper_target_id) if !existing_mapper_target_id.trim().is_empty() => {
                (existing_mapper_target_id.to_string(), false)
            }
            _ => {
                let created = self
                    .send_command(
                        "Target.createTarget",
                        json!({
                            "url": "about:blank#MAPPER_TARGET",
                            "hidden": true,
                            "background": true,
                        }),
                        None,
                    )
                    .await?;
                (required_string(&created, "/targetId")?, true)
            }
        };

        let attached = self
            .send_command(
                "Target.attachToTarget",
                json!({
                    "targetId": mapper_target_id,
                    "flatten": true,
                }),
                None,
            )
            .await?;
        let mapper_session_id = required_string(&attached, "/sessionId")?;

        self.send_command("Runtime.enable", json!({}), Some(&mapper_session_id))
            .await?;
        self.send_command(
            "Target.exposeDevToolsProtocol",
            json!({
                "bindingName": "cdp",
                "targetId": mapper_target_id,
                "inheritPermissions": true,
            }),
            None,
        )
        .await?;
        self.ensure_runtime_binding(&mapper_session_id, "sendBidiResponse")
            .await?;

        if created_target
            || !self
                .mapper_runtime_is_initialized(&mapper_session_id)
                .await?
        {
            self.evaluate_expression(&mapper_session_id, CHROMIUM_BIDI_MAPPER_BUNDLE, false)
                .await?;
            self.evaluate_expression(
                &mapper_session_id,
                &format!(
                    "window.runMapperInstance('{}')",
                    js_single_quote(&mapper_target_id)
                ),
                true,
            )
            .await?;
        }

        Ok(MapperConnectionInfo {
            package_version: CHROMIUM_BIDI_NPM_VERSION.to_string(),
            mapper_target_id,
            mapper_session_id,
        })
    }

    async fn ensure_runtime_binding(&mut self, session_id: &str, name: &str) -> Result<(), String> {
        match self
            .send_command(
                "Runtime.addBinding",
                json!({
                    "name": name,
                }),
                Some(session_id),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(error)
                if error.contains("already exists") || error.contains("Cannot add binding") =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn mapper_runtime_is_initialized(&mut self, session_id: &str) -> Result<bool, String> {
        let result = self
            .evaluate_expression(
                session_id,
                "typeof window.runMapperInstance === 'function' && typeof window.onBidiMessage === 'function'",
                true,
            )
            .await?;
        Ok(result
            .pointer("/result/value")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    async fn send_bidi_command(
        &mut self,
        mapper_session_id: &str,
        command: &Value,
    ) -> Result<Value, String> {
        let command_id = command
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "BiDi command is missing numeric id".to_string())?;
        let eval_id = self.next_id;
        self.next_id += 1;
        let command_json = command.to_string();
        let expression = format!(
            "onBidiMessage({})",
            serde_json::to_string(&command_json)
                .map_err(|error| format!("failed to encode BiDi command payload: {error}"))?
        );

        let payload = json!({
            "id": eval_id,
            "method": "Runtime.evaluate",
            "params": {
                "expression": expression,
                "awaitPromise": true,
            },
            "sessionId": mapper_session_id,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send mapper Runtime.evaluate call: {error}"))?;

        loop {
            let message = self.next_json_message().await?;

            if message.get("id").and_then(Value::as_u64) == Some(eval_id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("mapper Runtime.evaluate failed: {error}"));
                }
                if let Some(exception_details) = message.pointer("/result/exceptionDetails") {
                    return Err(format!(
                        "mapper Runtime.evaluate raised exception details: {exception_details}"
                    ));
                }
                continue;
            }

            if message.get("sessionId").and_then(Value::as_str) != Some(mapper_session_id) {
                continue;
            }

            if message.get("method").and_then(Value::as_str) != Some("Runtime.bindingCalled") {
                continue;
            }

            let params = message
                .get("params")
                .ok_or_else(|| "Runtime.bindingCalled message is missing params".to_string())?;
            if params.get("name").and_then(Value::as_str) != Some("sendBidiResponse") {
                continue;
            }

            let payload = params
                .get("payload")
                .and_then(Value::as_str)
                .ok_or_else(|| "Runtime.bindingCalled payload is missing".to_string())?;
            let response = serde_json::from_str::<Value>(payload)
                .map_err(|error| format!("failed to parse BiDi mapper response JSON: {error}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(command_id) {
                continue;
            }

            match response.get("type").and_then(Value::as_str) {
                Some("success") => return Ok(response),
                Some("error") => {
                    let message = response
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown BiDi mapper error");
                    let error = response
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    return Err(format!("BiDi mapper returned {error}: {message}"));
                }
                Some(other) => {
                    return Err(format!("unexpected BiDi response type {other}: {response}"));
                }
                None => {
                    return Err(format!("BiDi mapper response is missing type: {response}"));
                }
            }
        }
    }

    async fn navigate_and_wait_for_load(
        &mut self,
        session_id: &str,
        url: &str,
    ) -> Result<(), String> {
        let id = self.next_id;
        self.next_id += 1;

        let payload = json!({
            "id": id,
            "method": "Page.navigate",
            "params": {
                "url": url,
            },
            "sessionId": session_id,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .map_err(|error| format!("failed to send CDP command Page.navigate: {error}"))?;

        let mut navigate_confirmed = false;
        let mut load_seen = false;

        loop {
            let message = self.next_json_message().await?;

            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("CDP command Page.navigate failed: {error}"));
                }
                navigate_confirmed = true;
            } else if message.get("sessionId").and_then(Value::as_str) == Some(session_id)
                && message.get("method").and_then(Value::as_str) == Some("Page.loadEventFired")
            {
                load_seen = true;
            }

            if navigate_confirmed && load_seen {
                return Ok(());
            }
        }
    }

    async fn next_json_message(&mut self) -> Result<Value, String> {
        loop {
            let message = self
                .socket
                .next()
                .await
                .ok_or_else(|| "CDP websocket closed unexpectedly".to_string())?
                .map_err(|error| format!("failed to read CDP websocket message: {error}"))?;

            match message {
                Message::Text(text) => {
                    let value = serde_json::from_str::<Value>(&text)
                        .map_err(|error| format!("failed to parse CDP JSON message: {error}"))?;
                    return Ok(value);
                }
                Message::Binary(bytes) => {
                    let value = serde_json::from_slice::<Value>(&bytes)
                        .map_err(|error| format!("failed to parse CDP JSON message: {error}"))?;
                    return Ok(value);
                }
                Message::Ping(payload) => {
                    self.socket
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|error| format!("failed to reply to CDP ping: {error}"))?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Close(frame) => {
                    return Err(format!("CDP websocket closed: {frame:?}"));
                }
            }
        }
    }
}

fn required_string(value: &Value, pointer: &str) -> Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing string field at JSON pointer {pointer}"))
}

fn js_single_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}
