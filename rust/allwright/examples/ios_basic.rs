use allwright::{
    CommandOptions, RoleOptions, TextMatcher, TextOptions,
    mobile::{MobileIosConnectOptions, MobileIosLaunchOptions, ios},
    set_server_addr, shutdown,
};

const DEFAULT_IOS_APP_PATH: &str = "https://allwright.dev/Flights-simulator.ipa";
const EXPECTED_ALERT: &str = "No account found. Please sign up first.";

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn exact_role(name: &str) -> RoleOptions {
    RoleOptions {
        name: Some(TextMatcher::from(name)),
        exact: true,
        ..Default::default()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    set_server_addr(&env_or("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))?;

    let device = ios::connect(MobileIosConnectOptions {
        device: std::env::var("ALLWRIGHT_IOS_DEVICE").ok(),
        agent_endpoint: std::env::var("ALLWRIGHT_IOS_AGENT_ENDPOINT").ok(),
        timeout_ms: Some(60_000),
        ..Default::default()
    })
    .await?;

    let app = device
        .launch(MobileIosLaunchOptions {
            app_path: Some(env_or("ALLWRIGHT_IOS_APP_PATH", DEFAULT_IOS_APP_PATH)),
            app_id: std::env::var("ALLWRIGHT_IOS_APP_ID").ok(),
            stop_before_launch: true,
            timeout_ms: Some(120_000),
        })
        .await?;

    app.get_by_role_with_options("button", exact_role("Login"))
        .click(CommandOptions::default())
        .await?;
    app.get_by_label_with_options("Email", TextOptions { exact: true })
        .fill("allwright@example.com", CommandOptions::default())
        .await?;
    app.get_by_label_with_options("Password", TextOptions { exact: true })
        .fill("not-a-real-password", CommandOptions::default())
        .await?;
    app.get_by_role_with_options("button", exact_role("Submit"))
        .click(CommandOptions::default())
        .await?;

    let alert = app
        .get_by_text_with_options(EXPECTED_ALERT, TextOptions { exact: true })
        .text_content(CommandOptions::default())
        .await?;
    assert_eq!(alert.as_deref(), Some(EXPECTED_ALERT));
    println!("[rust-ios-basic] {}", alert.unwrap_or_default());

    shutdown().await;
    Ok(())
}
