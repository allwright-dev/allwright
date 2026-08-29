use std::env;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use flate2::read::GzDecoder;
use libloading::{Library, Symbol};
use reqwest::blocking::Client;
use tar::Archive;
use zip::ZipArchive;

use crate::plugins::package;

const ALLWRIGHT_AUTO_INSTALL_ENV_VAR: &str = "ALLWRIGHT_AUTO_INSTALL";
const ALLWRIGHT_GITHUB_TOKEN_ENV_VAR: &str = "ALLWRIGHT_GITHUB_TOKEN";
const ALLWRIGHT_HOME_ENV_VAR: &str = "ALLWRIGHT_HOME";
const ALLWRIGHT_REPOSITORY_ENV_VAR: &str = "ALLWRIGHT_REPOSITORY";
const ALLWRIGHT_VERSION_ENV_VAR: &str = "ALLWRIGHT_VERSION";
const DEFAULT_RELEASE_REPOSITORY: &str = "allwright-dev/allwright";
const GITHUB_RELEASE_BASE_URL: &str =
    "https://github.com/allwright-dev/allwright/releases/download";
const GITHUB_API_BASE_URL: &str = "https://api.github.com/repos";

type PluginApiVersionFn = unsafe extern "C" fn() -> u32;
type PluginIdFn = unsafe extern "C" fn() -> *const c_char;
type PluginInvokeFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut c_char);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub id: String,
    pub package_name: String,
    pub version: String,
}

pub fn installed_plugins() -> Result<Vec<InstalledPlugin>, String> {
    let path = plugin_manifest_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "failed to read allwright plugin manifest {}: {error}",
                path.display()
            ));
        }
    };

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
            version: normalize_release_version(version),
        });
    }
    Ok(plugins)
}

pub fn install_plugin(
    plugin_id: &str,
    version_override: Option<&str>,
) -> Result<InstalledPlugin, String> {
    install_plugin_inner(plugin_id, version_override, true)
}

pub fn ensure_plugin_available(plugin_id: &str) -> Result<InstalledPlugin, String> {
    install_plugin_inner(plugin_id, None, false)
}

pub fn invoke_plugin(plugin_id: &str, request_json: &str) -> Result<String, String> {
    ensure_plugin_available(plugin_id)?;
    let library_path = plugin_runtime_artifact_path(plugin_id)?;
    invoke_plugin_library(plugin_id, &library_path, request_json)
}

pub fn plugin_runtime_artifact_path(plugin_id: &str) -> Result<PathBuf, String> {
    Ok(plugin_install_root(plugin_id)?
        .join("lib")
        .join(plugin_runtime_artifact_filename(plugin_id)?))
}

fn install_plugin_inner(
    plugin_id: &str,
    version_override: Option<&str>,
    explicit_install: bool,
) -> Result<InstalledPlugin, String> {
    let package = package(plugin_id).ok_or_else(|| {
        let supported = crate::plugins::catalog()
            .iter()
            .map(|plugin| plugin.id)
            .collect::<Vec<_>>()
            .join(", ");
        format!("unknown plugin `{plugin_id}`. Supported plugins: {supported}")
    })?;
    ensure_plugin_install_supported(plugin_id)?;
    let target_version = resolve_target_version(package.version, version_override)?;
    let runtime_artifact = plugin_runtime_artifact_path(plugin_id)?;
    if runtime_artifact.is_file()
        && installed_plugin_version(plugin_id)?.as_deref() == Some(target_version.as_str())
    {
        return Ok(InstalledPlugin {
            id: plugin_id.to_string(),
            package_name: package.package_name.to_string(),
            version: target_version,
        });
    }

    if !explicit_install && !auto_install_enabled() {
        return Err(format!(
            "plugin `{plugin_id}` is required but automatic installation is disabled. Install it with `allwright plugin install {plugin_id}`."
        ));
    }

    install_plugin_package(plugin_id, package.package_name, &target_version)?;
    let mut installed = installed_plugins()?;
    upsert_plugin(
        &mut installed,
        plugin_id,
        package.package_name,
        &target_version,
    );
    write_installed_plugins(&installed)?;

    if !runtime_artifact.is_file() {
        return Err(format!(
            "plugin `{plugin_id}` installed but runtime artifact is missing at {}",
            runtime_artifact.display()
        ));
    }
    if installed_plugin_version(plugin_id)?.as_deref() != Some(target_version.as_str()) {
        return Err(format!(
            "plugin `{plugin_id}` installed but version {} is not active",
            target_version
        ));
    }

    Ok(InstalledPlugin {
        id: plugin_id.to_string(),
        package_name: package.package_name.to_string(),
        version: target_version,
    })
}

