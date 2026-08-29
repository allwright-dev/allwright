use allwright_surface_mobile::{ConnectOptions, LaunchOptions, MobilePlatform};
use allwright_surface_mobile_android::{click_element, connect, launch_app};
use serde_json::json;
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args(env::args().skip(1).collect())?;

    let session = connect(&ConnectOptions {
        platform: MobilePlatform::Android,
        device: args.device,
        adb_endpoint: args.adb_endpoint,
        preserve_app_state: false,
        timeout_ms: Some(10_000),
    })?;

    let page = launch_app(
        &session.browser_session,
        &LaunchOptions {
            apk_path: Some(args.apk_path),
            app_id: args.app_id,
            launch_activity: args.activity,
            stop_before_launch: args.stop_before_launch,
            timeout_ms: Some(30_000),
        },
    )?;

    let clicked = click_element(
        &session.browser_session,
        &page.page_session,
        &args.selector,
        Some(10_000),
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "session": session,
            "page": page,
            "click": clicked,
        }))?
    );

    Ok(())
}

#[derive(Debug, Default)]
struct SmokeArgs {
    apk_path: String,
    selector: String,
    app_id: Option<String>,
    activity: Option<String>,
    device: Option<String>,
    adb_endpoint: Option<String>,
    stop_before_launch: bool,
}

fn parse_args(args: Vec<String>) -> Result<SmokeArgs, Box<dyn Error>> {
    let mut parsed = SmokeArgs::default();
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--apk" => {
                index += 1;
                parsed.apk_path = required_value(&args, index, "--apk")?;
            }
            "--selector" => {
                index += 1;
                parsed.selector = required_value(&args, index, "--selector")?;
            }
            "--app-id" => {
                index += 1;
                parsed.app_id = Some(required_value(&args, index, "--app-id")?);
            }
            "--activity" => {
                index += 1;
                parsed.activity = Some(required_value(&args, index, "--activity")?);
            }
            "--device" => {
                index += 1;
                parsed.device = Some(required_value(&args, index, "--device")?);
            }
            "--adb-endpoint" => {
                index += 1;
                parsed.adb_endpoint = Some(required_value(&args, index, "--adb-endpoint")?);
            }
            "--stop-before-launch" => parsed.stop_before_launch = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument `{other}`").into());
            }
        }
        index += 1;
    }

    if parsed.apk_path.trim().is_empty() {
        return Err("missing required `--apk <path>` argument".into());
    }
    if parsed.selector.trim().is_empty() {
        return Err("missing required `--selector <selector>` argument".into());
    }

    Ok(parsed)
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, Box<dyn Error>> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("missing value for `{flag}`").into())
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run -p allwright-surface-mobile-android --example smoke -- \
  --apk /path/to/app.apk \
  --selector 'xpath=//*[@text=\"Login\"]' \
  [--app-id dev.allwright.sample] \
  [--activity .MainActivity] \
  [--device emulator-5554] \
  [--adb-endpoint 192.168.1.10:5555] \
  [--stop-before-launch]"
    );
}
