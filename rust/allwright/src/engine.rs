use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::proto;
use crate::web_lib;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

use proto::engine_service_server::{EngineService, EngineServiceServer};
use proto::{
    BrowserSessionClosedEvent, BrowserSessionCommand, BrowserSessionErrorEvent,
    BrowserSessionEvent, ChromeLaunchedEvent, ChromiumBidiInjectionEvent, ClickElementCommand,
    CloseBrowserSessionCommand, CloseTabSessionCommand, ElementClickedEvent, LaunchChromeCommand,
    NavigateTabCommand, OpenTabCommand, PingRequest, PingResponse, SessionPingCommand,
    SessionPongEvent, TabNavigatedEvent, TabOpenedEvent, TabSessionAttachedEvent,
    TabSessionClosedEvent, TabSessionCommand, TabSessionErrorEvent, TabSessionEvent,
    TabSessionPingCommand, TabSessionPongEvent, browser_session_command::Command as BrowserCommand,
    browser_session_event::Event as BrowserEvent, tab_session_command::Command as TabCommand,
    tab_session_event::Event as TabEvent,
};

static BROWSER_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static TAB_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static BIDI_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct BrowserBidiMapperState {
    bidi_session_id: String,
    mapper_target_id: String,
    mapper_session_id: String,
    package_version: String,
}

#[derive(Debug, Default)]
struct BrowserSessionState {
    launched: bool,
    cdp_websocket_url: Option<String>,
    process_id: Option<u32>,
    bidi_mapper: Option<BrowserBidiMapperState>,
}

#[derive(Debug, Clone)]
struct TabSessionState {
    browser_session_id: String,
    target_id: String,
}