fn invoke_plugin_library(
    plugin_id: &str,
    library_path: &Path,
    request_json: &str,
) -> Result<String, String> {
    let request_cstr = CString::new(request_json)
        .map_err(|error| format!("plugin request contains NUL: {error}"))?;
    let library = unsafe { Library::new(library_path) }.map_err(|error| {
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
            ));
        }

        let loaded_plugin_id: Symbol<'_, PluginIdFn> = library
            .get(b"allwright_plugin_id")
            .map_err(|error| format!("failed to load plugin id symbol: {error}"))?;
        let raw_plugin_id = loaded_plugin_id();
        if raw_plugin_id.is_null() {
            return Err(format!("plugin `{plugin_id}` returned a null plugin id"));
        }
        let loaded_plugin_id = CStr::from_ptr(raw_plugin_id)
            .to_str()
            .map_err(|error| format!("plugin id is not valid UTF-8: {error}"))?;
        if loaded_plugin_id != plugin_id {
            return Err(format!(
                "loaded plugin id `{loaded_plugin_id}` does not match requested plugin `{plugin_id}`"
            ));
        }

        let invoke: Symbol<'_, PluginInvokeFn> = library
            .get(b"allwright_plugin_invoke")
            .map_err(|error| format!("failed to load plugin invoke symbol: {error}"))?;
        let free_string: Symbol<'_, PluginFreeStringFn> = library
            .get(b"allwright_plugin_free_string")
            .map_err(|error| format!("failed to load plugin free-string symbol: {error}"))?;

        let response_ptr = invoke(request_cstr.as_ptr());
        if response_ptr.is_null() {
            return Err(format!("plugin `{plugin_id}` returned a null response"));
        }
        let response = CStr::from_ptr(response_ptr)
            .to_str()
            .map_err(|error| format!("plugin response is not valid UTF-8: {error}"))?
            .to_string();
        free_string(response_ptr);
        Ok(response)
    }
}

