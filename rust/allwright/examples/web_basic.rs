use allwright::{LaunchOptions, launch_firefox, set_server_addr, shutdown};

const DEFAULT_WEB_URL: &str = "https://themoderninternet.vercel.app";
const DEFAULT_WEB_ENTRY_SELECTOR: &str = "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']";
const DEFAULT_WEB_HEADING_SELECTOR: &str = "xpath=//h1[text()=\"Form Inputs\"]";
const DEFAULT_WEB_HEADING_TEXT: &str = "Form Inputs";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    set_server_addr(
        &std::env::var("ALLWRIGHT_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:50051".to_string()),
    )?;

    let browser = launch_firefox(LaunchOptions {
        browser_binary: std::env::var("ALLWRIGHT_BROWSER_BINARY").ok(),
        ..Default::default()
    })
    .await?;

    let result = async {
        let page = browser.page();
        page.navigate(
            std::env::var("ALLWRIGHT_WEB_URL").unwrap_or_else(|_| DEFAULT_WEB_URL.to_string()),
        )
        .await?;
        page.click(
            &std::env::var("ALLWRIGHT_WEB_ENTRY_SELECTOR")
                .unwrap_or_else(|_| DEFAULT_WEB_ENTRY_SELECTOR.to_string()),
        )
        .await?;
        page.wait_for_selector_with_options(
            &std::env::var("ALLWRIGHT_WEB_HEADING_SELECTOR")
                .unwrap_or_else(|_| DEFAULT_WEB_HEADING_SELECTOR.to_string()),
            allwright::WaitForSelectorOptions {
                visible: Some(true),
                timeout_ms: Some(10_000),
            },
        )
        .await?;
        let heading = page
            .text_content(
                &std::env::var("ALLWRIGHT_WEB_HEADING_SELECTOR")
                    .unwrap_or_else(|_| DEFAULT_WEB_HEADING_SELECTOR.to_string()),
            )
            .await?;
        let expected_heading = std::env::var("ALLWRIGHT_WEB_HEADING_TEXT")
            .unwrap_or_else(|_| DEFAULT_WEB_HEADING_TEXT.to_string());
        if !heading.text.contains(&expected_heading) {
            return Err(format!(
                "expected heading to contain {expected_heading:?}, got {:?}",
                heading.text
            )
            .into());
        }
        println!("[rust-web-basic] heading={:?}", heading.text);
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    browser.close().await?;
    shutdown().await;
    result
}
