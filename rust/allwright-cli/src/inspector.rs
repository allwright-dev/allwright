use allwright_surface_desktop::{
    ConnectOptions as DesktopConnectOptions, DesktopAppSessionHandle, DesktopCommand,
    DesktopCommandResult, DesktopPlatform, DesktopSessionHandle,
    LaunchOptions as DesktopLaunchOptions,
};
use allwright_surface_mobile::{
    ConnectOptions as MobileConnectOptions, LaunchOptions as MobileLaunchOptions,
    MobileBrowserSessionHandle, MobileCommand, MobileCommandResult, MobilePageSessionHandle,
    MobilePlatform,
};
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Command, Stdio};

const INDEX_HTML: &str = include_str!("inspector.html");

pub fn run(
    surface: &str,
    target: &str,
    device: Option<&str>,
    url: Option<&str>,
    listen_addr: SocketAddr,
    open_browser: bool,
) -> Result<(), Box<dyn Error>> {
    let surface = InspectorSurface::parse(surface)?;
    if !listen_addr.ip().is_loopback() {
        return Err("the inspector must listen on a loopback address".into());
    }

    let runtime = InspectorRuntime::start(surface, target, device, url)?;
    let listener = TcpListener::bind(listen_addr)?;
    let address = listener.local_addr()?;
    let inspector_url = format!("http://{address}/");
    println!("Inspecting `{target}` at {inspector_url}");
    println!("Press Ctrl+C to stop the inspector.");
    if open_browser {
        open_in_browser(&inspector_url)?;
    }

    for connection in listener.incoming() {
        match connection {
            Ok(mut stream) => {
                if let Err(error) = handle_request(&runtime, &mut stream) {
                    let _ = write_response(
                        &mut stream,
                        "500 Internal Server Error",
                        "application/json; charset=utf-8",
                        json!({"error": error.to_string()}).to_string().as_bytes(),
                    );
                }
            }
            Err(error) => eprintln!("inspector connection failed: {error}"),
        }
    }
    Ok(())
}

struct InspectorRuntime {
    surface: InspectorSurface,
    session: InspectorSession,
}

#[derive(Clone, Copy)]
enum InspectorSurface {
    Desktop {
        id: &'static str,
        platform: DesktopPlatform,
    },
    MobileAndroid,
    MobileIos,
}

enum InspectorSession {
    Desktop {
        desktop: DesktopSessionHandle,
        app: DesktopAppSessionHandle,
    },
    Mobile {
        browser: MobileBrowserSessionHandle,
        page: MobilePageSessionHandle,
    },
}

impl InspectorSurface {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "desktop-windows" => Ok(Self::Desktop {
                id: "desktop-windows",
                platform: DesktopPlatform::Windows,
            }),
            "desktop-mac" => Ok(Self::Desktop {
                id: "desktop-mac",
                platform: DesktopPlatform::Mac,
            }),
            "mobile-android" => Ok(Self::MobileAndroid),
            "mobile-ios" => Ok(Self::MobileIos),
            _ => Err(format!(
                "inspector surface `{value}` is not supported; expected `desktop-windows`, `desktop-mac`, `mobile-android`, or `mobile-ios`"
            )
            .into()),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Desktop { id, .. } => id,
            Self::MobileAndroid => "mobile-android",
            Self::MobileIos => "mobile-ios",
        }
    }
}

