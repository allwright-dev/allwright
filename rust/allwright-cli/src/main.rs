use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use libloading::{Library, Symbol};
use reqwest::blocking::Client;
use std::env;
use std::error::Error;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tar::Archive;
use zip::ZipArchive;

const GITHUB_RELEASE_BASE_URL: &str =
    "https://github.com/allwright-dev/allwright/releases/download";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Start the gRPC engine server
    Serve {
        /// gRPC listen address for the engine server
        #[arg(long, default_value = "127.0.0.1:50051")]
        listen_addr: SocketAddr,
    },
    /// Manage installable surface plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
}

#[derive(Subcommand, Debug)]
enum PluginCommand {
    /// List the supported plugin packages
    List,
    /// Register one plugin or all plugins in the local plugin manifest
    Install {
        /// Plugin id such as `web` or `mobile-android`, or `all`
        plugin: String,
        /// Override the registered plugin version
        #[arg(long)]
        version: Option<String>,
    },
    /// Invoke one installed plugin command with a JSON request payload
    Invoke {
        /// Plugin id such as `web` or `mobile-android`
        plugin: String,
        /// Raw JSON request payload for the plugin command
        #[arg(long)]
        request_json: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command.unwrap_or(CliCommand::Serve {
        listen_addr: "127.0.0.1:50051".parse()?,
    }) {
        CliCommand::Serve { listen_addr } => {
            println!("Starting allwright engine on {}", listen_addr);
            allwright::serve(listen_addr).await?;
        }
        CliCommand::Plugin { command } => {
            tokio::task::block_in_place(|| handle_plugin_command(command))?;
        }
    }

    Ok(())
}

fn handle_plugin_command(command: PluginCommand) -> Result<(), Box<dyn Error>> {
    match command {
        PluginCommand::List => {
            let installed = read_installed_plugins()?;
            for plugin in allwright::plugins::catalog() {
                let status = installed
                    .iter()
                    .find(|entry| entry.id == plugin.id)
                    .map(|entry| format!("installed@{}", entry.version))
                    .unwrap_or_else(|| "available".to_string());
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    plugin.id, plugin.package_name, plugin.version, status, plugin.description
                );
            }
        }
        PluginCommand::Install { plugin, version } => {
            let targets = if plugin == "all" {
                allwright::plugins::catalog()
                    .iter()
                    .map(|plugin| plugin.id)
                    .collect::<Vec<_>>()
            } else {
                vec![plugin.as_str()]
            };

            let mut installed = read_installed_plugins()?;
            for plugin_id in targets {
                let package = allwright::plugins::package(plugin_id).ok_or_else(|| {
                    let supported = allwright::plugins::catalog()
                        .iter()
                        .map(|plugin| plugin.id)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("unknown plugin `{plugin_id}`. Supported plugins: {supported}, all")
                })?;
                ensure_plugin_install_supported(package.id)?;
                let target_version = version.as_deref().unwrap_or(package.version);
                install_plugin_package(package.package_name, target_version, package.id)?;
                upsert_plugin(
                    &mut installed,
                    package.id,
                    package.package_name,
                    target_version,
                );
                println!(
                    "Installed plugin `{}` -> {}@{}",
                    package.id, package.package_name, target_version
                );
            }
            write_installed_plugins(&installed)?;
            println!("Plugin manifest: {}", plugin_manifest_path()?.display());
        }
        PluginCommand::Invoke {
            plugin,
            request_json,
        } => {
            let response = invoke_installed_plugin(&plugin, &request_json)?;
            print!("{response}");
            std::io::stdout().flush()?;
        }
    }

    Ok(())
}

type PluginApiVersionFn = unsafe extern "C" fn() -> u32;
type PluginIdFn = unsafe extern "C" fn() -> *const c_char;
type PluginInvokeFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut c_char);

