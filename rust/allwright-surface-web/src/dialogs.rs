//! One-shot, context-scoped JavaScript dialog hooks. All automation uses BiDi.
use super::*;
use allwright_plugin_sdk::DialogInfo;

const OPENED: &str = "browsingContext.userPromptOpened";

pub(super) fn interrupted_action_result(
    command: &Value,
    event: &Value,
    script_action: bool,
) -> Option<Value> {
    let method = command.get("method").and_then(Value::as_str)?;
    let context = match method {
        "input.performActions" => command.pointer("/params/context"),
        "script.evaluate" if script_action => command.pointer("/params/target/context"),
        _ => return None,
    }?
    .as_str()?;
    if event.get("method").and_then(Value::as_str) != Some(OPENED)
        || event.pointer("/params/context").and_then(Value::as_str) != Some(context)
    {
        return None;
    }
    // Only void input actions opt into this boundary; never fabricate query data.
    let result = if method == "script.evaluate" {
        json!({"type": "success", "result": {"type": "undefined"}})
    } else {
        json!({})
    };
    Some(json!({"type": "success", "id": command["id"], "result": result}))
}

static BROWSERS: OnceLock<Mutex<HashMap<u32, BrowserSessionHandle>>> = OnceLock::new();
pub(super) async fn track_browser(info: &BrowserLaunchInfo) {
    BROWSERS
        .get_or_init(Default::default)
        .lock()
        .await
        .insert(info.process_id, info.browser_session.clone());
}
pub(super) async fn close_browser(process_id: u32) {
    let browser = BROWSERS
        .get_or_init(Default::default)
        .lock()
        .await
        .remove(&process_id);
    if let Some(browser) = browser {
        let mut pages = Vec::new();
        for registry in [
            HOOKS.get_or_init(Default::default),
            DIALOGS.get_or_init(Default::default),
            IDLE.get_or_init(Default::default),
        ] {
            pages.extend(
                registry
                    .lock()
                    .await
                    .values()
                    .filter(|s| s.browser == browser)
                    .map(|s| s.page.clone()),
            );
        }
        for page in pages {
            close_page(&browser, &page).await;
        }
    }
}

static HOOKS: OnceLock<Mutex<HashMap<String, DialogState>>> = OnceLock::new();
static DIALOGS: OnceLock<Mutex<HashMap<String, DialogState>>> = OnceLock::new();
// Keep the subscribed connection and its event queue between one-shot hooks.
// Resuming one prompt can synchronously open another before registration runs.
static IDLE: OnceLock<Mutex<HashMap<String, DialogState>>> = OnceLock::new();

struct DialogState {
    browser: BrowserSessionHandle,
    page: PageSessionHandle,
    context: String,
    connection: FileChooserHookState,
    kind: String,
    default_value: String,
}

pub(super) async fn register(
    browser: &BrowserSessionHandle,
    page: &PageSessionHandle,
) -> Result<HookRegistration, String> {
    let mut hooks = HOOKS.get_or_init(Default::default).lock().await;
    if hooks
        .values()
        .any(|s| &s.browser == browser && &s.page == page)
        || DIALOGS
            .get_or_init(Default::default)
            .lock()
            .await
            .values()
            .any(|s| &s.browser == browser && &s.page == page)
    {
        return Err("a dialog hook or pending dialog already exists on this page".into());
    }
    let id = next_file_chooser_id("dialog-hook");
    let mut idle = IDLE.get_or_init(Default::default).lock().await;
    let previous = idle
        .iter()
        .find(|(_, s)| &s.browser == browser && &s.page == page)
        .map(|(id, _)| id.clone());
    if let Some(previous) = previous {
        hooks.insert(id.clone(), idle.remove(&previous).unwrap());
        return Ok(HookRegistration {
            opaque_state: json!({"hook_type": "dialog", "registration_id": id}).to_string(),
        });
    }
    drop(idle);
    let (context, connection) = match (browser, page) {
        (
            BrowserSessionHandle::Chromium { cdp_websocket_url },
            PageSessionHandle::Chromium {
                mapper_target_id,
                browsing_context_id,
                ..
            },
        ) => {
            let (mut connection, mapper, context) = chromium_bidi_session(
                cdp_websocket_url,
                mapper_target_id.as_deref(),
                browsing_context_id.as_deref(),
            )
            .await?;
            let response = connection.send_bidi_command(&mapper.mapper_session_id, &json!({
                "id": 7101, "method": "session.subscribe", "params": { "events": [OPENED], "contexts": [&context] }
            })).await?;
            (
                context,
                FileChooserHookState::Chromium {
                    connection,
                    mapper_session_id: mapper.mapper_session_id,
                    mapper_target_id: mapper.mapper_target_id,
                    subscription_id: required_string(&response, "/result/subscription")?,
                },
            )
        }
        (
            BrowserSessionHandle::Firefox { connection_id, .. },
            PageSessionHandle::Firefox {
                browsing_context_id,
            },
        ) => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let response = sessions
                .get_mut(connection_id)
                .unwrap()
                .connection
                .send_command(
                    "session.subscribe",
                    json!({ "events": [OPENED], "contexts": [browsing_context_id] }),
                )
                .await?;
            (
                browsing_context_id.clone(),
                FileChooserHookState::Firefox {
                    connection_id: connection_id.clone(),
                    subscription_id: required_string(&response, "/result/subscription")?,
                },
            )
        }
        _ => return Err("dialog browser/page backend mismatch".into()),
    };
    hooks.insert(
        id.clone(),
        DialogState {
            browser: browser.clone(),
            page: page.clone(),
            context,
            connection,
            kind: String::new(),
            default_value: String::new(),
        },
    );
    Ok(HookRegistration {
        opaque_state: json!({"hook_type": "dialog", "registration_id": id}).to_string(),
    })
}

