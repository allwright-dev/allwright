use allwright_plugin_sdk::{SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopLinuxPlugin;

impl SurfacePlugin for DesktopLinuxPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "desktop-linux",
            family: SurfaceFamily::Desktop,
            version: env!("CARGO_PKG_VERSION"),
            description: "Linux desktop surface plugin for the allwright engine.",
        }
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(10)).await;
    "linux ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_linux_runtime() {
        assert_eq!(boot().await, "linux ready");
    }
}
