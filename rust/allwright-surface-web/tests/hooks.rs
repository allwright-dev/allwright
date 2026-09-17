use allwright_plugin_sdk::{BrowserKind, HookResult, HookType};
use allwright_surface_web::{
    click_element, close_browser_process, get_text_content, launch_browser, navigate_page,
    poll_hook, register_hook, save_download, set_file_chooser_files,
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
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or_default();
                let (body, content_type, extra_headers) = if target.ends_with("/new") {
                    (
                        "<title>New page</title><h1>New page</h1>",
                        "text/html; charset=utf-8",
                        "",
                    )
                } else if target.ends_with("/download") {
                    (
                        "allwright download",
                        "text/plain; charset=utf-8",
                        "Content-Disposition: attachment; filename=allwright-download.txt\r\n",
                    )
                } else {
                    (
                        "<title>Hook fixture</title><a id='open' target='_blank' href='/new'>Open</a><a id='download' href='/download' download>Download</a><input id='upload' type='file' multiple><p id='files'></p><script>upload.onchange=()=>files.textContent=[...upload.files].map(file=>file.name).join(',')</script>",
                        "text/html; charset=utf-8",
                        "",
                    )
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires installed Chromium/Firefox"]
async fn file_chooser_hook_sets_selected_files() {
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
    let hook = register_hook(
        &browser.browser_session,
        &navigation.page_session,
        HookType::FileChooser,
    )
    .await
    .unwrap();
    click_element(
        &browser.browser_session,
        &navigation.page_session,
        "#upload",
    )
    .await
    .unwrap();

    let chooser = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match poll_hook(&browser.browser_session, &hook).await {
                Ok(HookResult::FileChooser(chooser)) => break chooser,
                Err(error) if error.contains("still waiting") => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(error) => panic!("file chooser hook failed: {error}"),
                Ok(_) => panic!("file chooser hook returned the wrong result type"),
            }
        }
    })
    .await
    .expect("file chooser hook timed out");
    assert!(chooser.is_multiple);

    let upload_path =
        std::env::temp_dir().join(format!("allwright-file-upload-{}.txt", std::process::id()));
    std::fs::write(&upload_path, b"allwright upload").unwrap();
    set_file_chooser_files(
        &browser.browser_session,
        &navigation.page_session,
        &chooser.file_chooser_id,
        &[upload_path.to_string_lossy().to_string()],
    )
    .await
    .unwrap();
    let text = get_text_content(&browser.browser_session, &navigation.page_session, "#files")
        .await
        .unwrap();
    assert_eq!(
        text.text,
        upload_path.file_name().unwrap().to_string_lossy()
    );
    let _ = std::fs::remove_file(upload_path);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires installed Chromium/Firefox"]
async fn download_hook_saves_completed_file() {
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
    let hook = register_hook(
        &browser.browser_session,
        &navigation.page_session,
        HookType::Download,
    )
    .await
    .unwrap();
    click_element(
        &browser.browser_session,
        &navigation.page_session,
        "#download",
    )
    .await
    .unwrap();

    let download = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match poll_hook(&browser.browser_session, &hook).await {
                Ok(HookResult::Download(download)) => break download,
                Err(error) if error.contains("still waiting") => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(error) => panic!("download hook failed: {error}"),
                Ok(_) => panic!("download hook returned the wrong result type"),
            }
        }
    })
    .await
    .expect("download hook timed out");
    assert!(download.url.ends_with("/download"));
    assert!(!download.suggested_filename.is_empty());

    let saved_path = std::env::temp_dir().join(format!(
        "allwright-saved-download-{}.txt",
        std::process::id()
    ));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match save_download(
                &browser.browser_session,
                &navigation.page_session,
                &download.download_id,
                &saved_path.to_string_lossy(),
            )
            .await
            {
                Ok(_) => break,
                Err(error) if error.contains("still in progress") => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(error) => panic!("save download failed: {error}"),
            }
        }
    })
    .await
    .expect("save download timed out");
    assert_eq!(
        std::fs::read_to_string(&saved_path).unwrap(),
        "allwright download"
    );
    let _ = std::fs::remove_file(saved_path);
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
                Ok(_) => panic!("new page hook returned the wrong result type"),
            }
        }
    })
    .await
    .expect("new page hook timed out");
    assert_ne!(page.page_session, navigation.page_session);
}
