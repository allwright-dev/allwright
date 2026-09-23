use super::*;

fn frame_expression(selector: &str) -> Result<String, String> {
    let segments = selector_segments_literal("frame", selector)?;
    let query = selector_chain_query_all_js();
    Ok(format!(
        r#"(() => {{
        const selectorSegments = {segments};
        const elements = {query};
        if (elements.length !== 1) throw new Error('Frame requires exactly one element; found ' + elements.length);
        const element = elements[0];
        if (!['iframe', 'frame'].includes(element.localName)) throw new Error('Frame locator must resolve to an iframe');
        if (!element.isConnected || !element.contentWindow) throw new Error('Frame is detached or not ready');
        const initialDocument = element.contentDocument;
        const src = (element.getAttribute('src') || '').trim();
        if (initialDocument && initialDocument.URL === 'about:blank' &&
            (element.hasAttribute('srcdoc') || (src && !src.startsWith('javascript:') && new URL(src, document.baseURI).href !== 'about:blank'))) {{
            throw new Error('Frame initial navigation is pending');
        }}
        return element.contentWindow;
    }})()"#
    ))
}

// Each attempt is bounded; the engine retries under the caller's overall deadline.
// A loaded document must remain free of DOM mutations for 200ms.
const READY: &str = r#"new Promise(resolve => {
    if (document.readyState !== 'complete') { resolve(false); return; }
    let quiet;
    const finish = value => { clearTimeout(quiet); clearTimeout(limit); observer.disconnect(); resolve(value); };
    const arm = () => { clearTimeout(quiet); quiet = setTimeout(() => finish(document.readyState === 'complete'), 200); };
    const observer = new MutationObserver(arm);
    const limit = setTimeout(() => finish(false), 500);
    observer.observe(document, {subtree:true, childList:true, attributes:true, characterData:true});
    arm();
})"#;

fn frame_context(response: &Value) -> Result<String, String> {
    if response
        .pointer("/result/result/type")
        .and_then(Value::as_str)
        != Some("window")
    {
        return Err(format!(
            "cannot resolve frame: {}",
            response
                .pointer("/result/exceptionDetails")
                .unwrap_or(response)
        ));
    }
    required_string(response, "/result/result/value/context")
}
fn ready(response: &Value) -> Result<(), String> {
    if response
        .pointer("/result/result/value")
        .and_then(Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err("frame document is still loading or changing".to_string())
    }
}

pub async fn resolve_frame(
    browser: &BrowserSessionHandle,
    page: &PageSessionHandle,
    selector: &str,
) -> Result<PageInfo, String> {
    let expression = frame_expression(selector)?;
    let params = |context: &str, expression: &str| json!({"expression":expression,"target":{"context":context},"awaitPromise":true});
    let page_session = match (browser, page) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium {
                target_id,
                browsing_context_id,
                mapper_target_id,
            },
        ) => {
            let (mut cdp, mapper, parent) = chromium_bidi_session(
                cdp_websocket_url,
                mapper_target_id.as_deref(),
                Some(browsing_context_id.as_deref().unwrap_or(target_id)),
            )
            .await?;
            let response = cdp.send_bidi_command(&mapper.mapper_session_id, &json!({"id":7100,"method":"script.evaluate","params":params(&parent, &expression)})).await?;
            let context = frame_context(&response)?;
            ready(
                &cdp.send_bidi_command(
                    &mapper.mapper_session_id,
                    &json!({"id":7101,"method":"script.evaluate","params":params(&context, READY)}),
                )
                .await?,
            )?;
            let current = cdp.send_bidi_command(&mapper.mapper_session_id, &json!({"id":7102,"method":"script.evaluate","params":params(&parent, &expression)})).await?;
            if frame_context(&current)? != context {
                return Err("frame was replaced while waiting".into());
            }
            PageSessionHandle::Chromium {
                target_id: context.clone(),
                browsing_context_id: Some(context),
                mapper_target_id: Some(mapper.mapper_target_id),
            }
        }
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let bidi = &mut sessions.get_mut(connection_id).unwrap().connection;
            let context = frame_context(
                &bidi
                    .send_command("script.evaluate", params(browsing_context_id, &expression))
                    .await?,
            )?;
            bidi.frame_contexts.insert(context.clone());
            ready(
                &bidi
                    .send_command("script.evaluate", params(&context, READY))
                    .await?,
            )?;
            let current = bidi
                .send_command("script.evaluate", params(browsing_context_id, &expression))
                .await?;
            if frame_context(&current)? != context {
                return Err("frame was replaced while waiting".into());
            }
            PageSessionHandle::Firefox {
                browsing_context_id: context,
            }
        }
        _ => return Err("frame browser and page backend mismatch".into()),
    };
    Ok(PageInfo {
        note: "resolved stable frame via WebDriver BiDi".into(),
        page_session,
    })
}

// getTree without a root lists active documents, excluding cached frame realms.
pub(super) fn tree_contains_context(tree: &Value, context: &str) -> bool {
    fn contains(nodes: &[Value], context: &str) -> bool {
        nodes.iter().any(|node| {
            node.get("context").and_then(Value::as_str) == Some(context)
                || node
                    .get("children")
                    .and_then(Value::as_array)
                    .is_some_and(|children| contains(children, context))
        })
    }
    tree.pointer("/result/contexts")
        .and_then(Value::as_array)
        .is_some_and(|nodes| contains(nodes, context))
}