fn invoke_installed_plugin(plugin_id: &str, request_json: &str) -> Result<String, Box<dyn Error>> {
    let library_path = plugin_runtime_artifact_path(plugin_id);
    if !library_path.is_file() {
        return Err(format!(
            "plugin `{plugin_id}` is not installed. Run `allwright plugin install {plugin_id}` first."
        )
        .into());
    }

    let request_cstr = CString::new(request_json)
        .map_err(|error| format!("plugin request contains NUL: {error}"))?;
    let library = unsafe { Library::new(&library_path) }.map_err(|error| {
        format!(
            "failed to load plugin `{plugin_id}` from {}: {error}",
            library_path.display()
        )
    })?;

    unsafe {
        let api_version: Symbol<'_, PluginApiVersionFn> = library
            .get(b"allwright_plugin_api_version")
            .map_err(|error| format!("failed to load plugin api version symbol: {error}"))?;
        let actual_api_version = api_version();
        if actual_api_version != allwright_plugin_sdk::ALLWRIGHT_PLUGIN_API_VERSION {
            return Err(format!(
                "plugin `{plugin_id}` ABI version mismatch: expected {}, got {}",
                allwright_plugin_sdk::ALLWRIGHT_PLUGIN_API_VERSION,
                actual_api_version
            )
            .into());
        }

        let loaded_plugin_id: Symbol<'_, PluginIdFn> = library
            .get(b"allwright_plugin_id")
            .map_err(|error| format!("failed to load plugin id symbol: {error}"))?;
        let raw_plugin_id = loaded_plugin_id();
        if raw_plugin_id.is_null() {
            return Err(format!("plugin `{plugin_id}` returned a null plugin id").into());
        }
        let loaded_plugin_id = CStr::from_ptr(raw_plugin_id)
            .to_str()
            .map_err(|error| format!("plugin id is not valid UTF-8: {error}"))?;
        if loaded_plugin_id != plugin_id {
            return Err(format!(
                "loaded plugin id `{loaded_plugin_id}` does not match requested plugin `{plugin_id}`"
            )
            .into());
        }

        let invoke: Symbol<'_, PluginInvokeFn> = library
            .get(b"allwright_plugin_invoke")
            .map_err(|error| format!("failed to load plugin invoke symbol: {error}"))?;
        let free_string: Symbol<'_, PluginFreeStringFn> = library
            .get(b"allwright_plugin_free_string")
            .map_err(|error| format!("failed to load plugin free-string symbol: {error}"))?;

        let response_ptr = invoke(request_cstr.as_ptr());
        if response_ptr.is_null() {
            return Err(format!("plugin `{plugin_id}` returned a null response").into());
        }

        let response = CStr::from_ptr(response_ptr)
            .to_str()
            .map_err(|error| format!("plugin response is not valid UTF-8: {error}"))?
            .to_string();
        free_string(response_ptr);
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledPlugin {
    id: String,
    package_name: String,
    version: String,
}

fn upsert_plugin(
    installed: &mut Vec<InstalledPlugin>,
    id: &str,
    package_name: &str,
    version: &str,
) {
    if let Some(existing) = installed.iter_mut().find(|entry| entry.id == id) {
        existing.package_name = package_name.to_string();
        existing.version = version.to_string();
        return;
    }

    installed.push(InstalledPlugin {
        id: id.to_string(),
        package_name: package_name.to_string(),
        version: version.to_string(),
    });
}

fn read_installed_plugins() -> Result<Vec<InstalledPlugin>, Box<dyn Error>> {
    let path = plugin_manifest_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)?;
    let mut plugins = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(3, '\t');
        let Some(id) = parts.next() else { continue };
        let Some(package_name) = parts.next() else {
            continue;
        };
        let Some(version) = parts.next() else {
            continue;
        };
        plugins.push(InstalledPlugin {
            id: id.to_string(),
            package_name: package_name.to_string(),
            version: version.to_string(),
        });
    }
    Ok(plugins)
}

