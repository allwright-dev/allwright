use clap::{Parser, Subcommand};
use std::env;
use std::error::Error;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

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
            println!("Starting lightweight allwright engine core on {}", listen_addr);
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
                let target_version = version.as_deref().unwrap_or(package.version);
                upsert_plugin(&mut installed, package.id, package.package_name, target_version);
                println!(
                    "Registered plugin `{}` -> {}@{}",
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