impl InspectorRuntime {
    fn start(
        surface: InspectorSurface,
        target: &str,
        device: Option<&str>,
        url: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
        let session = match surface {
            InspectorSurface::Desktop { id, platform } => {
                if device.is_some() {
                    return Err("--device is only supported by mobile inspectors".into());
                }
                let connect = invoke_desktop(
                    id,
                    DesktopCommand::Connect(DesktopConnectOptions {
                        platform,
                        agent_endpoint: None,
                        timeout_ms: Some(30_000),
                    }),
                )?;
                let DesktopCommandResult::Connect(connection) = connect else {
                    return Err(format!("{id} returned an unexpected connect response").into());
                };
                let launch = invoke_desktop(
                    id,
                    DesktopCommand::LaunchApp {
                        desktop_session: connection.desktop_session.clone(),
                        options: DesktopLaunchOptions {
                            app_id: target.to_string(),
                            terminate_running: false,
                            timeout_ms: Some(30_000),
                        },
                    },
                )?;
                let DesktopCommandResult::LaunchApp(application) = launch else {
                    return Err(format!("{id} returned an unexpected launch response").into());
                };
                if let Some(url) = url {
                    invoke_desktop(
                        id,
                        DesktopCommand::NavigateApp {
                            desktop_session: connection.desktop_session.clone(),
                            app_session: application.app_session.clone(),
                            url: url.to_string(),
                            timeout_ms: Some(10_000),
                        },
                    )?;
                }
                InspectorSession::Desktop {
                    desktop: connection.desktop_session,
                    app: application.app_session,
                }
            }
            InspectorSurface::MobileAndroid | InspectorSurface::MobileIos => {
                let plugin_id = surface.id();
                let platform = match surface {
                    InspectorSurface::MobileAndroid => MobilePlatform::Android,
                    InspectorSurface::MobileIos => MobilePlatform::Ios,
                    InspectorSurface::Desktop { .. } => unreachable!(),
                };
                let connect = invoke_mobile(
                    plugin_id,
                    MobileCommand::Connect(MobileConnectOptions {
                        platform,
                        device: device.map(str::to_string),
                        adb_endpoint: None,
                        agent_endpoint: None,
                        preserve_app_state: false,
                        timeout_ms: Some(60_000),
                    }),
                )?;
                let MobileCommandResult::Connect(connection) = connect else {
                    return Err(
                        format!("{plugin_id} returned an unexpected connect response").into(),
                    );
                };
                let launch = invoke_mobile(
                    plugin_id,
                    MobileCommand::LaunchApp {
                        browser_session: connection.browser_session.clone(),
                        options: MobileLaunchOptions {
                            apk_path: Some(target.to_string()),
                            app_id: None,
                            launch_activity: None,
                            stop_before_launch: true,
                            timeout_ms: Some(120_000),
                        },
                    },
                )?;
                let MobileCommandResult::LaunchApp(application) = launch else {
                    return Err(
                        format!("{plugin_id} returned an unexpected launch response").into(),
                    );
                };
                if let Some(url) = url {
                    invoke_mobile(
                        plugin_id,
                        MobileCommand::NavigateApp {
                            browser_session: connection.browser_session.clone(),
                            page_session: application.page_session.clone(),
                            url: url.to_string(),
                            timeout_ms: Some(10_000),
                        },
                    )?;
                }
                InspectorSession::Mobile {
                    browser: connection.browser_session,
                    page: application.page_session,
                }
            }
        };
        Ok(Self { surface, session })
    }

