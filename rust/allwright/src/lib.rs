#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
#[path = "engine.rs"]
mod engine_lib;

pub mod proto {
    include!("proto_generated.rs");
}

pub mod plugins {
    pub use allwright_plugin_sdk::{SurfaceFamily, SurfacePluginDescriptor};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PluginPackage {
        pub id: &'static str,
        pub family: SurfaceFamily,
        pub package_name: &'static str,
        pub version: &'static str,
        pub description: &'static str,
    }

    const PLUGINS: [PluginPackage; 6] = [
        PluginPackage {
            id: "web",
            family: SurfaceFamily::Web,
            package_name: "allwright-surface-web",
            version: env!("CARGO_PKG_VERSION"),
            description: "Web surface plugin for the allwright engine.",
        },
        PluginPackage {
            id: "mobile-android",
            family: SurfaceFamily::Mobile,
            package_name: "allwright-surface-mobile-android",
            version: env!("CARGO_PKG_VERSION"),
            description: "Android mobile surface plugin for the allwright engine.",
        },
        PluginPackage {
            id: "mobile-ios",
            family: SurfaceFamily::Mobile,
            package_name: "allwright-surface-mobile-ios",
            version: env!("CARGO_PKG_VERSION"),
            description: "iOS mobile surface plugin for the allwright engine.",
        },
        PluginPackage {
            id: "desktop-mac",
            family: SurfaceFamily::Desktop,
            package_name: "allwright-surface-desktop-mac",
            version: env!("CARGO_PKG_VERSION"),
            description: "macOS desktop surface plugin for the allwright engine.",
        },
        PluginPackage {
            id: "desktop-windows",
            family: SurfaceFamily::Desktop,
            package_name: "allwright-surface-desktop-windows",
            version: env!("CARGO_PKG_VERSION"),
            description: "Windows desktop surface plugin for the allwright engine.",
        },
        PluginPackage {
            id: "desktop-linux",
            family: SurfaceFamily::Desktop,
            package_name: "allwright-surface-desktop-linux",
            version: env!("CARGO_PKG_VERSION"),
            description: "Linux desktop surface plugin for the allwright engine.",
        },
    ];

    pub fn catalog() -> &'static [PluginPackage] {
        &PLUGINS
    }

    pub fn package(plugin_id: &str) -> Option<&'static PluginPackage> {
        catalog().iter().find(|plugin| plugin.id == plugin_id)
    }

    pub fn descriptors() -> Vec<SurfacePluginDescriptor> {
        catalog()
            .iter()
            .map(|plugin| SurfacePluginDescriptor {
                id: plugin.id,
                family: plugin.family,
                version: plugin.version,
                description: plugin.description,
            })
            .collect()
    }
}

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
pub use engine_lib::serve;