#[derive(Debug, Default)]
struct EngineState {
    browser_sessions: std::collections::HashMap<String, BrowserSessionState>,
    tab_sessions: std::collections::HashMap<String, TabSessionState>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineGrpcService {
    state: Arc<Mutex<EngineState>>,
}

type BrowserSessionStream =
    Pin<Box<dyn Stream<Item = Result<BrowserSessionEvent, Status>> + Send + 'static>>;
type TabSessionStream =
    Pin<Box<dyn Stream<Item = Result<TabSessionEvent, Status>> + Send + 'static>>;

struct CommandOutcome {
    event: BrowserSessionEvent,
    should_close: bool,
}

struct TabCommandOutcome {
    events: Vec<TabSessionEvent>,
    should_close: bool,
}

fn next_browser_session_id() -> String {
    format!(
        "browser-session-{}",
        BROWSER_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_tab_session_id() -> String {
    format!(
        "tab-session-{}",
        TAB_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_bidi_session_id() -> String {
    format!(
        "bidi-session-{}",
        BIDI_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn browser_event(session_id: &str, event: BrowserEvent) -> BrowserSessionEvent {
    BrowserSessionEvent {
        session_id: session_id.to_string(),
        event: Some(event),
    }
}

fn tab_event(tab_session_id: &str, event: TabEvent) -> TabSessionEvent {
    TabSessionEvent {
        tab_session_id: tab_session_id.to_string(),
        event: Some(event),
    }
}

async fn ensure_browser_bidi_mapper(
    state: Arc<Mutex<EngineState>>,
    browser_session_id: &str,
    cdp_websocket_url: &str,
) -> Result<(BrowserBidiMapperState, bool), Status> {
    if let Some(existing) = state
        .lock()
        .await
        .browser_sessions
        .get(browser_session_id)
        .and_then(|session| session.bidi_mapper.clone())
    {
        return Ok((existing, false));
    }

    let injected = web_lib::inject_chromium_bidi_mapper(cdp_websocket_url)
        .await
        .map_err(Status::internal)?;
    let mapper_state = BrowserBidiMapperState {
        bidi_session_id: next_bidi_session_id(),
        mapper_target_id: injected.mapper_target_id,
        mapper_session_id: injected.mapper_session_id,
        package_version: injected.package_version,
    };

    let mut state = state.lock().await;
    let browser_session = state
        .browser_sessions
        .get_mut(browser_session_id)
        .ok_or_else(|| Status::internal("browser session disappeared during mapper injection"))?;

    if let Some(existing) = browser_session.bidi_mapper.clone() {
        return Ok((existing, false));
    }

    browser_session.bidi_mapper = Some(mapper_state.clone());
    Ok((mapper_state, true))
}

async fn update_browser_bidi_mapper(
    state: Arc<Mutex<EngineState>>,
    browser_session_id: &str,
    bidi_mapper: BrowserBidiMapperState,
) -> Result<(), Status> {
    let mut state = state.lock().await;
    let browser_session = state
        .browser_sessions
        .get_mut(browser_session_id)
        .ok_or_else(|| {
            Status::internal("browser session disappeared while updating BiDi mapper")
        })?;
    browser_session.bidi_mapper = Some(bidi_mapper);
    Ok(())
}

async fn handle_browser_command(
    state: Arc<Mutex<EngineState>>,
    session_id: &str,
    command: BrowserSessionCommand,
) -> Result<CommandOutcome, Status> {
    match command.command {
        Some(BrowserCommand::LaunchChrome(LaunchChromeCommand { chrome_binary })) => {
            println!(
                "[engine-lib][{session_id}] received LaunchChromeCommand with chrome_binary={:?}",
                chrome_binary
            );
            let launch =
                web_lib::open_chrome_window(chrome_binary.as_deref()).map_err(Status::internal)?;
            let initial_tab = web_lib::discover_initial_tab(&launch.cdp_websocket_url)
                .await
                .map_err(Status::internal)?;
            let initial_tab_session_id = next_tab_session_id();
            state.lock().await.browser_sessions.insert(
                session_id.to_string(),
                BrowserSessionState {
                    launched: true,
                    cdp_websocket_url: Some(launch.cdp_websocket_url.clone()),
                    process_id: Some(launch.process_id),
                    bidi_mapper: None,
                },
            );
            state.lock().await.tab_sessions.insert(
                initial_tab_session_id.clone(),
                TabSessionState {
                    browser_session_id: session_id.to_string(),
                    target_id: initial_tab.target_id,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::ChromeLaunched(ChromeLaunchedEvent {
                        browser: launch.browser,
                        note: format!("{}; {}", launch.note, initial_tab.note),
                        cdp_websocket_url: launch.cdp_websocket_url,
                        user_data_dir: launch.user_data_dir,
                        initial_tab_session_id,
                    }),
                ),
                should_close: false,
            })
        }
        Some(BrowserCommand::OpenTab(OpenTabCommand {})) => {
            println!("[engine-lib][{session_id}] received OpenTabCommand");
            let cdp_websocket_url = {
                let state = state.lock().await;
                match state.browser_sessions.get(session_id) {
                    Some(browser_session) if browser_session.launched => {
                        browser_session.cdp_websocket_url.clone().ok_or_else(|| {
                            Status::internal("browser session is missing CDP websocket metadata")
                        })?
                    }
                    Some(_) => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                BrowserEvent::Error(BrowserSessionErrorEvent {
                                    message: "browser must be launched before opening a tab"
                                        .to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                    None => {
                        return Ok(CommandOutcome {
                            event: browser_event(
                                session_id,
                                BrowserEvent::Error(BrowserSessionErrorEvent {
                                    message: "browser session is not registered".to_string(),
                                }),
                            ),
                            should_close: false,
                        });
                    }
                }
            };
            let tab = web_lib::open_chrome_tab(&cdp_websocket_url)
                .await
                .map_err(Status::internal)?;
            let tab_session_id = next_tab_session_id();
            state.lock().await.tab_sessions.insert(
                tab_session_id.clone(),
                TabSessionState {
                    browser_session_id: session_id.to_string(),
                    target_id: tab.target_id,
                },
            );

            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::TabOpened(TabOpenedEvent {
                        tab_session_id,
                        note: tab.note,
                    }),
                ),
                should_close: false,
            })
        }
        Some(BrowserCommand::Ping(SessionPingCommand { message })) => {
            println!("[engine-lib][{session_id}] received SessionPingCommand: {message}");
            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::Pong(SessionPongEvent {
                        message: if message.is_empty() {
                            "pong".to_string()
                        } else {
                            format!("pong: {message}")
                        },
                    }),
                ),
                should_close: false,
            })
        }
        Some(BrowserCommand::Close(CloseBrowserSessionCommand {})) => {
            println!("[engine-lib][{session_id}] received CloseBrowserSessionCommand");
            let process_id = state
                .lock()
                .await
                .browser_sessions
                .get(session_id)
                .and_then(|session| session.process_id);
            if let Some(process_id) = process_id {
                web_lib::close_browser_process(process_id).map_err(Status::internal)?;
            }
            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::Closed(BrowserSessionClosedEvent {
                        reason: "browser session closed by client".to_string(),
                    }),
                ),
                should_close: true,
            })
        }
        None => {
            println!("[engine-lib][{session_id}] received empty browser session command");
            Ok(CommandOutcome {
                event: browser_event(
                    session_id,
                    BrowserEvent::Error(BrowserSessionErrorEvent {
                        message: "browser session command payload is missing".to_string(),
                    }),
                ),
                should_close: false,
            })
        }
    }
}

