use allwright_plugin_sdk::{SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopMacPlugin;

impl SurfacePlugin for DesktopMacPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "desktop-mac",
            family: SurfaceFamily::Desktop,
            version: env!("CARGO_PKG_VERSION"),
            description: "macOS desktop surface plugin for the allwright engine.",
        }
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(10)).await;
    "macos ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_macos_runtime() {
        assert_eq!(boot().await, "macos ready");
    }
}
