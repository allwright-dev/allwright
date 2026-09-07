//! Native snapshot semantics and ephemeral references belong to the Android plugin.
use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone)]
struct Capture {
    scope: String,
    device: String,
    package: Option<String>,
    activity: Option<String>,
    source: String,
    references: HashMap<String, String>,
    generation: String,
    created: std::time::Instant,
}
static CACHE: OnceLock<Mutex<HashMap<String, Capture>>> = OnceLock::new();
fn cache() -> &'static Mutex<HashMap<String, Capture>> {
    CACHE.get_or_init(Default::default)
}

pub(super) fn unique_id(prefix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
fn scope(browser: &MobileBrowserSessionHandle, page: &MobilePageSessionHandle) -> String {
    format!(
        "{}:{}:{}",
        browser.device.device_id, browser.automation.session_id, page.page_id
    )
}
fn prune(cache: &mut HashMap<String, Capture>) {
    cache.retain(|_, capture| capture.created.elapsed() < Duration::from_secs(600));
}
pub(super) fn clear(
    browser: &MobileBrowserSessionHandle,
    page: &MobilePageSessionHandle,
) -> Result<(), String> {
    cache()
        .lock()
        .map_err(|_| "snapshot reference cache poisoned")?
        .remove(&scope(browser, page));
    Ok(())
}
fn reference(selector: &str) -> Result<Option<String>, String> {
    let segments = parse_selector_segments(selector)?;
    let refs: Vec<_> = segments
        .iter()
        .filter_map(|s| s.value.strip_prefix("ref="))
        .collect();
    if refs.is_empty() {
        return Ok(None);
    }
    if segments.len() != 1 || refs[0].is_empty() {
        return Err("snapshot references must be standalone ref=<id> selectors".into());
    }
    Ok(Some(refs[0].to_string()))
}
pub(super) fn validate_scope(
    browser: &MobileBrowserSessionHandle,
    page: &MobilePageSessionHandle,
    selector: &str,
) -> Result<(), String> {
    let Some(reference) = reference(selector)? else {
        return Ok(());
    };
    let mut cache = cache()
        .lock()
        .map_err(|_| "snapshot reference cache poisoned")?;
    prune(&mut cache);
    if cache
        .get(&scope(browser, page))
        .is_some_and(|capture| capture.references.contains_key(&reference))
    {
        return Ok(());
    }
    Err("stale or foreign snapshot reference; capture a new AI accessibility snapshot".into())
}

fn absolute_path(nodes: &[AndroidUiNode], index: usize) -> String {
    let node = &nodes[index];
    let ordinal = nodes[..index]
        .iter()
        .filter(|n| n.parent_index == node.parent_index)
        .count()
        + 1;
    let parent = node
        .parent_index
        .map(|p| absolute_path(nodes, p))
        .unwrap_or_else(|| "/hierarchy".into());
    format!("{parent}/node[{ordinal}]")
}
pub(super) fn resolve_absolute_path(
    nodes: &[AndroidUiNode],
    path: &str,
) -> Result<Option<usize>, String> {
    let tail = path
        .strip_prefix("/hierarchy/")
        .ok_or("absolute Android XPath must start with /hierarchy/")?;
    let mut parent = None;
    for step in tail.split('/') {
        let ordinal = step
            .strip_prefix("node[")
            .and_then(|s| s.strip_suffix(']'))
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .ok_or("absolute Android XPath supports only node[positive-position] steps")?;
        parent = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent_index == parent)
            .nth(ordinal - 1)
            .map(|(i, _)| i);
        if parent.is_none() {
            return Ok(None);
        }
    }
    Ok(parent)
}
pub(super) fn matching_source_nodes<'a>(
    device: &str,
    source: &UiAutomator2SourceInfo,
    nodes: &'a [AndroidUiNode],
    selector: &str,
) -> Result<Vec<&'a AndroidUiNode>, String> {
    let Some(reference) = reference(selector)? else {
        return matching_nodes_by_selector(nodes, selector);
    };
    let mut cache = cache()
        .lock()
        .map_err(|_| "snapshot reference cache poisoned")?;
    prune(&mut cache);
    let capture = cache
        .values()
        .find(|c| c.device == device && c.references.contains_key(&reference))
        .cloned()
        .ok_or("stale snapshot reference; capture a new AI accessibility snapshot")?;
    if capture.source != source.source
        || capture.package != source.current_package
        || capture.activity != source.current_activity
    {
        cache.remove(&capture.scope);
        return Err("stale snapshot reference: Android hierarchy or foreground app changed; capture a new AI accessibility snapshot".into());
    }
    let index = resolve_absolute_path(nodes, &capture.references[&reference])?
        .ok_or("stale snapshot reference path")?;
    Ok(vec![&nodes[index]])
}
fn rendered(node: &AndroidUiNode) -> bool {
    node.bounds
        .is_some_and(|b| b.right > b.left && b.bottom > b.top)
}
fn role(node: &AndroidUiNode) -> &'static str {
    match node
        .class_name
        .as_deref()
        .unwrap_or("")
        .rsplit('.')
        .next()
        .unwrap_or("")
    {
        "Button" | "ImageButton" => "button",
        "EditText" => "textbox",
        "CheckBox" => "checkbox",
        "RadioButton" => "radio",
        "Switch" | "SwitchCompat" | "ToggleButton" => "switch",
        "SeekBar" => "slider",
        "ProgressBar" => "progressbar",
        "Spinner" => "combobox",
        "ListView" | "RecyclerView" => "list",
        "ImageView" => "img",
        "TextView" => "text",
        _ => "generic",
    }
}
fn node_json(
    nodes: &[AndroidUiNode],
    index: usize,
    mode: &str,
    refs: &HashMap<String, String>,
) -> Vec<Value> {
    let node = &nodes[index];
    let children: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.parent_index == Some(index))
        .flat_map(|(i, _)| node_json(nodes, i, mode, refs))
        .collect();
    if mode == "autoexpect" && !rendered(node) {
        return children;
    }
    let name = if node.password == Some(true) {
        ""
    } else {
        node.content_desc
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(node.text.as_deref())
            .unwrap_or("")
    };
    let mut states = json!({});
    for (key, value) in [
        (
            "checked",
            node.checked.filter(|_| node.checkable == Some(true)),
        ),
        ("selected", node.selected),
        ("focused", node.focused),
        ("disabled", node.enabled.map(|v| !v)),
    ] {
        if let Some(value) = value {
            states[key] = json!(value);
        }
    }
    let path = absolute_path(nodes, index);
    let mut properties = json!({"class":node.class_name, "resourceId":node.resource_id, "package":node.package_name, "xpath":path, "clickable":node.clickable, "focusable":node.focusable, "scrollable":node.scrollable, "password":node.password, "rendered":rendered(node)});
    if let Some(b) = node.bounds {
        properties["bounds"] =
            json!({"x":b.left,"y":b.top,"width":b.right-b.left,"height":b.bottom-b.top});
    }
    let mut result = json!({"role":role(node),"name":name,"states":states,"properties":properties,"children":children});
    if let Some((id, _)) = refs.iter().find(|(_, p)| **p == path) {
        result["aria-ref"] = json!(id);
    }
    vec![result]
}
fn capture_source(
    browser: &MobileBrowserSessionHandle,
    page: &MobilePageSessionHandle,
    source: UiAutomator2SourceInfo,
    mode: &str,
) -> Result<Value, String> {
    let nodes = parse_android_ui_nodes(&source.source)?;
    if nodes.is_empty() {
        return Err("Android hierarchy contains no accessible nodes".into());
    }
    let scope = scope(browser, page);
    {
        let mut cache = cache()
            .lock()
            .map_err(|_| "snapshot reference cache poisoned")?;
        prune(&mut cache);
        if cache.get(&scope).is_some_and(|c| {
            c.source != source.source
                || c.package != source.current_package
                || c.activity != source.current_activity
        }) {
            cache.remove(&scope);
        }
    }
    let mut references = HashMap::new();
    let mut generation = None;
    if mode == "ai" {
        let mut cache = cache()
            .lock()
            .map_err(|_| "snapshot reference cache poisoned")?;
        prune(&mut cache);
        let previous = cache.get(&scope).filter(|c| {
            c.source == source.source
                && c.package == source.current_package
                && c.activity == source.current_activity
        });
        if let Some(previous) = previous {
            references = previous.references.clone();
            generation = Some(previous.generation.clone());
        } else {
            let id = unique_id("m");
            for (i, node) in nodes.iter().enumerate() {
                if rendered(node)
                    && node.enabled != Some(false)
                    && (node.clickable == Some(true)
                        || node.focusable == Some(true)
                        || role(node) == "textbox")
                {
                    references.insert(format!("{id}-{i}"), absolute_path(&nodes, i));
                }
            }
            generation = Some(id);
        }
        if cache.len() >= 128 && !cache.contains_key(&scope) {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, c)| c.created)
                .map(|(s, _)| s.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(
            scope.clone(),
            Capture {
                scope,
                device: browser.device.device_id.clone(),
                package: source.current_package.clone(),
                activity: source.current_activity.clone(),
                source: source.source.clone(),
                references: references.clone(),
                generation: generation.clone().unwrap(),
                created: std::time::Instant::now(),
            },
        );
    }
    let children: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.parent_index.is_none())
        .flat_map(|(i, _)| node_json(&nodes, i, mode, &references))
        .collect();
    Ok(
        json!({"version":1,"documents":[{"contextId":page.page_id,"parentContextId":null,"platform":"android","package":source.current_package,"activity":source.current_activity,"snapshotId":generation,"root":{"role":"application","name":source.current_package.clone().unwrap_or_default(),"states":{},"properties":{},"children":children}}]}),
    )
}
pub(super) fn snapshot(
    browser: &MobileBrowserSessionHandle,
    page: &MobilePageSessionHandle,
    format: &str,
    mode: &str,
) -> Result<allwright_plugin_sdk::AccessibilitySnapshotInfo, String> {
    let format = if format.is_empty() { "json" } else { format };
    let mode = if mode.is_empty() { "default" } else { mode };
    if !["json", "yaml"].contains(&format) {
        return Err("accessibility snapshot format must be json or yaml".into());
    }
    if !["default", "ai", "autoexpect", "codegen"].contains(&mode) {
        return Err("invalid accessibility snapshot mode".into());
    }
    let value = capture_source(
        browser,
        page,
        adb_dump_source(&browser.device.device_id)?,
        mode,
    )?;
    let snapshot = if format == "json" {
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
    } else {
        allwright_plugin_sdk::accessibility_yaml::to_string(&value)?
    };
    Ok(allwright_plugin_sdk::AccessibilitySnapshotInfo {
        snapshot,
        format: format.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = r#"<hierarchy rotation="0"><node index="9" class="android.widget.FrameLayout" bounds="[0,0][400,800]" enabled="true"><node index="7" class="android.widget.Button" text="yes" content-desc="Save" clickable="true" enabled="true" bounds="[0,0][100,100]"/><node index="0" class="android.widget.CheckBox" text="001" checkable="true" checked="true" enabled="false" bounds="[0,100][100,200]"/><node index="1" class="android.widget.EditText" text="secret" password="true" focusable="true" enabled="true" bounds="[0,200][100,300]"/><node class="android.widget.TextView" text="offscreen" bounds="[0,0][0,0]"/></node></hierarchy>"#;
    fn fixture() -> (
        MobileBrowserSessionHandle,
        MobilePageSessionHandle,
        UiAutomator2SourceInfo,
    ) {
        let browser = MobileBrowserSessionHandle {
            platform: MobilePlatform::Android,
            automation: MobileAutomationSessionInfo {
                backend: "android-adb".into(),
                session_id: unique_id("test"),
                note: String::new(),
            },
            device: DeviceTarget {
                platform: MobilePlatform::Android,
                device_id: "test-device".into(),
                connection_kind: DeviceConnectionKind::Emulator,
            },
        };
        let page = MobilePageSessionHandle {
            page_id: unique_id("page"),
            package_name: Some("test.app".into()),
            activity_name: None,
            webview_context: None,
        };
        let source = UiAutomator2SourceInfo {
            source: XML.into(),
            current_package: Some("test.app".into()),
            current_activity: Some(".Main".into()),
        };
        (browser, page, source)
    }
    fn selector(value: &Value) -> String {
        format!(
            "ref={}",
            value["documents"][0]["root"]["children"][0]["children"][0]["aria-ref"]
                .as_str()
                .unwrap()
        )
    }
    #[test]
    fn positional_paths_use_xml_siblings_and_round_trip() {
        let nodes = parse_android_ui_nodes(XML).unwrap();
        assert_eq!(absolute_path(&nodes, 2), "/hierarchy/node[1]/node[2]");
        for i in 0..nodes.len() {
            assert_eq!(
                resolve_absolute_path(&nodes, &absolute_path(&nodes, i)).unwrap(),
                Some(i)
            );
        }
        assert_eq!(
            resolve_absolute_path(&nodes, "/hierarchy/node[1]/node[99]").unwrap(),
            None
        );
        for path in [
            "/hierarchy/node[0]",
            "/hierarchy/node[-1]",
            "/hierarchy/node[1]/",
            "/hierarchy/node[foo]",
        ] {
            assert!(resolve_absolute_path(&nodes, path).is_err());
        }
        assert_eq!(
            matching_nodes_by_selector(&nodes, "xpath=\"/hierarchy/node[1]/node[2]\"").unwrap()[0]
                .text
                .as_deref(),
            Some("001")
        );
    }
    #[test]
    fn native_semantics_and_yaml_preserve_types_and_hide_passwords() {
        let (browser, page, source) = fixture();
        for mode in ["default", "codegen", "autoexpect", "ai"] {
            let value = capture_source(&browser, &page, source.clone(), mode).unwrap();
            let children = value["documents"][0]["root"]["children"][0]["children"]
                .as_array()
                .unwrap();
            assert_eq!(children.len(), if mode == "autoexpect" { 3 } else { 4 });
            assert_eq!(children[0]["role"], "button");
            assert_eq!(children[0]["name"], "Save");
            assert_eq!(children[1]["states"]["checked"], true);
            assert_eq!(children[1]["states"]["disabled"], true);
            assert_eq!(children[2]["name"], "");
            assert_eq!(children[0].get("aria-ref").is_some(), mode == "ai");
            assert!(children[1].get("aria-ref").is_none());
            let yaml = allwright_plugin_sdk::accessibility_yaml::to_string(&value).unwrap();
            assert!(!yaml.contains("secret"));
            assert_eq!(serde_yaml::from_str::<Value>(&yaml).unwrap(), value);
        }
        clear(&browser, &page).unwrap();
    }
    #[test]
    fn references_expire_and_non_ai_observations_invalidate_changes() {
        let (browser, page, mut source) = fixture();
        let first = capture_source(&browser, &page, source.clone(), "ai").unwrap();
        let query = normalize_selector_for_transport(&selector(&first));
        cache()
            .lock()
            .unwrap()
            .get_mut(&scope(&browser, &page))
            .unwrap()
            .created = std::time::Instant::now() - Duration::from_secs(601);
        assert!(validate_scope(&browser, &page, &query).is_err());
        let next = capture_source(&browser, &page, source.clone(), "ai").unwrap();
        let query = normalize_selector_for_transport(&selector(&next));
        source.source = source.source.replace("Save", "Delete");
        capture_source(&browser, &page, source, "default").unwrap();
        assert!(validate_scope(&browser, &page, &query).is_err());
        assert!(snapshot(&browser, &page, "invalid", "ai").is_err());
        assert!(snapshot(&browser, &page, "json", "invalid").is_err());
    }

    #[test]
    fn references_are_scoped_reused_and_invalidated_permanently() {
        let (browser, page, mut source) = fixture();
        let first = capture_source(&browser, &page, source.clone(), "ai").unwrap();
        let query = selector(&first);
        assert_eq!(
            selector(&capture_source(&browser, &page, source.clone(), "ai").unwrap()),
            query
        );
        let normalized = normalize_selector_for_transport(&query);
        validate_scope(&browser, &page, &normalized).unwrap();
        let nodes = parse_android_ui_nodes(&source.source).unwrap();
        assert_eq!(
            matching_source_nodes(&browser.device.device_id, &source, &nodes, &normalized)
                .unwrap()
                .len(),
            1
        );
        let (_, other_page, _) = fixture();
        assert!(validate_scope(&browser, &other_page, &normalized).is_err());
        let (other_browser, _, _) = fixture();
        assert!(validate_scope(&other_browser, &page, &normalized).is_err());
        // Even an attribute not projected into AndroidUiNode invalidates the generation.
        source.source = source.source.replace("rotation=\"0\"", "rotation=\"1\"");
        assert!(
            matching_source_nodes(&browser.device.device_id, &source, &nodes, &normalized)
                .unwrap_err()
                .contains("stale")
        );
        source.source = XML.into();
        assert!(
            matching_source_nodes(&browser.device.device_id, &source, &nodes, &normalized).is_err()
        );
        let second = capture_source(&browser, &page, source.clone(), "ai").unwrap();
        assert_ne!(selector(&second), query);
        let query = normalize_selector_for_transport(&selector(&second));
        source.current_activity = Some(".Other".into());
        assert!(matching_source_nodes(&browser.device.device_id, &source, &nodes, &query).is_err());
        let third = capture_source(&browser, &page, source, "ai").unwrap();
        clear(&browser, &page).unwrap();
        assert!(
            validate_scope(
                &browser,
                &page,
                &normalize_selector_for_transport(&selector(&third))
            )
            .is_err()
        );
        assert!(reference("css=\"ref=id\" css=\"Save\"").is_err());
    }
}