async fn handle_tab_command(
    state: Arc<Mutex<EngineState>>,
    command: TabSessionCommand,
) -> Result<TabCommandOutcome, Status> {
    let browser_session_id = command.browser_session_id;
    let tab_session_id = command.tab_session_id;
    if browser_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                if tab_session_id.trim().is_empty() {
                    "unknown-tab-session"
                } else {
                    &tab_session_id
                },
                TabEvent::Error(TabSessionErrorEvent {
                    message: "browser_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    if tab_session_id.trim().is_empty() {
        return Ok(TabCommandOutcome {
            events: vec![tab_event(
                "unknown-tab-session",
                TabEvent::Error(TabSessionErrorEvent {
                    message: "tab_session_id is required".to_string(),
                }),
            )],
            should_close: false,
        });
    }

    let (cdp_websocket_url, target_id, existing_bidi_mapper) = {
        let state = state.lock().await;
        let browser_session = match state.browser_sessions.get(&browser_session_id) {
            Some(browser_session) => browser_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &tab_session_id,
                        TabEvent::Error(TabSessionErrorEvent {
                            message: format!(
                                "browser_session_id {browser_session_id} is not active"
                            ),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        let tab_session = match state.tab_sessions.get(&tab_session_id) {
            Some(tab_session) => tab_session,
            None => {
                return Ok(TabCommandOutcome {
                    events: vec![tab_event(
                        &tab_session_id,
                        TabEvent::Error(TabSessionErrorEvent {
                            message: format!("unknown tab_session_id {tab_session_id}"),
                        }),
                    )],
                    should_close: false,
                });
            }
        };

        if tab_session.browser_session_id != browser_session_id {
            return Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Error(TabSessionErrorEvent {
                        message: format!(
                            "tab_session_id {tab_session_id} belongs to browser_session_id {}, not {browser_session_id}",
                            tab_session.browser_session_id
                        ),
                    }),
                )],
                should_close: false,
            });
        }

        (
            browser_session.cdp_websocket_url.clone().ok_or_else(|| {
                Status::internal("browser session is missing CDP websocket metadata")
            })?,
            tab_session.target_id.clone(),
            browser_session.bidi_mapper.clone(),
        )
    };

    match command.command {
        Some(TabCommand::Ping(TabSessionPingCommand { message })) => {
            println!(
                "[engine-lib][{browser_session_id}/{tab_session_id}] received TabSessionPingCommand: {message}"
            );
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Pong(TabSessionPongEvent {
                        message: if message.is_empty() {
                            "tab-pong".to_string()
                        } else {
                            format!("tab-pong: {message}")
                        },
                    }),
                )],
                should_close: false,
            })
        }
        Some(TabCommand::Close(CloseTabSessionCommand {})) => {
            println!(
                "[engine-lib][{browser_session_id}/{tab_session_id}] received CloseTabSessionCommand"
            );
            web_lib::close_chrome_tab(&cdp_websocket_url, &target_id)
                .await
                .map_err(Status::internal)?;
            state.lock().await.tab_sessions.remove(&tab_session_id);
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Closed(TabSessionClosedEvent {
                        reason: "tab session closed by client".to_string(),
                    }),
                )],
                should_close: true,
            })
        }
        Some(TabCommand::Navigate(NavigateTabCommand { url })) => {
            println!(
                "[engine-lib][{browser_session_id}/{tab_session_id}] received NavigateTabCommand: {url}"
            );
            let navigation = web_lib::navigate_chrome_tab(&cdp_websocket_url, &target_id, &url)
                .await
                .map_err(Status::internal)?;
            let (bidi_mapper, created) = ensure_browser_bidi_mapper(
                Arc::clone(&state),
                &browser_session_id,
                &cdp_websocket_url,
            )
            .await?;
            let bidi_note = if created {
                format!(
                    "chromium-bidi mapper injected from pinned chromium-bidi@{} into hidden mapper target after navigation",
                    bidi_mapper.package_version
                )
            } else {
                format!(
                    "reused existing chromium-bidi mapper session {} from pinned chromium-bidi@{} after navigation",
                    bidi_mapper.bidi_session_id, bidi_mapper.package_version
                )
            };
            Ok(TabCommandOutcome {
                events: vec![
                    tab_event(
                        &tab_session_id,
                        TabEvent::Navigated(TabNavigatedEvent {
                            url: navigation.url,
                            note: navigation.note,
                        }),
                    ),
                    tab_event(
                        &tab_session_id,
                        TabEvent::ChromiumBidiInjection(ChromiumBidiInjectionEvent {
                            note: bidi_note,
                            bidi_session_id: bidi_mapper.bidi_session_id,
                            mapper_target_id: bidi_mapper.mapper_target_id,
                            mapper_session_id: bidi_mapper.mapper_session_id,
                            package_version: bidi_mapper.package_version,
                        }),
                    ),
                ],
                should_close: false,
            })
        }
        Some(TabCommand::ClickElement(ClickElementCommand { css_selector })) => {
            println!(
                "[engine-lib][{browser_session_id}/{tab_session_id}] received ClickElementCommand: {css_selector}"
            );
            let click = web_lib::click_element_via_bidi(
                &cdp_websocket_url,
                existing_bidi_mapper
                    .as_ref()
                    .map(|bidi_mapper| bidi_mapper.mapper_target_id.as_str()),
                &target_id,
                &css_selector,
            )
            .await
            .map_err(Status::internal)?;
            let bidi_mapper = BrowserBidiMapperState {
                bidi_session_id: existing_bidi_mapper
                    .as_ref()
                    .map(|bidi_mapper| bidi_mapper.bidi_session_id.clone())
                    .unwrap_or_else(next_bidi_session_id),
                mapper_target_id: click.mapper_target_id.clone(),
                mapper_session_id: click.mapper_session_id.clone(),
                package_version: click.package_version.clone(),
            };
            update_browser_bidi_mapper(
                Arc::clone(&state),
                &browser_session_id,
                bidi_mapper.clone(),
            )
            .await?;
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::ElementClicked(ElementClickedEvent {
                        css_selector: click.css_selector,
                        note: click.note,
                        bidi_session_id: bidi_mapper.bidi_session_id,
                    }),
                )],
                should_close: false,
            })
        }
        None => {
            println!(
                "[engine-lib][{browser_session_id}/{tab_session_id}] received empty tab session command"
            );
            Ok(TabCommandOutcome {
                events: vec![tab_event(
                    &tab_session_id,
                    TabEvent::Error(TabSessionErrorEvent {
                        message: "tab session command payload is missing".to_string(),
                    }),
                )],
                should_close: false,
            })
        }
    }
}

