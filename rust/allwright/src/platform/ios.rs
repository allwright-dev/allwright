use tokio::time::{Duration, sleep};

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
