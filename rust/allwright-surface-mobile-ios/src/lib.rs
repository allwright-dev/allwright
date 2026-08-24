use allwright_plugin_sdk::{SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Copy, Default)]
pub struct MobileIosPlugin;

impl SurfacePlugin for MobileIosPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "mobile-ios",
            family: SurfaceFamily::Mobile,
            version: env!("CARGO_PKG_VERSION"),
            description: "iOS mobile surface plugin for the allwright engine.",
        }
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(10)).await;
    "ios ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_ios_runtime() {
        assert_eq!(boot().await, "ios ready");
    }
}
