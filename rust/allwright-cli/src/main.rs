use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Write};
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command.unwrap_or(CliCommand::Serve {
        listen_addr: "127.0.0.1:50051".parse()?,
    }) {
        CliCommand::Serve { listen_addr } => {
            println!(
                "Starting lightweight allwright engine core on {}. Install `web` to enable browser commands.",
                listen_addr
            );
            allwright::serve(listen_addr).await?;
        }
        CliCommand::Plugin { command } => handle_plugin_command(command)?,
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
                    plugin.id,
                    plugin.package_name,
                    plugin.version,
                    status,
                    plugin.description
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
                upsert_plugin(&mut installed, package.id, package.package_name, target_version);
                println!(
                    "Installed plugin `{}` -> {}@{}",
                    package.id, package.package_name, target_version
                );
            }
            write_installed_plugins(&installed)?;
            println!("Plugin manifest: {}", plugin_manifest_path()?.display());
        }
    }

    Ok(())
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
        let Some(package_name) = parts.next() else { continue };
        let Some(version) = parts.next() else { continue };
        plugins.push(InstalledPlugin {
            id: id.to_string(),
            package_name: package_name.to_string(),
            version: version.to_string(),
        });
    }
    Ok(plugins)
}

fn plugin_is_installed(id: &str) -> Result<bool, Box<dyn Error>> {
    Ok(read_installed_plugins()?.iter().any(|plugin| plugin.id == id))
}

fn install_plugin_package(
    _package_name: &str,
    version: &str,
    plugin_id: &str,
) -> Result<(), Box<dyn Error>> {
    let install_root = plugin_install_root(plugin_id)?;
    if install_root.exists() {
        fs::remove_dir_all(&install_root)?;
    }
    fs::create_dir_all(&install_root)?;

    let asset_name = plugin_asset_name(plugin_id, version)?;
    let asset_bytes = download_plugin_release_asset(version, &asset_name)?;
    unpack_plugin_release_asset(&asset_name, &asset_bytes, &install_root)?;

    if !plugin_runtime_artifact_path(plugin_id).exists() {
        return Err(format!(
            "downloaded plugin `{plugin_id}` but did not find runtime artifact `{}` in the archive",
            plugin_runtime_artifact_filename(plugin_id)
        )
        .into());
    }

    Ok(())
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
        "windows" => format!("{}.dll", plugin_runtime_artifact_stem(plugin_id).replace('-', "_")),
        _ => plugin_runtime_artifact_stem(plugin_id).to_string(),
    }
}

fn ensure_plugin_install_supported(plugin_id: &str) -> Result<(), Box<dyn Error>> {
    match plugin_id {
        "web" => Ok(()),
        _ => Err(format!(
            "plugin `{plugin_id}` is not yet installable. Only `web` currently ships a standalone runtime artifact."
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
        (os, arch) => Err(format!(
            "unsupported platform for plugin downloads: os={os}, arch={arch}"
        )
        .into()),
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
    let mut request = client
        .get(url)
        .header("User-Agent", format!("allwright-cli/{}", env!("CARGO_PKG_VERSION")));

    if let Ok(token) = env::var("ALLWRIGHT_GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token);
        }
    }

    let response = request.send()?.error_for_status()?;
    Ok(response.bytes()?.to_vec())
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