fn install_plugin_package(
    package_name: &str,
    version: &str,
    plugin_id: &str,
) -> Result<(), Box<dyn Error>> {
    let install_root = plugin_install_root(plugin_id)?;
    let runtime_artifact = plugin_runtime_artifact_filename(plugin_id);

    println!(
        "Installing plugin `{plugin_id}` from {package_name}@{version} for {}-{}...",
        env::consts::OS,
        env::consts::ARCH
    );

    if install_root.exists() {
        println!(
            "Removing previous installation at {}...",
            install_root.display()
        );
        fs::remove_dir_all(&install_root)?;
    }
    println!("Preparing install directory {}...", install_root.display());
    fs::create_dir_all(&install_root)?;

    if let Some(local_artifact) = repo_local_plugin_artifact_path(plugin_id) {
        println!(
            "Using local plugin artifact {} for `{plugin_id}`...",
            local_artifact.display()
        );
        let destination = install_root
            .join(plugin_runtime_artifact_dir(plugin_id))
            .join(&runtime_artifact);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&local_artifact, &destination)?;
        println!("Verified runtime artifact `{runtime_artifact}`.");
        return Ok(());
    }

    let asset_name = plugin_asset_name(plugin_id, version)?;
    let asset_bytes = download_plugin_release_asset(version, &asset_name)?;
    println!("Unpacking {asset_name} into {}...", install_root.display());
    unpack_plugin_release_asset(&asset_name, &asset_bytes, &install_root)?;

    if !plugin_runtime_artifact_path(plugin_id).exists() {
        return Err(format!(
            "downloaded plugin `{plugin_id}` but did not find runtime artifact `{}` in the archive",
            runtime_artifact
        )
        .into());
    }

    println!("Verified runtime artifact `{runtime_artifact}`.");

    Ok(())
}

fn repo_local_plugin_artifact_path(plugin_id: &str) -> Option<PathBuf> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?;
    let filename = plugin_runtime_artifact_filename(plugin_id);
    ["target/debug", "target/release"]
        .into_iter()
        .map(|dir| repo_root.join(dir).join(&filename))
        .find(|candidate| candidate.is_file())
}

fn plugin_runtime_artifact_path(plugin_id: &str) -> PathBuf {
    plugin_install_root(plugin_id)
        .unwrap_or_else(|_| PathBuf::from(plugin_id))
        .join(plugin_runtime_artifact_dir(plugin_id))
        .join(plugin_runtime_artifact_filename(plugin_id))
}

fn plugin_runtime_artifact_stem(plugin_id: &str) -> &'static str {
    match plugin_id {
        "web" => "allwright-surface-web",
        "mobile-android" => "allwright-surface-mobile-android",
        _ => "allwright-plugin",
    }
}

fn plugin_runtime_artifact_dir(plugin_id: &str) -> &'static str {
    match plugin_id {
        "web" => "lib",
        _ => "lib",
    }
}

fn plugin_runtime_artifact_filename(plugin_id: &str) -> String {
    match env::consts::OS {
        "macos" => format!(
            "lib{}.dylib",
            plugin_runtime_artifact_stem(plugin_id).replace('-', "_")
        ),
        "linux" => format!(
            "lib{}.so",
            plugin_runtime_artifact_stem(plugin_id).replace('-', "_")
        ),
        "windows" => format!(
            "{}.dll",
            plugin_runtime_artifact_stem(plugin_id).replace('-', "_")
        ),
        _ => plugin_runtime_artifact_stem(plugin_id).to_string(),
    }
}

fn ensure_plugin_install_supported(plugin_id: &str) -> Result<(), Box<dyn Error>> {
    match plugin_id {
        "web" | "mobile-android" => Ok(()),
        _ => Err(format!(
            "plugin `{plugin_id}` is not yet installable. Supported standalone runtime artifacts currently ship for `web` and `mobile-android`."
        )
        .into()),
    }
}

fn plugin_asset_name(plugin_id: &str, version: &str) -> Result<String, Box<dyn Error>> {
    let (target, extension) = release_target_platform()?;
    Ok(format!(
        "{}-v{}-{}.{}",
        plugin_runtime_artifact_stem(plugin_id),
        version,
        target,
        extension
    ))
}

