use allwright_plugin_sdk::{SurfaceFamily, SurfacePluginDescriptor};
use tokio::time::{Duration, sleep};

pub const SURFACE_ID: &str = "desktop";

pub fn shared_descriptor() -> SurfacePluginDescriptor {
    SurfacePluginDescriptor {
        id: SURFACE_ID,
        family: SurfaceFamily::Desktop,
        version: env!("CARGO_PKG_VERSION"),
        description: "Shared desktop surface abstractions for macOS, Windows, and Linux plugins.",
    }
}

pub async fn boot() -> String {
    sleep(Duration::from_millis(20)).await;
    "desktop ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boots_desktop_runtime() {
        assert_eq!(boot().await, "desktop ready");
    }
}
