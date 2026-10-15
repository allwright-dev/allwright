use allwright_surface_desktop::{
    ConnectOptions, DesktopAppSessionHandle, DesktopCommand, DesktopCommandResult, DesktopPlatform,
    DesktopSessionHandle, LaunchOptions,
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
    url: Option<&str>,
    listen_addr: SocketAddr,
    open_browser: bool,
) -> Result<(), Box<dyn Error>> {
    if surface != "desktop-windows" {
        return Err(format!(
            "inspector surface `{surface}` is not supported yet; expected `desktop-windows`"
        )
        .into());
    }
    if !listen_addr.ip().is_loopback() {
        return Err("the inspector must listen on a loopback address".into());
    }

    let runtime = InspectorRuntime::start(target, url)?;
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
    desktop: DesktopSessionHandle,
    app: DesktopAppSessionHandle,
}

impl InspectorRuntime {
    fn start(target: &str, url: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let connect = invoke(DesktopCommand::Connect(ConnectOptions {
            platform: DesktopPlatform::Windows,
            agent_endpoint: None,
            timeout_ms: Some(30_000),
        }))?;
        let DesktopCommandResult::Connect(connection) = connect else {
            return Err("desktop-windows returned an unexpected connect response".into());
        };

        let launch = invoke(DesktopCommand::LaunchApp {
            desktop_session: connection.desktop_session.clone(),
            options: LaunchOptions {
                app_id: target.to_string(),
                terminate_running: false,
                timeout_ms: Some(30_000),
            },
        })?;
        let DesktopCommandResult::LaunchApp(application) = launch else {
            return Err("desktop-windows returned an unexpected launch response".into());
        };

        if let Some(url) = url {
            invoke(DesktopCommand::NavigateApp {
                desktop_session: connection.desktop_session.clone(),
                app_session: application.app_session.clone(),
                url: url.to_string(),
                timeout_ms: Some(10_000),
            })?;
        }

        Ok(Self {
            desktop: connection.desktop_session,
            app: application.app_session,
        })
    }

    fn snapshot(&self) -> Result<Value, Box<dyn Error>> {
        let screenshot = invoke(DesktopCommand::Screenshot {
            desktop_session: self.desktop.clone(),
            app_session: self.app.clone(),
            timeout_ms: Some(10_000),
            full_page: false,
        })?;
        let DesktopCommandResult::Screenshot(screenshot) = screenshot else {
            return Err("desktop-windows returned an unexpected screenshot response".into());
        };

        let tree = invoke(DesktopCommand::AccessibilitySnapshot {
            desktop_session: self.desktop.clone(),
            app_session: self.app.clone(),
            format: "json".to_string(),
            mode: "default".to_string(),
        })?;
        let DesktopCommandResult::AccessibilitySnapshot(tree) = tree else {
            return Err("desktop-windows returned an unexpected accessibility response".into());
        };
        let tree: Value = serde_json::from_str(&tree.snapshot)?;
        Ok(json!({
            "image": format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(screenshot.png_data)
            ),
            "tree": tree,
        }))
    }

    fn click(&self, selector: String) -> Result<Value, Box<dyn Error>> {
        invoke(DesktopCommand::ClickElement {
            desktop_session: self.desktop.clone(),
            app_session: self.app.clone(),
            selector,
            timeout_ms: Some(10_000),
        })?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.snapshot()
    }
}

#[derive(Deserialize)]
struct PluginEnvelope {
    ok: bool,
    result: Option<DesktopCommandResult>,
    error: Option<String>,
}

fn invoke(command: DesktopCommand) -> Result<DesktopCommandResult, Box<dyn Error>> {
    let request = serde_json::to_string(&command)?;
    let response = allwright::plugins::invoke_plugin("desktop-windows", &request)
        .map_err(|error| format!("desktop-windows: {error}"))?;
    let response: PluginEnvelope = serde_json::from_str(&response)?;
    if response.ok {
        response
            .result
            .ok_or_else(|| "desktop-windows returned no result".into())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "desktop-windows failed without an error message".to_string())
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
        ("POST", "/api/click") => {
            let payload: Value = serde_json::from_slice(&request.body)?;
            let selector = payload
                .get("selector")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or("click requires a non-empty selector")?;
            let body = serde_json::to_vec(&runtime.click(selector.to_string())?)?;
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
        command.arg(url);
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
        assert!(INDEX_HTML.contains("/api/click"));
        assert!(INDEX_HTML.contains("Element hierarchy"));
    }

    #[test]
    fn accepts_only_loopback_browser_origins() {
        assert!(is_loopback_origin("http://127.0.0.1:9227"));
        assert!(is_loopback_origin("http://localhost:9227"));
        assert!(!is_loopback_origin("https://example.com"));
    }
}
