use tokio::time::{Duration, sleep};

pub async fn boot() -> String {
    let (_ios, _android) = tokio::join!(ios_lib::boot(), android_lib::boot());
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