pub(super) async fn poll(
    browser: &BrowserSessionHandle,
    state: &Value,
) -> Result<HookResult, String> {
    let id = required_string(state, "/registration_id")?;
    let mut hooks = HOOKS.get_or_init(Default::default).lock().await;
    let hook = hooks
        .get_mut(&id)
        .ok_or("dialog hook is no longer available")?;
    if &hook.browser != browser {
        return Err("dialog hook belongs to another browser".into());
    }
    let event = match &mut hook.connection {
        FileChooserHookState::Chromium {
            connection,
            mapper_session_id,
            ..
        } => {
            connection
                .poll_bidi_event_for_context(mapper_session_id, OPENED, Some(&hook.context))
                .await?
        }
        FileChooserHookState::Firefox { connection_id, .. } => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            sessions
                .get_mut(connection_id)
                .unwrap()
                .connection
                .poll_event_for_context(OPENED, Some(&hook.context))
                .await?
        }
    }
    .ok_or("dialog hook is still waiting for a dialog")?;
    hook.kind = required_string(&event, "/params/type")?;
    hook.default_value = event
        .pointer("/params/defaultValue")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let dialog_id = next_file_chooser_id("dialog");
    let result = DialogInfo {
        dialog_id: dialog_id.clone(),
        kind: hook.kind.clone(),
        message: required_string(&event, "/params/message")?,
        default_value: event
            .pointer("/params/defaultValue")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    };
    DIALOGS
        .get_or_init(Default::default)
        .lock()
        .await
        .insert(dialog_id, hooks.remove(&id).unwrap());
    Ok(HookResult::Dialog(result))
}

pub(super) async fn close_page(browser: &BrowserSessionHandle, page: &PageSessionHandle) {
    // Drop retained subscriptions/connections when their owning page is closed.
    for registry in [
        HOOKS.get_or_init(Default::default),
        DIALOGS.get_or_init(Default::default),
        IDLE.get_or_init(Default::default),
    ] {
        let mut states = registry.lock().await;
        let ids: Vec<_> = states
            .iter()
            .filter(|(_, s)| &s.browser == browser && &s.page == page)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(mut state) = states.remove(&id) {
                match &mut state.connection {
                    FileChooserHookState::Chromium {
                        connection,
                        mapper_session_id,
                        subscription_id,
                        ..
                    } => {
                        // Dismissing a prompt resumes page script, which can immediately
                        // open another prompt. Drain a bounded chain before unsubscribing.
                        for attempt in 0..32 {
                            let result = timeout(
                                Duration::from_millis(250),
                                connection.send_bidi_command(
                                    mapper_session_id,
                                    &json!({
                                        "id": 7200 + attempt,
                                        "method": "browsingContext.handleUserPrompt",
                                        "params": { "context": state.context, "accept": false }
                                    }),
                                ),
                            )
                            .await;
                            if !matches!(result, Ok(Ok(_))) {
                                break;
                            }
                            sleep(Duration::from_millis(10)).await;
                        }
                        let _ = connection.send_bidi_command(mapper_session_id, &json!({ "id": 7299, "method": "session.unsubscribe", "params": { "subscriptions": [subscription_id] } })).await;
                    }
                    FileChooserHookState::Firefox {
                        connection_id,
                        subscription_id,
                    } => {
                        if let Ok(mut sessions) = firefox_session_guard(connection_id).await {
                            let connection =
                                &mut sessions.get_mut(connection_id).unwrap().connection;
                            for _ in 0..32 {
                                let result = timeout(
                                    Duration::from_millis(250),
                                    connection.send_command(
                                        "browsingContext.handleUserPrompt",
                                        json!({ "context": state.context, "accept": false }),
                                    ),
                                )
                                .await;
                                if !matches!(result, Ok(Ok(_))) {
                                    break;
                                }
                                sleep(Duration::from_millis(10)).await;
                            }
                            let _ = connection
                                .send_command(
                                    "session.unsubscribe",
                                    json!({ "subscriptions": [subscription_id] }),
                                )
                                .await;
                        }
                    }
                }
            }
        }
    }
}