fn release_target_platform() -> Result<(&'static str, &'static str), Box<dyn Error>> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok(("aarch64-apple-darwin", "tar.gz")),
        ("macos", "x86_64") => Ok(("x86_64-apple-darwin", "tar.gz")),
        ("linux", "aarch64") => Ok(("aarch64-unknown-linux-gnu", "tar.gz")),
        ("linux", "x86_64") => Ok(("x86_64-unknown-linux-gnu", "tar.gz")),
        ("windows", "aarch64") => Ok(("aarch64-pc-windows-msvc", "zip")),
        ("windows", "x86_64") => Ok(("x86_64-pc-windows-msvc", "zip")),
        (os, arch) => {
            Err(format!("unsupported platform for plugin downloads: os={os}, arch={arch}").into())
        }
    }
}

fn download_plugin_release_asset(
    version: &str,
    asset_name: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let url = format!("{GITHUB_RELEASE_BASE_URL}/v{version}/{asset_name}");
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let mut request = client.get(&url).header(
        "User-Agent",
        format!("allwright-cli/{}", env!("CARGO_PKG_VERSION")),
    );

    if let Ok(token) = env::var("ALLWRIGHT_GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token);
        }
    }

    println!("Downloading {asset_name} from {url}...");
    let mut response = request.send()?.error_for_status()?;
    let total_bytes = response.content_length();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    let mut next_progress_marker = 0_u64;

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        bytes.extend_from_slice(&buffer[..read]);
        downloaded += read as u64;

        match total_bytes {
            Some(total) if total > 0 => {
                let percent = downloaded.saturating_mul(100) / total;
                if percent >= next_progress_marker || downloaded == total {
                    println!("Downloaded {downloaded}/{total} bytes ({percent}%)...");
                    next_progress_marker = percent.saturating_add(10);
                }
            }
            _ => {
                if downloaded >= next_progress_marker {
                    println!("Downloaded {downloaded} bytes...");
                    next_progress_marker = downloaded.saturating_add(512 * 1024);
                }
            }
        }
    }

    println!("Download complete: {} bytes.", bytes.len());
    Ok(bytes)
}

fn unpack_plugin_release_asset(
    asset_name: &str,
    asset_bytes: &[u8],
    install_root: &Path,
) -> Result<(), Box<dyn Error>> {
    if asset_name.ends_with(".tar.gz") {
        let decoder = GzDecoder::new(Cursor::new(asset_bytes));
        let mut archive = Archive::new(decoder);
        for entry in archive.entries()? {
            let mut entry = entry?;
            entry.unpack_in(install_root)?;
        }
        return Ok(());
    }

    if asset_name.ends_with(".zip") {
        let reader = Cursor::new(asset_bytes);
        let mut archive = ZipArchive::new(reader)?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let Some(relative_path) = safe_archive_path(entry.name()) else {
                continue;
            };
            let destination = install_root.join(relative_path);
            if entry.is_dir() {
                fs::create_dir_all(&destination)?;
                continue;
            }

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = fs::File::create(destination)?;
            std::io::copy(&mut entry, &mut output)?;
            output.flush()?;
        }
        return Ok(());
    }

    Err(format!("unsupported plugin archive format for `{asset_name}`").into())
}

fn safe_archive_path(raw_path: &str) -> Option<PathBuf> {
    let path = Path::new(raw_path);
    let mut clean = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }

    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn write_installed_plugins(plugins: &[InstalledPlugin]) -> Result<(), Box<dyn Error>> {
    let path = plugin_manifest_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lines = Vec::with_capacity(plugins.len() + 1);
    lines.push("# plugin_id\tcrate_name\tversion".to_string());
    for plugin in plugins {
        lines.push(format!(
            "{}\t{}\t{}",
            plugin.id, plugin.package_name, plugin.version
        ));
    }
    fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

fn plugin_manifest_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(home) = env::var("ALLWRIGHT_HOME") {
        return Ok(PathBuf::from(home).join("plugins.txt"));
    }

    let home = env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set and ALLWRIGHT_HOME was not provided")?;
    Ok(home.join(".allwright").join("plugins.txt"))
}

fn plugin_install_root(plugin_id: &str) -> Result<PathBuf, Box<dyn Error>> {
    let manifest_path = plugin_manifest_path()?;
    let plugin_home = manifest_path
        .parent()
        .ok_or("plugin manifest path has no parent directory")?;
    Ok(plugin_home.join("plugins").join(plugin_id))
}
