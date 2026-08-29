use std::sync::{Arc, Mutex, OnceLock};

use crate::proto::engine_service_client::EngineServiceClient;

use super::bootstrap::{ensure_runtime_ready, shutdown_managed_server};
use super::types::{Error, Result, RuntimeClient};

const DEFAULT_SERVER_ADDR: &str = "http://127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR: &str = "ALLWRIGHT_SERVER_ADDR";

static RUNTIME: OnceLock<Mutex<Option<Arc<RuntimeClient>>>> = OnceLock::new();
static SERVER_ADDR_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub async fn ping() -> Result<String> {
    let runtime = get_runtime().await?;
    let mut engine = runtime.engine.clone();
    let response = engine
        .ping(tonic::Request::new(crate::proto::PingRequest {}))
        .await?;
    Ok(response.into_inner().message)
}

pub fn set_server_addr(server_addr: impl Into<String>) -> Result<()> {
    let normalized = normalize_server_addr(&server_addr.into());
    let mut override_slot = server_addr_override_slot()
        .lock()
        .map_err(|_| Error::new("server address override lock is poisoned"))?;
    *override_slot = Some(normalized);
    drop(override_slot);

    let mut runtime = runtime_slot()
        .lock()
        .map_err(|_| Error::new("runtime singleton lock is poisoned"))?;
    *runtime = None;
    shutdown_managed_server()?;
    Ok(())
}

pub async fn shutdown() {
    if let Ok(mut runtime) = runtime_slot().lock() {
        *runtime = None;
    }
    let _ = shutdown_managed_server();
}

pub(crate) async fn get_runtime() -> Result<Arc<RuntimeClient>> {
    if let Ok(runtime) = runtime_slot().lock() {
        if let Some(existing) = runtime.as_ref() {
            return Ok(Arc::clone(existing));
        }
    }

    let endpoint = configured_server_addr();
    let resolved_endpoint = ensure_runtime_ready(&endpoint).await?;
    let engine = EngineServiceClient::connect(resolved_endpoint).await?;
    let runtime = Arc::new(RuntimeClient { engine });

    let mut slot = runtime_slot()
        .lock()
        .map_err(|_| Error::new("runtime singleton lock is poisoned"))?;
    if let Some(existing) = slot.as_ref() {
        return Ok(Arc::clone(existing));
    }
    *slot = Some(Arc::clone(&runtime));
    Ok(runtime)
}

fn runtime_slot() -> &'static Mutex<Option<Arc<RuntimeClient>>> {
    RUNTIME.get_or_init(|| Mutex::new(None))
}

fn server_addr_override_slot() -> &'static Mutex<Option<String>> {
    SERVER_ADDR_OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn configured_server_addr() -> String {
    if let Ok(server_addr_override) = server_addr_override_slot().lock() {
        if let Some(server_addr) = server_addr_override.as_ref() {
            return server_addr.clone();
        }
    }

    normalize_server_addr(
        std::env::var(SERVER_ADDR_ENV_VAR)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .as_deref()
            .unwrap_or(DEFAULT_SERVER_ADDR),
    )
}

fn normalize_server_addr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}
