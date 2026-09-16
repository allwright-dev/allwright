use allwright_plugin_sdk::{BrowserKind, HookResult, HookType};
use allwright_surface_web::{
    click_element, close_browser_process, launch_browser, navigate_page, poll_hook, register_hook,
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
                let request = String::from_utf8_lossy(&request[..n]);
                let body = if request.starts_with("GET /new ") {
                    "<title>New page</title><h1>New page</h1>"
                } else {
                    "<title>Hook fixture</title><a id='open' target='_blank' href='/new'>Open</a>"
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
async fn new_page_hook_captures_page_opened_before_wait() {
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
    let hook = register_hook(
        &browser.browser_session,
        &browser.initial_page.page_session,
        HookType::NewPage,
    )
    .await
    .unwrap();
    let navigation = navigate_page(
        &browser.browser_session,
        &browser.initial_page.page_session,
        &fixture.url,
    )
    .await
    .unwrap();
    click_element(&browser.browser_session, &navigation.page_session, "#open")
        .await
        .unwrap();

    let page = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match poll_hook(&browser.browser_session, &hook).await {
                Ok(HookResult::NewPage(page)) => break page,
                Err(error) if error.contains("still waiting") => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(error) => panic!("new page hook failed: {error}"),
            }
        }
    })
    .await
    .expect("new page hook timed out");
    assert_ne!(page.page_session, navigation.page_session);
}
