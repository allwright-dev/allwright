use allwright_plugin_sdk::BrowserKind;
use allwright_surface_web::{
    click_element, close_browser_process, count_elements, fill_element, get_text_content,
    launch_browser, navigate_page,
};
use serde_json::Value;
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
                let body = if request.starts_with("GET /frame ") {
                    "<title>Frame</title><button>Cross origin frame action</button>".to_string()
                } else {
                    include_str!("fixtures/selectors.html").replace("FRAME_PORT", &port.to_string())
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

fn semantic(spec: Value) -> String {
    format!("aw={}", serde_json::to_string(&spec.to_string()).unwrap())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires installed Chromium/Firefox"]
async fn semantic_locators_in_browser() {
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
    let navigation = navigate_page(
        &browser.browser_session,
        &browser.initial_page.page_session,
        &fixture.url,
    )
    .await
    .unwrap();
    let page = &navigation.page_session;
    let cases = vec![
        (serde_json::json!({"kind":"testId","text":"a  \"b\""}), 1),
        (serde_json::json!({"kind":"testId","text":"a \"b\""}), 0),
        (
            serde_json::json!({"kind":"role","role":"heading","level":2,"name":"Account settings","exact":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","name":"Buy"}),
            2,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","name":"Buy","includeHidden":true}),
            3,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","name":"Hidden action","includeHidden":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","name":"Save changes"}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"checkbox","checked":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"checkbox","checked":false}),
            0,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","pressed":false}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","expanded":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","disabled":true}),
            2,
        ),
        (
            serde_json::json!({"kind":"role","role":"option","selected":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"role","role":"button","name":"Shadow action"}),
            1,
        ),
        (
            serde_json::json!({"kind":"text","text":"Hello world","exact":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"text","text":{"regex":"^hello WORLD$","flags":"i"}}),
            1,
        ),
        (
            serde_json::json!({"kind":"label","text":"email ADDRESS"}),
            1,
        ),
        (
            serde_json::json!({"kind":"label","text":"Secret label","exact":true}),
            1,
        ),
        (
            serde_json::json!({"kind":"placeholder","text":"example.com"}),
            1,
        ),
        (
            serde_json::json!({"kind":"altText","text":"Company logo","exact":true}),
            1,
        ),
        (serde_json::json!({"kind":"title","text":"Purchase"}), 2),
        (
            serde_json::json!({"kind":"testId","text":"CaseSensitive"}),
            1,
        ),
        (
            serde_json::json!({"kind":"testId","text":"casesensitive"}),
            0,
        ),
    ];
    for (spec, expected) in cases {
        let selector = semantic(spec);
        assert_eq!(
            count_elements(&browser.browser_session, page, &selector)
                .await
                .unwrap()
                .count,
            expected,
            "{selector}"
        );
    }
    let rows = "css=\"li\"";
    let beta = semantic(serde_json::json!({"kind":"role","role":"heading","name":"Beta"}));
    for (filter, expected) in [
        (
            serde_json::json!({"kind":"filter","has":"CSS:\"button\""}),
            3,
        ),
        (serde_json::json!({"kind":"filter","has":beta}), 1),
        (serde_json::json!({"kind":"filter","hasNot":beta}), 2),
        (serde_json::json!({"kind":"filter","hasText":"Beta"}), 1),
        (
            serde_json::json!({"kind":"filter","hasNotText":{"regex":"alpha","flags":"i"}}),
            2,
        ),
        (serde_json::json!({"kind":"filter","visible":true}), 2),
        (serde_json::json!({"kind":"filter","visible":false}), 1),
    ] {
        let selector = format!("{rows} {}", semantic(filter));
        assert_eq!(
            count_elements(&browser.browser_session, page, &selector)
                .await
                .unwrap()
                .count,
            expected,
            "{selector}"
        );
    }
    let selector = format!(
        "{rows} {} {}",
        semantic(serde_json::json!({"kind":"filter","hasText":"Beta"})),
        semantic(serde_json::json!({"kind":"role","role":"button","name":"Buy"}))
    );
    click_element(&browser.browser_session, page, &selector)
        .await
        .unwrap();
    let selector = format!(
        "{} css=\"li\" {} xpath=\"//h3\"",
        semantic(serde_json::json!({"kind":"role","role":"region","name":"Products"})),
        semantic(serde_json::json!({"kind":"nth","index":1}))
    );
    assert_eq!(
        get_text_content(&browser.browser_session, page, &selector)
            .await
            .unwrap()
            .text,
        "Beta"
    );
    let alpha =
        semantic(serde_json::json!({"kind":"role", "role":"listitem", "name":"", "exact":true}));
    let excluded = format!(
        "css=\"li\" {}",
        semantic(serde_json::json!({"kind":"filter", "hasText":"Beta"}))
    );
    let remaining = format!(
        "css=\"li\" {}",
        semantic(serde_json::json!({"kind":"exclude", "selector":excluded}))
    );
    assert_eq!(
        count_elements(&browser.browser_session, page, &remaining)
            .await
            .unwrap()
            .count,
        2
    );
    // Excluding descendants must not remove their ancestor candidates (unlike hasNot).
    let unchanged = format!(
        "css=\"li\" {}",
        semantic(serde_json::json!({"kind":"exclude", "selector":beta}))
    );
    assert_eq!(
        count_elements(&browser.browser_session, page, &unchanged)
            .await
            .unwrap()
            .count,
        3
    );
    let empty = format!(
        "{alpha} {}",
        semantic(serde_json::json!({"kind":"exclude", "selector":alpha}))
    );
    assert_eq!(
        count_elements(&browser.browser_session, page, &empty)
            .await
            .unwrap()
            .count,
        0
    );
    let email = semantic(serde_json::json!({"kind":"label","text":"Email address"}));
    fill_element(&browser.browser_session, page, &email, "test@example.com")
        .await
        .unwrap();
}
