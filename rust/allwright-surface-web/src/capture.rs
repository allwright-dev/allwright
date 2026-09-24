use super::*;
use allwright_plugin_sdk::CaptureInfo;

fn expression(kind: &str, selector: &str, attribute: &str) -> Result<String, String> {
    let element = if kind == "url" {
        "null".to_string()
    } else {
        let segments = selector_segments_literal("capture", selector)?;
        format!(
            "(() => {{ const selectorSegments = {segments}; return ({})[0]; }})()",
            selector_chain_query_all_js()
        )
    };
    Ok(format!(
        "JSON.stringify(({})({}, {}, {}))",
        include_str!("capture.js"),
        json_string_literal(kind, "capture kind")?,
        element,
        json_string_literal(attribute, "attribute name")?
    ))
}

pub async fn capture(
    browser: &BrowserSessionHandle,
    page: &PageSessionHandle,
    kind: &str,
    selector: &str,
    attribute: &str,
) -> Result<CaptureInfo, String> {
    let expression = expression(kind, selector, attribute)?;
    let value = match (browser, page) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium {
                target_id,
                browsing_context_id,
                mapper_target_id,
            },
        ) => {
            chromium_bidi_evaluate_json(
                cdp_websocket_url,
                mapper_target_id.as_deref(),
                Some(browsing_context_id.as_deref().unwrap_or(target_id)),
                &expression,
            )
            .await?
        }
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions
                .get_mut(connection_id)
                .expect("validated Firefox session")
                .connection;
            firefox_evaluate_json(bidi, browsing_context_id, &expression).await?
        }
        _ => return Err("browser/page backend mismatch while capturing".into()),
    };
    serde_json::from_value(value).map_err(|error| format!("invalid capture result: {error}"))
}
