use allwright_surface_mobile::{ConnectOptions, LaunchOptions, MobilePlatform};
use allwright_surface_mobile_android::{connect, dump_source, launch_app};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

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

    let source = dump_source(&session.browser_session, &page.page_session)?;
    fs::write(&args.output, source.source)?;

    println!("{}", args.output.display());
    Ok(())
}

#[derive(Debug)]
struct SourceArgs {
    apk_path: String,
    output: PathBuf,
    app_id: Option<String>,
    activity: Option<String>,
    device: Option<String>,
    adb_endpoint: Option<String>,
    stop_before_launch: bool,
}

fn parse_args(args: Vec<String>) -> Result<SourceArgs, Box<dyn Error>> {
    let mut apk_path = String::new();
    let mut output = None;
    let mut app_id = None;
    let mut activity = None;
    let mut device = None;
    let mut adb_endpoint = None;
    let mut stop_before_launch = false;

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--apk" => {
                index += 1;
                apk_path = required_value(&args, index, "--apk")?;
            }
            "--out" => {
                index += 1;
                output = Some(PathBuf::from(required_value(&args, index, "--out")?));
            }
            "--app-id" => {
                index += 1;
                app_id = Some(required_value(&args, index, "--app-id")?);
            }
            "--activity" => {
                index += 1;
                activity = Some(required_value(&args, index, "--activity")?);
            }
            "--device" => {
                index += 1;
                device = Some(required_value(&args, index, "--device")?);
            }
            "--adb-endpoint" => {
                index += 1;
                adb_endpoint = Some(required_value(&args, index, "--adb-endpoint")?);
            }
            "--stop-before-launch" => stop_before_launch = true,
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

    if apk_path.trim().is_empty() {
        return Err("missing required `--apk <path>` argument".into());
    }

    Ok(SourceArgs {
        apk_path,
        output: output.unwrap_or_else(|| PathBuf::from("tmp/mobile-android-source.xml")),
        app_id,
        activity,
        device,
        adb_endpoint,
        stop_before_launch,
    })
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, Box<dyn Error>> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("missing value for `{flag}`").into())
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run -p allwright-surface-mobile-android --example source -- \
  --apk /path/to/app.apk \
  --out tmp/mobile-android-source.xml \
  [--app-id dev.allwright.sample] \
  [--activity .MainActivity] \
  [--device emulator-5554] \
  [--adb-endpoint 192.168.1.10:5555] \
  [--stop-before-launch]"
    );
}
