use allwright::{
    desktop::{DesktopMacConnectOptions, DesktopMacLaunchOptions, mac},
    set_server_addr, shutdown,
};

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    set_server_addr(&env_or("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))?;

    let desktop = mac::connect(DesktopMacConnectOptions {
        agent_endpoint: std::env::var("ALLWRIGHT_MAC_AGENT_ENDPOINT").ok(),
        timeout_ms: Some(30_000),
    })
    .await?;
    let app = desktop
        .launch(DesktopMacLaunchOptions {
            app_id: env_or("ALLWRIGHT_MAC_APP_ID", "com.apple.calculator"),
            terminate_running: false,
            timeout_ms: Some(60_000),
        })
        .await?;

    let screenshot = app.screenshot().await?;
    assert!(
        !screenshot.is_empty(),
        "macOS screenshot should not be empty"
    );
    println!("[rust-macos-basic] captured {} bytes", screenshot.len());

    shutdown().await;
    Ok(())
}
