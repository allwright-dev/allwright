use allwright_client::{LaunchOptions, launch_chrome, set_server_addr, shutdown};
use clap::Parser;
use std::io;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Engine server address, with or without scheme
    #[arg(long, default_value = "127.0.0.1:50051")]
    server_addr: String,

    /// Optional Chrome binary path or executable name override for launch_chrome
    #[arg(long)]
    chrome_binary: Option<String>,

    /// Number of additional tabs to open after the initial Chrome tab
    #[arg(long, default_value_t = 3)]
    tabs: u8,

    /// URL to navigate each opened tab to
    #[arg(long, default_value = "https://example.com")]
    navigate_url: String,

    /// Optional CSS selector to click over BiDi after navigation
    #[arg(long)]
    click_selector: Option<String>,

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    set_server_addr(&args.server_addr)?;
    println!(
        "[playground] launching chrome with chrome_binary={:?} via singleton Rust client runtime",
        args.chrome_binary
    );
    let browser = launch_chrome(LaunchOptions {
        chrome_binary: args.chrome_binary.clone(),
    })
    .await?;

    let initial_tab = browser.initial_tab();
    println!(
        "[{}] chrome launched: {} ({}) cdp={} user_data_dir={} initial_tab_session_id={}",
        browser.session_id(),
        browser.browser_name(),
        browser.launch_note(),
        browser.cdp_websocket_url(),
        browser.user_data_dir(),
        initial_tab.session_id()
    );

    let initial_navigation = initial_tab.navigate(&args.navigate_url).await?;
    println!(
        "[{}] tab navigated: {} ({})",
        initial_tab.session_id(),
        initial_navigation.url,
        initial_navigation.note
    );
    println!(
        "[{}] chromium-bidi injected: bidi_session_id={} mapper_target_id={} mapper_session_id={} package_version={}",
        initial_tab.session_id(),
        initial_navigation.bidi_session_id,
        initial_navigation.mapper_target_id,
        initial_navigation.mapper_session_id,
        initial_navigation.package_version
    );

    if let Some(selector) = args.click_selector.as_deref() {
        let click = initial_tab.click(selector).await?;
        println!(
            "[{}] element clicked: selector={} ({}) bidi_session_id={}",
            initial_tab.session_id(),
            click.selector,
            click.note,
            click.bidi_session_id
        );
    }

    let mut additional_tabs = Vec::new();
    for tab_index in 0..args.tabs {
        let tab_number = usize::from(tab_index) + 2;
        let tab = browser.new_tab().await?;
        println!(
            "[{}] tab opened: {} (requested additional tab {})",
            browser.session_id(),
            tab.session_id(),
            tab_number
        );

        let navigation = tab.navigate(&args.navigate_url).await?;
        println!(
            "[{}] tab navigated: {} ({})",
            tab.session_id(),
            navigation.url,
            navigation.note
        );
        println!(
            "[{}] chromium-bidi injected: bidi_session_id={} mapper_target_id={} mapper_session_id={} package_version={}",
            tab.session_id(),
            navigation.bidi_session_id,
            navigation.mapper_target_id,
            navigation.mapper_session_id,
            navigation.package_version
        );

        if let Some(selector) = args.click_selector.as_deref() {
            let click = tab.click(selector).await?;
            println!(
                "[{}] element clicked: selector={} ({}) bidi_session_id={}",
                tab.session_id(),
                click.selector,
                click.note,
                click.bidi_session_id
            );
        }

        additional_tabs.push(tab);
    }

    wait_for_enter(
        "[playground] Press Enter to close the tabs and browser session so Chrome stays open for observation...",
    )
    .await?;

    for tab in &additional_tabs {
        tab.close().await?;
        println!("[{}] tab session closed", tab.session_id());
    }
    initial_tab.close().await?;
    println!("[{}] tab session closed", initial_tab.session_id());

    browser.close().await?;
    println!("[{}] session closed", browser.session_id());

    shutdown().await;
    Ok(())
}

async fn wait_for_enter(prompt: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{prompt}");
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        Ok::<(), io::Error>(())
    })
    .await??;
    Ok(())
}
