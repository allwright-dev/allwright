#[path = "client_bootstrap.rs"]
mod bootstrap;
#[path = "client_browser.rs"]
mod browser;
#[path = "client_capture.rs"]
mod capture;
#[path = "client_command.rs"]
mod command;
#[path = "client_config.rs"]
mod config;
#[path = "client_hook.rs"]
mod hook;
#[path = "client_launch.rs"]
mod launch;
#[path = "client_locator.rs"]
mod locator;
#[path = "client_mobile.rs"]
pub mod mobile;
#[path = "client_runtime.rs"]
mod runtime;
#[path = "client_selectors.rs"]
mod selectors;
#[path = "client_tab.rs"]
mod tab;
#[path = "client_tab_accessibility.rs"]
mod tab_accessibility;
#[path = "client_tab_actions.rs"]
mod tab_actions;
#[path = "client_tab_query.rs"]
mod tab_query;
#[path = "client_types.rs"]
mod types;
#[path = "client_web_locators.rs"]
mod web_locators;

pub use config::{find_config_file, launch_configured_browser, load_config_file, resolve_config};
pub use hook::{
    DOWNLOAD, Download, DownloadHook, FILE_CHOOSER, FileChooser, FileChooserHook, Hook, HookType,
    NEW_PAGE, NewPage,
};
pub use launch::{chromium, firefox, launch_browser, launch_chrome, launch_firefox};
pub use runtime::{ping, set_server_addr, shutdown};
pub use types::*;

pub use web_locators::{LocatorFilterOptions, RoleOptions, TextMatcher, TextOptions};
