use allwright_plugin_sdk::{SurfaceFamily, SurfacePlugin, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Copy, Default)]
pub struct MobileAndroidPlugin;

impl SurfacePlugin for MobileAndroidPlugin {
    fn descriptor(&self) -> SurfacePluginDescriptor {
        SurfacePluginDescriptor {
            id: "mobile-android",
            family: SurfaceFamily::Mobile,
            version: env!("CARGO_PKG_VERSION"),
            description: "Android mobile surface plugin for the allwright engine.",
        }
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(10)).await;
    "android ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_android_runtime() {
        assert_eq!(boot().await, "android ready");
    }
}