    fn snapshot(&self) -> Result<Value, Box<dyn Error>> {
        let png_data = self.screenshot()?;
        let tree = match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                let tree = invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::InspectorSnapshot {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                    },
                )?;
                let DesktopCommandResult::InspectorSnapshot(tree) = tree else {
                    return Err(format!(
                        "{} returned an unexpected inspector snapshot response",
                        self.surface.id()
                    )
                    .into());
                };
                tree
            }
            InspectorSession::Mobile { browser, page } => {
                let tree = invoke_mobile(
                    self.surface.id(),
                    MobileCommand::InspectorSnapshot {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                    },
                )?;
                let MobileCommandResult::InspectorSnapshot(tree) = tree else {
                    return Err(format!(
                        "{} returned an unexpected inspector snapshot response",
                        self.surface.id()
                    )
                    .into());
                };
                tree
            }
        };
        let backend = tree.backend.clone();
        Ok(json!({
            "image": format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(png_data)
            ),
            "tree": tree,
            "surface": self.surface.id(),
            "backend": backend,
        }))
    }

    fn screenshot(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                let result = invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::Screenshot {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        timeout_ms: Some(10_000),
                        full_page: false,
                    },
                )?;
                let DesktopCommandResult::Screenshot(screenshot) = result else {
                    return Err(format!(
                        "{} returned an unexpected screenshot response",
                        self.surface.id()
                    )
                    .into());
                };
                Ok(screenshot.png_data)
            }
            InspectorSession::Mobile { browser, page } => {
                let result = invoke_mobile(
                    self.surface.id(),
                    MobileCommand::Screenshot {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        timeout_ms: Some(30_000),
                        full_page: false,
                    },
                )?;
                let MobileCommandResult::Screenshot(screenshot) = result else {
                    return Err(format!(
                        "{} returned an unexpected screenshot response",
                        self.surface.id()
                    )
                    .into());
                };
                Ok(screenshot.png_data)
            }
        }
    }

    fn live_input(&self, input: InspectorInputRequest) -> Result<(), Box<dyn Error>> {
        if !matches!(input.kind.as_str(), "pointer" | "keyboard") {
            return Err(format!("unsupported live input kind `{}`", input.kind).into());
        }
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                let command = if input.kind == "keyboard" {
                    DesktopCommand::KeyboardInput {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        key: input.key,
                        text: input.text,
                    }
                } else {
                    DesktopCommand::PointerInput {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        action: input.action.ok_or("pointer input requires an action")?,
                        x: input.x.ok_or("pointer input requires x")?,
                        y: input.y.ok_or("pointer input requires y")?,
                        to_x: input.to_x,
                        to_y: input.to_y,
                        delta_x: input.delta_x,
                        delta_y: input.delta_y,
                        duration_ms: input.duration_ms,
                    }
                };
                invoke_desktop(self.surface.id(), command)?;
            }
            InspectorSession::Mobile { browser, page } => {
                let command = if input.kind == "keyboard" {
                    MobileCommand::KeyboardInput {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        key: input.key,
                        text: input.text,
                    }
                } else {
                    MobileCommand::PointerInput {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        action: input.action.ok_or("pointer input requires an action")?,
                        x: input.x.ok_or("pointer input requires x")?,
                        y: input.y.ok_or("pointer input requires y")?,
                        to_x: input.to_x,
                        to_y: input.to_y,
                        delta_x: input.delta_x,
                        delta_y: input.delta_y,
                        duration_ms: input.duration_ms,
                    }
                };
                invoke_mobile(self.surface.id(), command)?;
            }
        }
        Ok(())
    }

    fn action(
        &self,
        action: &str,
        selector: String,
        value: Option<String>,
        key: Option<String>,
        text: Option<String>,
        direction: Option<String>,
    ) -> Result<Value, Box<dyn Error>> {
        match action {
            "click" => self.click_element(&selector)?,
            "double_click" => {
                self.click_element(&selector)?;
                std::thread::sleep(std::time::Duration::from_millis(75));
                self.click_element(&selector)?;
            }
            "fill" => {
                self.fill_element(&selector, value.as_deref().ok_or("fill requires a value")?)?
            }
            "press" => self.press_key(
                &selector,
                key.as_deref()
                    .filter(|key| !key.trim().is_empty())
                    .ok_or("press requires a non-empty key")?,
                text.as_deref(),
            )?,
            "scroll" => self.scroll_element(
                &selector,
                direction
                    .as_deref()
                    .filter(|direction| matches!(*direction, "up" | "down" | "left" | "right"))
                    .ok_or("scroll requires an up, down, left, or right direction")?,
            )?,
            _ => return Err(format!("unsupported inspector action `{action}`").into()),
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.snapshot()
    }

    fn click_element(&self, selector: &str) -> Result<(), Box<dyn Error>> {
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::ClickElement {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        selector: selector.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
            InspectorSession::Mobile { browser, page } => {
                invoke_mobile(
                    self.surface.id(),
                    MobileCommand::ClickElement {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        selector: selector.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
        }
        Ok(())
    }

    fn fill_element(&self, selector: &str, value: &str) -> Result<(), Box<dyn Error>> {
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::FillElement {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        selector: selector.to_string(),
                        value: value.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
            InspectorSession::Mobile { browser, page } => {
                invoke_mobile(
                    self.surface.id(),
                    MobileCommand::FillElement {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        selector: selector.to_string(),
                        value: value.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
        }
        Ok(())
    }

    fn press_key(
        &self,
        selector: &str,
        key: &str,
        text: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::PressKey {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        selector: selector.to_string(),
                        key: key.to_string(),
                        text: text.map(str::to_string),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
            InspectorSession::Mobile { browser, page } => {
                invoke_mobile(
                    self.surface.id(),
                    MobileCommand::PressKey {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        selector: selector.to_string(),
                        key: key.to_string(),
                        text: text.map(str::to_string),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
        }
        Ok(())
    }

    fn scroll_element(&self, selector: &str, direction: &str) -> Result<(), Box<dyn Error>> {
        match &self.session {
            InspectorSession::Desktop { desktop, app } => {
                invoke_desktop(
                    self.surface.id(),
                    DesktopCommand::ScrollElement {
                        desktop_session: desktop.clone(),
                        app_session: app.clone(),
                        selector: selector.to_string(),
                        direction: direction.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
            InspectorSession::Mobile { browser, page } => {
                invoke_mobile(
                    self.surface.id(),
                    MobileCommand::ScrollElement {
                        browser_session: browser.clone(),
                        page_session: page.clone(),
                        selector: selector.to_string(),
                        direction: direction.to_string(),
                        timeout_ms: Some(10_000),
                    },
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct DesktopPluginEnvelope {
    ok: bool,
    result: Option<DesktopCommandResult>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct MobilePluginEnvelope {
    ok: bool,
    result: Option<MobileCommandResult>,
    error: Option<String>,
}

fn invoke_desktop(
    plugin_id: &str,
    command: DesktopCommand,
) -> Result<DesktopCommandResult, Box<dyn Error>> {
    let request = serde_json::to_string(&command)?;
    let response = allwright::plugins::invoke_plugin(plugin_id, &request)
        .map_err(|error| format!("{plugin_id}: {error}"))?;
    let response: DesktopPluginEnvelope = serde_json::from_str(&response)?;
    if response.ok {
        response
            .result
            .ok_or_else(|| format!("{plugin_id} returned no result").into())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| format!("{plugin_id} failed without an error message"))
            .into())
    }
}

fn invoke_mobile(
    plugin_id: &str,
    command: MobileCommand,
) -> Result<MobileCommandResult, Box<dyn Error>> {
    let request = serde_json::to_string(&command)?;
    let response = allwright::plugins::invoke_plugin(plugin_id, &request)
        .map_err(|error| format!("{plugin_id}: {error}"))?;
    let response: MobilePluginEnvelope = serde_json::from_str(&response)?;
    if response.ok {
        response
            .result
            .ok_or_else(|| format!("{plugin_id} returned no result").into())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| format!("{plugin_id} failed without an error message"))
            .into())
    }
}

fn handle_request(
    runtime: &InspectorRuntime,
    stream: &mut TcpStream,
) -> Result<(), Box<dyn Error>> {
    let request = read_request(stream)?;
    if request
        .origin
        .as_deref()
        .is_some_and(|origin| !is_loopback_origin(origin))
    {
        return Err("cross-origin inspector request was rejected".into());
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => write_response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML.as_bytes(),
        )?,
        ("GET", "/api/snapshot") => {
            let body = serde_json::to_vec(&runtime.snapshot()?)?;
            write_response(stream, "200 OK", "application/json", &body)?;
        }
        ("GET", "/api/frame") => {
            let png = runtime.screenshot()?;
            write_response(stream, "200 OK", "image/png", &png)?;
        }
        ("POST", "/api/input") => {
            let payload: InspectorInputRequest = serde_json::from_slice(&request.body)?;
            runtime.live_input(payload)?;
            write_response(stream, "200 OK", "application/json", br#"{"ok":true}"#)?;
        }
        ("POST", "/api/click") => {
            let payload: Value = serde_json::from_slice(&request.body)?;
            let selector = payload
                .get("selector")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or("click requires a non-empty selector")?;
            let body = serde_json::to_vec(&runtime.action(
                "click",
                selector.to_string(),
                None,
                None,
                None,
                None,
            )?)?;
            write_response(stream, "200 OK", "application/json", &body)?;
        }
        ("POST", "/api/action") => {
            let payload: InspectorActionRequest = serde_json::from_slice(&request.body)?;
            if payload.selector.trim().is_empty() {
                return Err("inspector action requires a non-empty selector".into());
            }
            let body = serde_json::to_vec(&runtime.action(
                &payload.action,
                payload.selector,
                payload.value,
                payload.key,
                payload.text,
                payload.direction,
            )?)?;
            write_response(stream, "200 OK", "application/json", &body)?;
        }
        _ => write_response(
            stream,
            "404 Not Found",
            "application/json",
            br#"{"error":"not found"}"#,
        )?,
    }
    Ok(())
}

#[derive(Deserialize)]
struct InspectorActionRequest {
    action: String,
    selector: String,
    value: Option<String>,
    key: Option<String>,
    text: Option<String>,
    direction: Option<String>,
}

#[derive(Deserialize)]
struct InspectorInputRequest {
    kind: String,
    action: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    to_x: Option<f64>,
    to_y: Option<f64>,
    delta_x: Option<f64>,
    delta_y: Option<f64>,
    duration_ms: Option<u32>,
    key: Option<String>,
    text: Option<String>,
}

struct HttpRequest {
    method: String,
    path: String,
    origin: Option<String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, Box<dyn Error>> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let (header_end, content_length) = loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("incomplete HTTP request".into());
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > 1024 * 1024 {
            return Err("HTTP request is too large".into());
        }
        if let Some(end) = find_bytes(&bytes, b"\r\n\r\n") {
            let headers = std::str::from_utf8(&bytes[..end])?;
            let length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                })
                .unwrap_or(0);
            break (end + 4, length);
        }
    };
    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("incomplete HTTP request body".into());
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let headers = std::str::from_utf8(&bytes[..header_end - 4])?;
    let request_line = headers.lines().next().ok_or("missing HTTP request line")?;
    let origin = headers.lines().skip(1).find_map(|line| {
        line.split_once(':').and_then(|(name, value)| {
            name.eq_ignore_ascii_case("origin")
                .then(|| value.trim().to_string())
        })
    });
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing HTTP method")?.to_string();
    let path = parts
        .next()
        .ok_or("missing HTTP path")?
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();
    Ok(HttpRequest {
        method,
        path,
        origin,
        body: bytes[header_end..header_end + content_length].to_vec(),
    })
}

fn is_loopback_origin(origin: &str) -> bool {
    ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
        .iter()
        .any(|prefix| origin.starts_with(prefix))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

fn open_in_browser(url: &str) -> Result<(), Box<dyn Error>> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        // A separate app window prevents a URL target and the inspector from
        // replacing one another in the user's default browser tab.
        command.args(["/C", "start", "", "msedge", &format!("--app={url}")]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        // Keep the inspector out of an already-running browser window. This is
        // important when the inspected native application is itself Safari.
        command.args(["-n", "-a", "Safari", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_http_header_boundary() {
        assert_eq!(find_bytes(b"GET / HTTP/1.1\r\n\r\n", b"\r\n\r\n"), Some(14));
    }

    #[test]
    fn inspector_html_has_core_workflow() {
        assert!(INDEX_HTML.contains("/api/snapshot"));
        assert!(INDEX_HTML.contains("/api/action"));
        assert!(INDEX_HTML.contains("Element hierarchy"));
        assert!(INDEX_HTML.contains(">Inspect</button>"));
        assert!(INDEX_HTML.contains(">Interact</button>"));
        assert!(INDEX_HTML.contains("double_click"));
        assert!(INDEX_HTML.contains("wheelInteract"));
        assert!(INDEX_HTML.contains("pointerFinish"));
        assert!(INDEX_HTML.contains("/api/frame"));
        assert!(INDEX_HTML.contains("/api/input"));
        assert!(INDEX_HTML.contains("streamFrames"));
        assert!(INDEX_HTML.contains("keyboardInteract"));
        assert!(INDEX_HTML.contains("Text to fill"));
        assert!(INDEX_HTML.contains("Press key"));
        assert!(INDEX_HTML.contains("app.getByRole"));
        assert!(INDEX_HTML.contains("app.get_by_role"));
        assert!(INDEX_HTML.contains("app.GetByRole"));
        assert!(INDEX_HTML.contains("app.get_by_role_with_options"));
        assert!(INDEX_HTML.contains("object-fit:contain"));
        assert!(INDEX_HTML.contains("function fitStage()"));
        assert!(INDEX_HTML.contains("window.addEventListener('resize',fitStage)"));
    }

    #[test]
    fn parses_inspector_interaction_actions() {
        let action: InspectorActionRequest = serde_json::from_str(
            r#"{"action":"fill","selector":"id=email","value":"person@example.com"}"#,
        )
        .unwrap();
        assert_eq!(action.action, "fill");
        assert_eq!(action.selector, "id=email");
        assert_eq!(action.value.as_deref(), Some("person@example.com"));
        assert!(action.key.is_none());
        assert!(action.direction.is_none());
    }

    #[test]
    fn parses_live_pointer_and_keyboard_input() {
        let pointer: InspectorInputRequest = serde_json::from_str(
            r#"{"kind":"pointer","action":"swipe","x":10.5,"y":20,"to_x":30,"to_y":40,"duration_ms":250}"#,
        )
        .unwrap();
        assert_eq!(pointer.kind, "pointer");
        assert_eq!(pointer.action.as_deref(), Some("swipe"));
        assert_eq!(pointer.x, Some(10.5));
        assert_eq!(pointer.to_y, Some(40.0));

        let keyboard: InspectorInputRequest =
            serde_json::from_str(r#"{"kind":"keyboard","text":"hello"}"#).unwrap();
        assert_eq!(keyboard.kind, "keyboard");
        assert_eq!(keyboard.text.as_deref(), Some("hello"));
        assert!(keyboard.key.is_none());
    }

    #[test]
    fn accepts_only_loopback_browser_origins() {
        assert!(is_loopback_origin("http://127.0.0.1:9227"));
        assert!(is_loopback_origin("http://localhost:9227"));
        assert!(!is_loopback_origin("https://example.com"));
    }
}
