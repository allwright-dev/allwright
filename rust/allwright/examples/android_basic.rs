use allwright::{
    mobile::{android, MobileAndroidConnectOptions, MobileAndroidLaunchOptions},
    set_server_addr, shutdown, CommandOptions,
};

const DEFAULT_ANDROID_DEVICE: &str = "emulator-5554";
const DEFAULT_ANDROID_APP_ID: &str = "com.example.airticket";
const DEFAULT_ANDROID_TAP_SELECTOR: &str = "Id=com.example.airticket:id/bottom_nav_account";
const DEFAULT_ANDROID_FILL_SELECTOR: &str = "xpath=//*[@text=\"Email\"]";
const DEFAULT_ANDROID_FILL_VALUE: &str = "user@example.com";

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    set_server_addr(&env_or("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))?;

    let device = android::connect(MobileAndroidConnectOptions {
        device: Some(env_or("ALLWRIGHT_ANDROID_DEVICE", DEFAULT_ANDROID_DEVICE)),
        adb_endpoint: std::env::var("ALLWRIGHT_ANDROID_ADB_ENDPOINT").ok(),
        timeout_ms: Some(30_000),
        ..Default::default()
    })
    .await?;

    let apk_path = std::env::var("ALLWRIGHT_ANDROID_APK_PATH").ok();
    let app_id = Some(env_or("ALLWRIGHT_ANDROID_APP_ID", DEFAULT_ANDROID_APP_ID));

    let app = device
        .launch(MobileAndroidLaunchOptions {
            apk_path,
            app_id,
            launch_activity: std::env::var("ALLWRIGHT_ANDROID_APP_ACTIVITY").ok(),
            timeout_ms: Some(60_000),
            ..Default::default()
        })
        .await?;

    app.click(
        &env_or(
            "ALLWRIGHT_ANDROID_TAP_SELECTOR",
            DEFAULT_ANDROID_TAP_SELECTOR,
        ),
        CommandOptions::default(),
    )
    .await?;
    app.fill(
        &env_or("ALLWRIGHT_ANDROID_FILL_SELECTOR", DEFAULT_ANDROID_FILL_SELECTOR),
        &env_or("ALLWRIGHT_ANDROID_FILL_VALUE", DEFAULT_ANDROID_FILL_VALUE),
        CommandOptions::default(),
    )
    .await?;

    println!("[rust-android-basic] app_session_id={}", app.session_id());
    shutdown().await;
    Ok(())
}