pub(super) async fn handle(
    browser: &BrowserSessionHandle,
    page: &PageSessionHandle,
    id: &str,
    accept: bool,
    text: Option<String>,
) -> Result<(), String> {
    let mut dialogs = DIALOGS.get_or_init(Default::default).lock().await;
    let dialog = dialogs
        .get_mut(id)
        .ok_or("dialog is unknown or already handled")?;
    if &dialog.browser != browser || &dialog.page != page {
        return Err("dialog belongs to another page".into());
    }
    if text.is_some() && (!accept || dialog.kind != "prompt") {
        return Err("prompt text requires accepting a prompt dialog".into());
    }
    let mut params = json!({ "context": dialog.context, "accept": accept });
    if accept && dialog.kind == "prompt" {
        params["userText"] = json!(text.unwrap_or_else(|| dialog.default_value.clone()));
    }
    match &mut dialog.connection {
        FileChooserHookState::Chromium {
            connection,
            mapper_session_id,
            ..
        } => {
            connection.send_bidi_command(mapper_session_id, &json!({ "id": 7102, "method": "browsingContext.handleUserPrompt", "params": params })).await?;
        }
        FileChooserHookState::Firefox { connection_id, .. } => {
            let mut sessions = firefox_session_guard(connection_id).await?;
            let connection = &mut sessions.get_mut(connection_id).unwrap().connection;
            connection
                .send_command("browsingContext.handleUserPrompt", params)
                .await?;
        }
    }
    let state = dialogs.remove(id).unwrap();
    IDLE.get_or_init(Default::default)
        .lock()
        .await
        .insert(id.to_string(), state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_matching_prompt_events_interrupt_input() {
        let command = json!({"method":"input.performActions", "params":{"context":"page"}});
        let event = json!({"method": OPENED, "params":{"context":"page"}});
        assert!(interrupted_action_result(&command, &event, false).is_some());
        assert!(
            interrupted_action_result(
                &command,
                &json!({"method": OPENED, "params":{"context":"other"}}),
                false
            )
            .is_none()
        );
        assert!(
            interrupted_action_result(
                &json!({"method":"browsingContext.navigate", "params":{"context":"page"}}),
                &event,
                true
            )
            .is_none()
        );
        assert!(
            interrupted_action_result(
                &command,
                &json!({"method":"browsingContext.userPromptClosed", "params":{"context":"page"}}),
                false
            )
            .is_none()
        );
        let script = json!({"id": 7, "method": "script.evaluate", "params": {"target": {"context": "page"}}});
        assert!(
            interrupted_action_result(&script, &event, false).is_none(),
            "reads must not yield fabricated data"
        );
        let result = interrupted_action_result(&script, &event, true).unwrap();
        assert_eq!(result["id"], 7);
        assert_eq!(result["result"]["type"], "success");
        assert!(
            interrupted_action_result(
                &script,
                &json!({"method": OPENED, "params":{"context":"other"}}),
                true
            )
            .is_none()
        );
    }

    #[tokio::test]
    async fn invalid_dialog_ownership_and_input_fail_before_browser_work() {
        let browser = BrowserSessionHandle::Firefox {
            connection_id: "test-missing".into(),
            bidi_session_id: "test".into(),
        };
        let page = PageSessionHandle::Firefox {
            browsing_context_id: "page".into(),
        };
        let other = PageSessionHandle::Firefox {
            browsing_context_id: "other".into(),
        };
        let id = next_file_chooser_id("test-dialog");
        DIALOGS.get_or_init(Default::default).lock().await.insert(
            id.clone(),
            DialogState {
                browser: browser.clone(),
                page: page.clone(),
                context: "page".into(),
                kind: "alert".into(),
                default_value: String::new(),
                connection: FileChooserHookState::Firefox {
                    connection_id: "test-missing".into(),
                    subscription_id: "test".into(),
                },
            },
        );
        assert!(
            handle(&browser, &other, &id, true, None)
                .await
                .unwrap_err()
                .contains("another page")
        );
        assert!(
            handle(&browser, &page, &id, true, Some(String::new()))
                .await
                .unwrap_err()
                .contains("prompt text")
        );
        assert!(
            handle(&browser, &page, &id, false, Some("text".into()))
                .await
                .unwrap_err()
                .contains("prompt text")
        );
        assert!(
            register(&browser, &page)
                .await
                .unwrap_err()
                .contains("already exists")
        );
        DIALOGS
            .get_or_init(Default::default)
            .lock()
            .await
            .remove(&id);
        assert!(
            handle(&browser, &page, &id, true, None)
                .await
                .unwrap_err()
                .contains("already handled")
        );
    }
}
