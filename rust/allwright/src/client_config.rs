use std::fs;
use std::path::{Path, PathBuf};

use super::command::merge_retry_config;
use super::runtime::set_server_addr;
use super::launch::launch_browser;
use super::types::{
    AllwrightConfig, BrowserKind, Error, LaunchOptions, ResolveConfigOptions, ResolvedConfig,
    Result,
};

const CONFIG_FILENAMES: [&str; 6] = [
    "allwright.config.yaml",
    "allwright.config.yml",
    "allwright.config.json",
    ".allwright/config.yaml",
    ".allwright/config.yml",
    ".allwright/config.json",
];

pub fn find_config_file(start_dir: impl AsRef<Path>) -> Option<PathBuf> {
    let mut current_dir = start_dir.as_ref().to_path_buf();

    loop {
        for filename in CONFIG_FILENAMES {
            let candidate = current_dir.join(filename);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if !current_dir.pop() {
            return None;
        }
    }
}

pub fn load_config_file(config_file: impl AsRef<Path>) -> Result<AllwrightConfig> {
    let resolved = config_file.as_ref().to_path_buf();
    let raw = fs::read_to_string(&resolved).map_err(|error| {
        Error::new(format!(
            "failed to read allwright config {}: {error}",
            resolved.display()
        ))
    })?;

    let extension = resolved
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let config = match extension.as_str() {
        "json" => serde_json::from_str::<AllwrightConfig>(&raw).map_err(|error| {
            Error::new(format!(
                "failed to parse allwright config {} as JSON: {error}",
                resolved.display()
            ))
        })?,
        "yaml" | "yml" => serde_yaml::from_str::<AllwrightConfig>(&raw).map_err(|error| {
            Error::new(format!(
                "failed to parse allwright config {} as YAML: {error}",
                resolved.display()
            ))
        })?,
        _ => {
            return Err(Error::new(format!(
                "unsupported allwright config file extension .{} for {}",
                if extension.is_empty() {
                    "<none>"
                } else {
                    &extension
                },
                resolved.display()
            )));
        }
    };

    validate_config_shape(&config, &resolved)?;
    Ok(config)
}

pub fn resolve_config(options: ResolveConfigOptions) -> Result<ResolvedConfig> {
    let cwd = options.cwd.unwrap_or(std::env::current_dir().map_err(|error| {
        Error::new(format!(
            "failed to determine current working directory: {error}"
        ))
    })?);
    let config_file_path = match options.config_file {
        Some(path) => Some(path),
        None => find_config_file(cwd),
    };
    let file_config = match &config_file_path {
        Some(path) => load_config_file(path)?,
        None => AllwrightConfig::default(),
    };
    let suite_name = options.suite.and_then(|suite| {
        let trimmed = suite.trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let suite_config = match &suite_name {
        Some(name) => {
            let suite = file_config
                .suites
                .as_ref()
                .and_then(|suites| suites.get(name))
                .cloned();
            if suite.is_none() {
                return Err(Error::new(format!(
                    "allwright config suite \"{}\" was not found in {}",
                    name,
                    config_file_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "the resolved config file".to_string())
                )));
            }
            suite
        }
        None => None,
    };

    let server_addr = suite_config
        .as_ref()
        .and_then(|suite| suite.server.as_ref())
        .and_then(|server| server.addr.clone())
        .or_else(|| {
            file_config
                .server
                .as_ref()
                .and_then(|server| server.addr.clone())
        });
    let browser_name = suite_config
        .as_ref()
        .and_then(|suite| suite.browser.as_ref())
        .and_then(|browser| browser.name)
        .or_else(|| {
            file_config
                .browser
                .as_ref()
                .and_then(|browser| browser.name)
        })
        .unwrap_or(BrowserKind::Chromium);
    let browser_binary = suite_config
        .as_ref()
        .and_then(|suite| suite.browser.as_ref())
        .and_then(|browser| browser.binary.clone())
        .or_else(|| {
            file_config
                .browser
                .as_ref()
                .and_then(|browser| browser.binary.clone())
        });
    let mut launch_options = merge_launch_options(
        file_config
            .browser
            .as_ref()
            .and_then(|browser| browser.launch_options.clone()),
        suite_config
            .as_ref()
            .and_then(|suite| suite.browser.as_ref())
            .and_then(|browser| browser.launch_options.clone()),
    );
    if let Some(binary) = &browser_binary {
        launch_options.browser_binary = Some(binary.clone());
    }
    let expect = merge_retry_config(
        file_config.expect.clone(),
        suite_config.and_then(|suite| suite.expect),
    );

    Ok(ResolvedConfig {
        config_file_path,
        suite_name,
        server_addr,
        browser_name,
        browser_binary,
        launch_options,
        expect,
    })
}

pub async fn launch_configured_browser(config: &ResolvedConfig) -> Result<super::types::Browser> {
    if let Some(server_addr) = &config.server_addr {
        set_server_addr(server_addr.clone())?;
    }
    launch_browser(config.browser_name, config.launch_options.clone()).await
}

fn merge_launch_options(
    base: Option<LaunchOptions>,
    override_options: Option<LaunchOptions>,
) -> LaunchOptions {
    let mut merged = base.unwrap_or_default();
    if let Some(override_options) = override_options {
        if override_options.browser_binary.is_some() {
            merged.browser_binary = override_options.browser_binary;
        }
        if override_options.timeout_ms.is_some() {
            merged.timeout_ms = override_options.timeout_ms;
        }
    }
    merged
}

fn validate_config_shape(config: &AllwrightConfig, source: &Path) -> Result<()> {
    if let Some(schema_version) = config.schema_version {
        if schema_version != 1 {
            return Err(Error::new(format!(
                "allwright config {} has unsupported schemaVersion {}; expected 1",
                source.display(),
                schema_version
            )));
        }
    }
    Ok(())
}