fn install_plugin_package(
    plugin_id: &str,
    package_name: &str,
    version: &str,
) -> Result<(), String> {
    let install_root = plugin_install_root(plugin_id)?;
    let runtime_artifact = plugin_runtime_artifact_filename(plugin_id)?;

    if install_root.exists() {
        fs::remove_dir_all(&install_root).map_err(|error| {
            format!(
                "failed to remove previous plugin installation {}: {error}",
                install_root.display()
            )
        })?;
    }
    fs::create_dir_all(&install_root).map_err(|error| {
        format!(
            "failed to prepare plugin install directory {}: {error}",
            install_root.display()
        )
    })?;

    if let Some(local_artifact) = repo_local_plugin_artifact_path(plugin_id, version) {
        let destination = install_root.join("lib").join(&runtime_artifact);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to prepare plugin runtime directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        fs::copy(&local_artifact, &destination).map_err(|error| {
            format!(
                "failed to copy local plugin artifact {} to {}: {error}",
                local_artifact.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }

    let asset_name = plugin_asset_name(plugin_id, version)?;
    let asset_bytes = download_plugin_release_asset(version, &asset_name)?;
    unpack_plugin_release_asset(&asset_name, &asset_bytes, &install_root)?;

    let runtime_path = install_root.join("lib").join(&runtime_artifact);
    if !runtime_path.is_file() {
        return Err(format!(
            "downloaded plugin `{plugin_id}` from {package_name}@{version} but did not find runtime artifact `{runtime_artifact}`"
        ));
    }
    Ok(())
}

fn resolve_target_version(
    default_version: &str,
    version_override: Option<&str>,
) -> Result<String, String> {
    let requested = version_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            env::var(ALLWRIGHT_VERSION_ENV_VAR)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| default_version.to_string());

    if requested.eq_ignore_ascii_case("latest") {
        let tag = resolve_latest_release_tag()?;
        return Ok(normalize_release_version(&tag));
    }
    Ok(normalize_release_version(&requested))
}

fn resolve_latest_release_tag() -> Result<String, String> {
    let repository = release_repository();
    let url = format!("{GITHUB_API_BASE_URL}/{repository}/releases/latest");
    let client = release_client()?;
    let response = client
        .get(&url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("failed to resolve latest allwright release: {error}"))?;
    let payload_text = response
        .text()
        .map_err(|error| format!("failed to read latest allwright release metadata: {error}"))?;
    let payload: serde_json::Value = serde_json::from_str(&payload_text)
        .map_err(|error| format!("failed to decode latest allwright release metadata: {error}"))?;
    let tag = payload
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "latest allwright release metadata did not include tag_name".to_string())?;
    Ok(tag.to_string())
}

fn release_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(format!("allwright-core/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("failed to build allwright release client: {error}"))
}

fn release_repository() -> String {
    env::var(ALLWRIGHT_REPOSITORY_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASE_REPOSITORY.to_string())
}

fn download_plugin_release_asset(version: &str, asset_name: &str) -> Result<Vec<u8>, String> {
    let url = format!("{GITHUB_RELEASE_BASE_URL}/v{version}/{asset_name}");
    let client = release_client()?;
    let mut request = client.get(&url);
    if let Ok(token) = env::var(ALLWRIGHT_GITHUB_TOKEN_ENV_VAR) {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token);
        }
    }
    let mut response = request
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("failed to download plugin asset {asset_name}: {error}"))?;
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read plugin asset {asset_name}: {error}"))?;
    Ok(bytes)
}

fn unpack_plugin_release_asset(
    asset_name: &str,
    asset_bytes: &[u8],
    install_root: &Path,
) -> Result<(), String> {
    if asset_name.ends_with(".tar.gz") {
        let decoder = GzDecoder::new(Cursor::new(asset_bytes));
        let mut archive = Archive::new(decoder);
        for entry in archive
            .entries()
            .map_err(|error| format!("read plugin tar entries: {error}"))?
        {
            let mut entry = entry.map_err(|error| format!("read plugin tar entry: {error}"))?;
            entry.unpack_in(install_root).map_err(|error| {
                format!(
                    "unpack plugin archive entry into {}: {error}",
                    install_root.display()
                )
            })?;
        }
        return Ok(());
    }

    if asset_name.ends_with(".zip") {
        let reader = Cursor::new(asset_bytes);
        let mut archive =
            ZipArchive::new(reader).map_err(|error| format!("open plugin zip archive: {error}"))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("read plugin zip entry {index}: {error}"))?;
            let Some(relative_path) = safe_archive_path(entry.name()) else {
                continue;
            };
            let destination = install_root.join(relative_path);
            if entry.is_dir() {
                fs::create_dir_all(&destination).map_err(|error| {
                    format!("create plugin directory {}: {error}", destination.display())
                })?;
                continue;
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!("create plugin directory {}: {error}", parent.display())
                })?;
            }
            let mut output = fs::File::create(&destination).map_err(|error| {
                format!("create plugin file {}: {error}", destination.display())
            })?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|error| format!("copy plugin file {}: {error}", destination.display()))?;
        }
        return Ok(());
    }

    Err(format!(
        "unsupported plugin archive format for `{asset_name}`"
    ))
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

fn write_installed_plugins(plugins: &[InstalledPlugin]) -> Result<(), String> {
    let path = plugin_manifest_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create allwright plugin manifest directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut lines = Vec::with_capacity(plugins.len() + 1);
    lines.push("# plugin_id\tcrate_name\tversion".to_string());
    for plugin in plugins {
        lines.push(format!(
            "{}\t{}\t{}",
            plugin.id, plugin.package_name, plugin.version
        ));
    }
    fs::write(&path, lines.join("\n") + "\n").map_err(|error| {
        format!(
            "failed to write allwright plugin manifest {}: {error}",
            path.display()
        )
    })
}

