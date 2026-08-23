use tokio::time::{Duration, sleep};

pub async fn boot() -> String {
    let (_windows, _macos, _linux) =
        tokio::join!(windows_lib::boot(), macos_lib::boot(), linux_lib::boot());
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
