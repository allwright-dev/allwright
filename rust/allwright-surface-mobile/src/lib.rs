use allwright_plugin_sdk::{SurfaceFamily, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

pub const SURFACE_ID: &str = "mobile";

pub fn shared_descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: SURFACE_ID,
        family: SurfaceFamily::Mobile,
        version: env!("CARGO_PKG_VERSION"),
        description: "Shared mobile surface abstractions for Android and iOS plugins.",
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(25)).await;
    "mobile ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_mobile_runtime() {
        assert_eq!(boot().await, "mobile ready");
    }
}
