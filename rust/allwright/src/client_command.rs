use crate::proto::{CommandRetryOptions, ElementCountedEvent, ElementsHighlightedEvent};

use super::types::{CountResult, HighlightResult, RetryConfig};

pub(crate) fn command_retry_options(timeout_ms: Option<u32>) -> Option<CommandRetryOptions> {
    timeout_ms.map(|timeout_ms| CommandRetryOptions {
        timeout_ms: Some(timeout_ms),
        retry_interval_ms: None,
    })
}

pub(crate) fn merge_retry_config(
    base: Option<RetryConfig>,
    override_config: Option<RetryConfig>,
) -> RetryConfig {
    let mut merged = base.unwrap_or_default();
    if let Some(override_config) = override_config {
        if override_config.timeout_ms.is_some() {
            merged.timeout_ms = override_config.timeout_ms;
        }
        if override_config.interval_ms.is_some() {
            merged.interval_ms = override_config.interval_ms;
        }
    }
    merged
}

pub(crate) fn count_result_from_event(event: ElementCountedEvent) -> CountResult {
    CountResult {
        selector: event.css_selector,
        count: event.count,
        note: event.note,
    }
}

pub(crate) fn highlight_result_from_event(event: ElementsHighlightedEvent) -> HighlightResult {
    HighlightResult {
        selector: event.css_selector,
        count: event.count,
        note: event.note,
    }
}
