use allwright_plugin_sdk::{SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopWindowsPlugin;

impl SurfacePlugin for DesktopWindowsPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "desktop-windows",
            family: SurfaceFamily::Desktop,
            version: env!("CARGO_PKG_VERSION"),
            description: "Windows desktop surface plugin for the allwright engine.",
        }
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(10)).await;
    "windows ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_windows_runtime() {
        assert_eq!(boot().await, "windows ready");
    }
}
