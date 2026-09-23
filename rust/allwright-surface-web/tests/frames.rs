use allwright_plugin_sdk::BrowserKind;
use allwright_surface_web::{
    click_element, close_browser_process, count_elements, fill_element, get_text_content,
    launch_browser, navigate_page,
};

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

struct Fixture {
    url: String,
    stop: Arc<AtomicBool>,
}
impl Fixture {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let n = stream.read(&mut request).unwrap_or(0);
                if n == 0 {
                    continue;
                }
                let request = String::from_utf8_lossy(&request[..n]);
                let body = if request.starts_with("GET /slow ") {
                    std::thread::sleep(Duration::from_millis(800));
                    "<p>Loaded</p>".to_string()
                } else if request.starts_with("GET /frame ") {
                    r#"<title>Frame</title><input><button onclick="this.textContent='Clicked'; const frame=document.createElement('iframe'); frame.id='slow'; frame.src='/slow'; document.body.append(frame)">Cross origin frame action</button><iframe srcdoc="<p>Nested</p>"></iframe>"#.to_string()
                } else {
                    format!(
                        r#"<iframe id="child" src="http://localhost:{port}/frame"></iframe><iframe id="unstable" srcdoc="<body><script>setInterval(() => document.body.setAttribute('data-tick', Date.now()), 20)</script>"></iframe>"#
                    )
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}/"),
            stop,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
struct BrowserProcess(u32);
impl Drop for BrowserProcess {
    fn drop(&mut self) {
        let pid = self.0;
        let _ = std::thread::spawn(move || close_browser_process(pid)).join();
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires installed Chromium/Firefox"]
async fn frames_are_independent_pages() {
    let kind = if std::env::var("ALLWRIGHT_TEST_BROWSER").as_deref() == Ok("firefox") {
        BrowserKind::Firefox
    } else {
        BrowserKind::Chromium
    };
    let browser = tokio::task::spawn_blocking(move || launch_browser(kind, None))
        .await
        .unwrap()
        .unwrap();
    let _process = BrowserProcess(browser.process_id);
    let fixture = Fixture::new();
    let parent = navigate_page(
        &browser.browser_session,
        &browser.initial_page.page_session,
        &fixture.url,
    )
    .await
    .unwrap()
    .page_session;
    assert!(
        allwright_surface_web::resolve_frame(&browser.browser_session, &parent, "iframe")
            .await
            .unwrap_err()
            .contains("exactly one")
    );
    assert!(
        allwright_surface_web::resolve_frame(&browser.browser_session, &parent, "#unstable")
            .await
            .unwrap_err()
            .contains("still loading or changing")
    );
    let started = std::time::Instant::now();
    let frame = allwright_surface_web::resolve_frame(&browser.browser_session, &parent, "#child")
        .await
        .unwrap()
        .page_session;
    assert!(started.elapsed() >= Duration::from_millis(200));
    assert_eq!(
        get_text_content(&browser.browser_session, &frame, "button")
            .await
            .unwrap()
            .text,
        "Cross origin frame action"
    );
    assert_eq!(
        count_elements(&browser.browser_session, &parent, "button")
            .await
            .unwrap()
            .count,
        0
    );
    let nested = allwright_surface_web::resolve_frame(&browser.browser_session, &frame, "iframe")
        .await
        .unwrap()
        .page_session;
    assert_eq!(
        get_text_content(&browser.browser_session, &nested, "p")
            .await
            .unwrap()
            .text,
        "Nested"
    );
    fill_element(&browser.browser_session, &frame, "input", "in frame")
        .await
        .unwrap();
    click_element(&browser.browser_session, &frame, "button")
        .await
        .unwrap();
    assert_eq!(
        get_text_content(&browser.browser_session, &frame, "button")
            .await
            .unwrap()
            .text,
        "Clicked"
    );
    assert!(
        allwright_surface_web::resolve_frame(&browser.browser_session, &frame, "#slow")
            .await
            .is_err()
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let loaded = loop {
        match allwright_surface_web::resolve_frame(&browser.browser_session, &frame, "#slow").await
        {
            Ok(page) => break page.page_session,
            Err(error) => {
                assert!(std::time::Instant::now() < deadline, "{error}");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    assert_eq!(
        get_text_content(&browser.browser_session, &loaded, "p")
            .await
            .unwrap()
            .text,
        "Loaded"
    );
    assert!(
        allwright_surface_web::resolve_frame(&browser.browser_session, &frame, "button")
            .await
            .unwrap_err()
            .contains("iframe")
    );
    assert!(
        allwright_surface_web::resolve_frame(&browser.browser_session, &parent, ".missing")
            .await
            .is_err()
    );
    navigate_page(&browser.browser_session, &parent, "about:blank")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        count_elements(&browser.browser_session, &frame, "button")
            .await
            .is_err(),
        "detached frame must not fall back to parent"
    );
}