#[tonic::async_trait]
impl EngineService for EngineGrpcService {
    type BrowserSessionStream = BrowserSessionStream;
    type TabSessionStream = TabSessionStream;

    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
        }))
    }

    async fn browser_session(
        &self,
        request: Request<tonic::Streaming<BrowserSessionCommand>>,
    ) -> Result<Response<Self::BrowserSessionStream>, Status> {
        let session_id = next_browser_session_id();
        println!("[engine-lib][{session_id}] browser session opened");
        self.state
            .lock()
            .await
            .browser_sessions
            .insert(session_id.clone(), BrowserSessionState::default());
        let mut inbound = request.into_inner();
        let state = Arc::clone(&self.state);
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            loop {
                match inbound.message().await {
                    Ok(Some(command)) => {
                        match handle_browser_command(Arc::clone(&state), &session_id, command).await
                        {
                            Ok(outcome) => {
                                let should_close = outcome.should_close;
                                println!(
                                    "[engine-lib][{session_id}] sending browser session event"
                                );
                                if tx.send(Ok(outcome.event)).await.is_err() {
                                    println!(
                                        "[engine-lib][{session_id}] stream closed while sending event"
                                    );
                                    break;
                                }

                                if should_close {
                                    println!("[engine-lib][{session_id}] browser session closing");
                                    break;
                                }
                            }
                            Err(status) => {
                                println!(
                                    "[engine-lib][{session_id}] handler returned error: {status}"
                                );
                                let _ = tx.send(Err(status)).await;
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        println!("[engine-lib][{session_id}] client closed request stream");
                        break;
                    }
                    Err(status) => {
                        println!("[engine-lib][{session_id}] inbound stream error: {status}");
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }

            let mut state = state.lock().await;
            state.browser_sessions.remove(&session_id);
            state
                .tab_sessions
                .retain(|_, tab_session| tab_session.browser_session_id != session_id);
            println!("[engine-lib][{session_id}] browser session removed from engine state");
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn tab_session(
        &self,
        request: Request<tonic::Streaming<TabSessionCommand>>,
    ) -> Result<Response<Self::TabSessionStream>, Status> {
        let mut inbound = request.into_inner();
        let state = Arc::clone(&self.state);
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            let mut attached = false;

            loop {
                match inbound.message().await {
                    Ok(Some(command)) => {
                        let attach_browser_session_id = command.browser_session_id.clone();
                        let attach_tab_session_id = command.tab_session_id.clone();
                        if !attached {
                            attached = true;
                            println!(
                                "[engine-lib][{attach_browser_session_id}/{attach_tab_session_id}] tab session opened"
                            );
                            if tx
                                .send(Ok(tab_event(
                                    &attach_tab_session_id,
                                    TabEvent::Attached(TabSessionAttachedEvent {
                                        note: "tab session attached".to_string(),
                                    }),
                                )))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }

                        match handle_tab_command(Arc::clone(&state), command).await {
                            Ok(outcome) => {
                                let should_close = outcome.should_close;
                                for event in outcome.events {
                                    if tx.send(Ok(event)).await.is_err() {
                                        break;
                                    }
                                }

                                if should_close {
                                    break;
                                }
                            }
                            Err(status) => {
                                let _ = tx.send(Err(status)).await;
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

pub async fn serve(addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(EngineServiceServer::new(EngineGrpcService::default()))
        .serve(addr)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::engine_service_server::EngineService;

    fn test_state() -> Arc<Mutex<EngineState>> {
        Arc::new(Mutex::new(EngineState::default()))
    }

    #[tokio::test]
    async fn ping_returns_pong() {
        assert_eq!(
            EngineGrpcService::default()
                .ping(Request::new(PingRequest {}))
                .await
                .expect("ping should succeed")
                .into_inner()
                .message,
            "pong"
        );
    }

    #[tokio::test]
    async fn browser_ping_returns_session_pong() {
        let outcome = handle_browser_command(
            test_state(),
            "session-1",
            BrowserSessionCommand {
                command: Some(BrowserCommand::Ping(SessionPingCommand {
                    message: "hello".to_string(),
                })),
            },
        )
        .await
        .expect("ping command should succeed");

        assert!(!outcome.should_close);
        assert_eq!(outcome.event.session_id, "session-1");
        assert!(matches!(
            outcome.event.event,
            Some(BrowserEvent::Pong(SessionPongEvent { message })) if message == "pong: hello"
        ));
    }

    #[tokio::test]
    async fn browser_close_returns_closed_event() {
        let outcome = handle_browser_command(
            test_state(),
            "session-1",
            BrowserSessionCommand {
                command: Some(BrowserCommand::Close(CloseBrowserSessionCommand {})),
            },
        )
        .await
        .expect("close command should succeed");

        assert!(outcome.should_close);
        assert!(matches!(
            outcome.event.event,
            Some(BrowserEvent::Closed(BrowserSessionClosedEvent { reason }))
                if reason == "browser session closed by client"
        ));
    }

    #[tokio::test]
    async fn tab_ping_returns_tab_pong() {
        let state = Arc::new(Mutex::new(EngineState {
            browser_sessions: [(
                "browser-session-1".to_string(),
                BrowserSessionState {
                    launched: true,
                    cdp_websocket_url: Some(
                        "ws://127.0.0.1:9222/devtools/browser/test".to_string(),
                    ),
                    process_id: None,
                    bidi_mapper: None,
                },
            )]
            .into_iter()
            .collect(),
            tab_sessions: [(
                "tab-session-1".to_string(),
                TabSessionState {
                    browser_session_id: "browser-session-1".to_string(),
                    target_id: "target-1".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        }));
        let outcome = handle_tab_command(
            state,
            TabSessionCommand {
                browser_session_id: "browser-session-1".to_string(),
                tab_session_id: "tab-session-1".to_string(),
                command: Some(TabCommand::Ping(TabSessionPingCommand {
                    message: "hello".to_string(),
                })),
            },
        )
        .await
        .expect("tab ping command should succeed");

        assert!(!outcome.should_close);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].tab_session_id, "tab-session-1");
        assert!(matches!(
            outcome.events[0].event.as_ref(),
            Some(TabEvent::Pong(TabSessionPongEvent { message })) if message == "tab-pong: hello"
        ));
    }

    #[tokio::test]
    async fn open_tab_requires_launched_browser() {
        let state = Arc::new(Mutex::new(EngineState {
            browser_sessions: [(
                "browser-session-1".to_string(),
                BrowserSessionState {
                    launched: false,
                    cdp_websocket_url: None,
                    process_id: None,
                    bidi_mapper: None,
                },
            )]
            .into_iter()
            .collect(),
            tab_sessions: Default::default(),
        }));

        let outcome = handle_browser_command(
            state,
            "browser-session-1",
            BrowserSessionCommand {
                command: Some(BrowserCommand::OpenTab(OpenTabCommand {})),
            },
        )
        .await
        .expect("open tab should return a browser-session error event");

        assert!(matches!(
            outcome.event.event,
            Some(BrowserEvent::Error(BrowserSessionErrorEvent { message }))
                if message == "browser must be launched before opening a tab"
        ));
    }
}