fn installed_plugin_version(plugin_id: &str) -> Result<Option<String>, String> {
    Ok(installed_plugins()?
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .map(|plugin| plugin.version))
}

fn plugin_manifest_path() -> Result<PathBuf, String> {
    Ok(allwright_home()?.join("plugins.txt"))
}

fn plugin_install_root(plugin_id: &str) -> Result<PathBuf, String> {
    Ok(allwright_home()?.join("plugins").join(plugin_id))
}

fn allwright_home() -> Result<PathBuf, String> {
    if let Ok(home) = env::var(ALLWRIGHT_HOME_ENV_VAR) {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let home = env::var("HOME")
        .map_err(|_| "HOME is not set and ALLWRIGHT_HOME was not provided".to_string())?;
    Ok(PathBuf::from(home).join(".allwright"))
}

fn auto_install_enabled() -> bool {
    env::var(ALLWRIGHT_AUTO_INSTALL_ENV_VAR)
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            )
        })
        .unwrap_or(true)
}

fn repo_local_plugin_artifact_path(plugin_id: &str, version: &str) -> Option<PathBuf> {
    if version != normalize_release_version(env!("CARGO_PKG_VERSION")) {
        return None;
    }
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?;
    let filename = plugin_runtime_artifact_filename(plugin_id).ok()?;
    ["target/debug", "target/release"]
        .into_iter()
        .map(|dir| repo_root.join(dir).join(&filename))
        .find(|candidate| candidate.is_file())
}

fn ensure_plugin_install_supported(plugin_id: &str) -> Result<(), String> {
    match plugin_id {
        "web" | "mobile-android" => Ok(()),
        _ => Err(format!(
            "plugin `{plugin_id}` is not yet installable. Supported standalone runtime artifacts currently ship for `web` and `mobile-android`."
        )),
    }
}

fn plugin_asset_name(plugin_id: &str, version: &str) -> Result<String, String> {
    let (target, extension) = release_target_platform()?;
    Ok(format!(
        "{}-v{}-{}.{}",
        plugin_runtime_artifact_stem(plugin_id)?,
        version,
        target,
        extension
    ))
}

fn release_target_platform() -> Result<(&'static str, &'static str), String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok(("aarch64-apple-darwin", "tar.gz")),
        ("macos", "x86_64") => Ok(("x86_64-apple-darwin", "tar.gz")),
        ("linux", "aarch64") => Ok(("aarch64-unknown-linux-gnu", "tar.gz")),
        ("linux", "x86_64") => Ok(("x86_64-unknown-linux-gnu", "tar.gz")),
        ("windows", "aarch64") => Ok(("aarch64-pc-windows-msvc", "zip")),
        ("windows", "x86_64") => Ok(("x86_64-pc-windows-msvc", "zip")),
        (os, arch) => Err(format!(
            "unsupported platform for plugin downloads: os={os}, arch={arch}"
        )),
    }
}

fn plugin_runtime_artifact_stem(plugin_id: &str) -> Result<&'static str, String> {
    match plugin_id {
        "web" => Ok("allwright-surface-web"),
        "mobile-android" => Ok("allwright-surface-mobile-android"),
        _ => Err(format!(
            "automatic install is not supported for allwright plugin `{plugin_id}`"
        )),
    }
}

fn plugin_runtime_artifact_filename(plugin_id: &str) -> Result<String, String> {
    let stem = plugin_runtime_artifact_stem(plugin_id)?.replace('-', "_");
    Ok(match env::consts::OS {
        "macos" => format!("lib{stem}.dylib"),
        "linux" => format!("lib{stem}.so"),
        "windows" => format!("{stem}.dll"),
        os => {
            return Err(format!(
                "automatic install is not supported for allwright plugin `{plugin_id}` on {os}"
            ));
        }
    })
}

fn normalize_release_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').to_string()
}
