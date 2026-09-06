use allwright_plugin_sdk::BrowserKind;
use allwright_surface_web::{
    accessibility_snapshot, close_browser_process, launch_browser, navigate_page,
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
                    include_str!("fixtures/accessibility.html")
                        .replace("FRAME_PORT", &port.to_string())
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

fn nodes<'a>(node: &'a Value, result: &mut Vec<&'a Value>) {
    result.push(node);
    for child in node["children"].as_array().unwrap() {
        nodes(child, result);
    }
}
fn find<'a>(nodes: &[&'a Value], role: &str, name: &str) -> &'a Value {
    nodes
        .iter()
        .copied()
        .find(|node| node["role"] == role && node["name"] == name)
        .unwrap_or_else(|| panic!("missing {role} {name:?}"))
}

// Opt-in because this launches installed browser binaries. No drivers or external URLs.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires installed Chromium/Firefox; run with --ignored --nocapture"]
async fn browser_snapshot_round_trip() {
    let kind = if std::env::var("ALLWRIGHT_TEST_BROWSER").as_deref() == Ok("firefox") {
        BrowserKind::Firefox
    } else {
        BrowserKind::Chromium
    };
    let binary = std::env::var("ALLWRIGHT_BROWSER_BINARY").ok();
    let browser = tokio::task::spawn_blocking(move || launch_browser(kind, binary.as_deref()))
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
    let json = accessibility_snapshot(&browser.browser_session, &navigation.page_session, "")
        .await
        .unwrap();
    let yaml = accessibility_snapshot(&browser.browser_session, &navigation.page_session, "yaml")
        .await
        .unwrap();
    assert_eq!(json.format, "json");
    assert_eq!(yaml.format, "yaml");
    let json: Value = serde_json::from_str(&json.snapshot).unwrap();
    let yaml: Value = serde_yaml::from_str(&yaml.snapshot).unwrap();
    assert_eq!(json, yaml);
    assert_eq!(json["version"], 1);
    let documents = json["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 3);
    assert_eq!(documents[0]["parentContextId"], Value::Null);
    for document in &documents[1..] {
        assert_eq!(document["parentContextId"], documents[0]["contextId"]);
    }
    let mut all = Vec::new();
    for document in documents {
        nodes(&document["root"], &mut all);
    }
    assert_eq!(
        find(&all, "heading", "Snapshot fixture")["states"]["level"],
        1
    );
    assert_eq!(
        find(&all, "textbox", "Email address")["properties"]["value"],
        "yes: no"
    );
    assert_eq!(
        find(&all, "textbox", "Email address")["properties"]["description"],
        "Enter your email"
    );
    assert_eq!(
        find(&all, "checkbox", "Accept terms")["states"]["checked"],
        "mixed"
    );
    assert_eq!(
        find(&all, "button", "Disabled action")["states"]["disabled"],
        true
    );
    assert_eq!(find(&all, "button", "Toggle")["states"]["pressed"], false);
    assert_eq!(find(&all, "button", "Toggle")["states"]["expanded"], true);
    find(&all, "button", "Referenced hidden label");
    find(&all, "button", "Shadow slot action");
    find(&all, "button", "Shadow name content");
    find(&all, "button", "Visibility restored");
    find(&all, "button", "Cross origin frame action");
    find(&all, "button", "Same origin frame action");
    find(&all, "button", "Off screen action");
    find(&all, "button", "yes: \"no\" # [x] 😀");
    find(&all, "text", "Ordinary page text.");
    let list = find(&all, "list", "Owned items");
    assert_eq!(list["children"][0]["role"], "listitem");
    assert_eq!(list["children"][0]["children"][0]["name"], "Owned item");
    assert_eq!(
        all.iter()
            .filter(|node| node["name"] == "Owned item")
            .count(),
        1
    );
    find(&all, "textbox", "Hidden native label");
    find(&all, "checkbox", "Choose Blue option");
    find(&all, "button", "Fallback role");
    find(&all, "button", "Focusable presentation conflict");
    find(&all, "button", "Download now");
    find(&all, "button", "First Last");
    find(&all, "textbox", "First label Second label");
    find(&all, "button", "Fallback label");
    find(&all, "button", "Self reference");
    find(&all, "img", "Diagram");
    assert_eq!(
        find(&all, "img", "Vector diagram")["properties"]["description"],
        "SVG help"
    );
    find(&all, "group", "Contact details");
    assert_eq!(
        find(&all, "button", "Named action")["properties"]["description"],
        "Tooltip description"
    );
    assert_eq!(
        find(&all, "button", "Explicit description")["properties"]["description"],
        "Description attribute"
    );
    find(&all, "button", "Start Center End");
    find(&all, "button", "Icon action");
    find(&all, "button", "Visible name");
    assert!(
        find(&all, "button", "Empty description")["properties"]
            .get("description")
            .is_none()
    );
    assert_eq!(
        find(&all, "radio", "Mixed radio")["states"]["checked"],
        false
    );
    find(&all, "gridcell", "Grid value");
    find(&all, "rowheader", "Row heading");
    let article = find(&all, "article", "Scoped article");
    assert!(
        article["children"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["role"] != "banner" && node["role"] != "contentinfo")
    );
    assert_eq!(
        all.iter().filter(|node| node["role"] == "banner").count(),
        1
    );
    assert_eq!(
        all.iter()
            .filter(|node| node["role"] == "contentinfo")
            .count(),
        1
    );
    assert!(!all.iter().any(|node| node["role"] == "region"));
    assert_eq!(
        all.iter().filter(|node| node["role"] == "listitem").count(),
        1
    );
    let names: Vec<_> = all
        .iter()
        .map(|node| node["name"].as_str().unwrap())
        .collect();
    for absent in [
        "Hidden button",
        "Inert button",
        "Closed details action",
        "Unassigned light DOM",
        "secret-password",
    ] {
        assert!(
            !names.contains(&absent),
            "unexpected hidden content {absent}"
        );
    }
    assert!(
        !serde_json::to_string(&json)
            .unwrap()
            .contains("secret-password")
    );
    assert!(
        accessibility_snapshot(&browser.browser_session, &navigation.page_session, "xml")
            .await
            .is_err()
    );
    use allwright_surface_web::{accessibility_snapshot_with_mode, click_element, count_elements};
    let browser = &browser.browser_session;
    let page = &navigation.page_session;
    assert_eq!(
        count_elements(browser, page, "[aria-ref]")
            .await
            .unwrap()
            .count,
        0
    );
    let ai = accessibility_snapshot_with_mode(browser, page, "json", "ai")
        .await
        .unwrap();
    let ai: Value = serde_json::from_str(&ai.snapshot).unwrap();
    let yaml = accessibility_snapshot_with_mode(browser, page, "yaml", "ai")
        .await
        .unwrap();
    assert_eq!(ai, serde_yaml::from_str::<Value>(&yaml.snapshot).unwrap());
    let mut ai_nodes = Vec::new();
    for document in ai["documents"].as_array().unwrap() {
        nodes(&document["root"], &mut ai_nodes);
    }
    let refs: Vec<_> = ai_nodes
        .iter()
        .filter_map(|n| n["aria-ref"].as_str())
        .collect();
    assert!(!refs.is_empty());
    assert_eq!(
        refs.len(),
        refs.iter().collect::<std::collections::HashSet<_>>().len()
    );
    assert!(ai_nodes.iter().any(|n| n["role"] == "generic"));
    assert!(
        find(&ai_nodes, "button", "No pointer action")
            .get("aria-ref")
            .is_none()
    );
    assert!(
        find(&ai_nodes, "button", "Zero size action")
            .get("aria-ref")
            .is_none()
    );
    assert_eq!(
        count_elements(browser, page, "#visual-action[aria-ref]")
            .await
            .unwrap()
            .count,
        1
    );
    let id = find(&ai_nodes, "button", "Reference action")["aria-ref"]
        .as_str()
        .unwrap();
    let selector = format!("[aria-ref=\"{id}\"]");
    assert_eq!(
        count_elements(browser, page, &selector)
            .await
            .unwrap()
            .count,
        1
    );
    click_element(browser, page, &selector).await.unwrap();
    let changed = accessibility_snapshot_with_mode(browser, page, "json", "ai")
        .await
        .unwrap();
    let changed: Value = serde_json::from_str(&changed.snapshot).unwrap();
    let mut changed_nodes = Vec::new();
    nodes(&changed["documents"][0]["root"], &mut changed_nodes);
    assert_ne!(
        find(&changed_nodes, "button", "Reference changed")["aria-ref"],
        id
    );
    assert_eq!(
        count_elements(browser, page, &selector)
            .await
            .unwrap()
            .count,
        0
    );
    for mode in ["default", "codegen", "autoexpect"] {
        let snapshot = accessibility_snapshot_with_mode(browser, page, "json", mode)
            .await
            .unwrap();
        let snapshot: Value = serde_json::from_str(&snapshot.snapshot).unwrap();
        let mut items = Vec::new();
        nodes(&snapshot["documents"][0]["root"], &mut items);
        assert!(items.iter().all(|n| n.get("aria-ref").is_none()));
        if mode == "autoexpect" {
            assert!(!items.iter().any(|n| n["name"] == "Zero size action"));
        }
    }
    // Non-AI captures leave existing attributes intact. The next AI capture
    // removes our attribute once this element becomes hidden.
    let changed_id = find(&changed_nodes, "button", "Reference changed")["aria-ref"]
        .as_str()
        .unwrap();
    let changed_selector = format!("[aria-ref=\"{changed_id}\"]");
    assert_eq!(
        count_elements(browser, page, &changed_selector)
            .await
            .unwrap()
            .count,
        1
    );
    click_element(browser, page, &changed_selector)
        .await
        .unwrap();
    accessibility_snapshot_with_mode(browser, page, "json", "ai")
        .await
        .unwrap();
    assert_eq!(
        count_elements(browser, page, &changed_selector)
            .await
            .unwrap()
            .count,
        0
    );
    assert!(
        accessibility_snapshot_with_mode(browser, page, "json", "invalid")
            .await
            .is_err()
    );
}
