//! DOM accessibility collection and serialization belong to the web plugin.
use super::*;
use allwright_plugin_sdk::AccessibilitySnapshotInfo;

const ACCESSIBILITY_SEMANTICS: &str = include_str!("accessibility_semantics.js");
const COLLECTOR: &str = include_str!("accessibility.js");

fn expression() -> String {
    format!("(() => {{ {ACCESSIBILITY_SEMANTICS}\nreturn JSON.stringify(({COLLECTOR})()); }})()")
}

fn contexts(tree: &Value) -> Result<Vec<(String, Option<String>)>, String> {
    let roots = tree
        .pointer("/result/contexts")
        .and_then(Value::as_array)
        .ok_or("accessibility snapshot: browsingContext.getTree returned no contexts")?;
    let mut pending: Vec<_> = roots.iter().rev().map(|node| (node, None)).collect();
    let mut result = Vec::new();
    while let Some((node, parent)) = pending.pop() {
        let id = node
            .get("context")
            .and_then(Value::as_str)
            .ok_or("accessibility snapshot: browsing context has no id")?
            .to_string();
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            pending.extend(children.iter().rev().map(|child| (child, Some(id.clone()))));
        }
        result.push((id, parent));
    }
    if result.is_empty() {
        return Err("accessibility snapshot: page has no browsing context".to_string());
    }
    Ok(result)
}

fn serialize(snapshot: &Value, format: &str) -> Result<AccessibilitySnapshotInfo, String> {
    let snapshot = match format {
        "json" => serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?,
        "yaml" => super::accessibility_yaml::to_string(snapshot)?,
        _ => return Err("accessibility snapshot format must be 'json' or 'yaml'".to_string()),
    };
    Ok(AccessibilitySnapshotInfo {
        snapshot,
        format: format.to_string(),
    })
}

/// Capture every browsing context (including cross-origin frames) via WebDriver BiDi.
/// The serialized tree is a DOM approximation, not the browser's platform AX tree.
pub async fn accessibility_snapshot(
    browser_session: &BrowserSessionHandle,
    page_session: &PageSessionHandle,
    format: &str,
) -> Result<AccessibilitySnapshotInfo, String> {
    let format = if format.is_empty() { "json" } else { format };
    if !matches!(format, "json" | "yaml") {
        return Err("accessibility snapshot format must be 'json' or 'yaml'".to_string());
    }
    let expression = expression();
    let mut documents = Vec::new();
    match (browser_session, page_session) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium {
                browsing_context_id,
                mapper_target_id,
                target_id,
            },
        ) => {
            let (mut cdp, mapper, context) = chromium_bidi_session(
                cdp_websocket_url,
                mapper_target_id.as_deref(),
                Some(browsing_context_id.as_deref().unwrap_or(target_id)),
            )
            .await?;
            let tree = cdp
                .send_bidi_command(
                    &mapper.mapper_session_id,
                    &json!({
                        "id": 1, "method": "browsingContext.getTree", "params": { "root": context }
                    }),
                )
                .await?;
            for (id, parent) in contexts(&tree)? {
                let mut document = chromium_bidi_evaluate_json_on_session(
                    &mut cdp,
                    &mapper.mapper_session_id,
                    &id,
                    &expression,
                )
                .await?;
                document["contextId"] = json!(id);
                document["parentContextId"] = json!(parent);
                documents.push(document);
            }
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
                .expect("Firefox session guard validated connection id")
                .connection;
            let tree = bidi
                .send_command(
                    "browsingContext.getTree",
                    json!({
                        "root": browsing_context_id
                    }),
                )
                .await?;
            for (id, parent) in contexts(&tree)? {
                let mut document = firefox_evaluate_json(bidi, &id, &expression).await?;
                document["contextId"] = json!(id);
                document["parentContextId"] = json!(parent);
                documents.push(document);
            }
        }
        _ => {
            return Err(
                "browser/page backend mismatch while capturing accessibility snapshot".to_string(),
            );
        }
    }
    serialize(&json!({ "version": 1, "documents": documents }), format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_yaml_round_trip_the_same_typed_data() {
        let value = json!({"version": 1, "documents": [{"root": {
            "role": "checkbox", "name": "yes: \"no\"\n- [x] # 😀", "children": [],
            "states": {"checked": "mixed", "disabled": false, "level": 2},
            "properties": {"value": "null", "description": "true", "url": "https://example.test/?x=:"}
        }}]});
        let json = serialize(&value, "json").unwrap();
        let yaml = serialize(&value, "yaml").unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&json.snapshot).unwrap(),
            value
        );
        assert_eq!(
            serde_yaml::from_str::<Value>(&yaml.snapshot).unwrap(),
            value
        );
        assert!(serialize(&value, "xml").is_err());
    }

    #[test]
    fn includes_nested_frame_contexts_in_document_order() {
        let tree = json!({"result": {"contexts": [{"context": "top", "children": [
            {"context": "child", "children": [{"context": "nested", "children": null}]},
            {"context": "sibling", "children": []}
        ]}]}});
        assert_eq!(
            contexts(&tree).unwrap(),
            vec![
                ("top".into(), None),
                ("child".into(), Some("top".into())),
                ("nested".into(), Some("child".into())),
                ("sibling".into(), Some("top".into()))
            ]
        );
        assert!(contexts(&json!({"result": {"contexts": []}})).is_err());
    }
}
