use std::collections::HashSet;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::proto;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

use proto::engine_service_server::{EngineService, EngineServiceServer};
use proto::{
    BrowserSessionClosedEvent, BrowserSessionCommand, BrowserSessionErrorEvent,
    BrowserSessionEvent, CloseBrowserSessionCommand, CloseTabSessionCommand, OpenTabCommand,
    PingRequest, PingResponse, SessionPingCommand, SessionPongEvent, TabSessionAttachedEvent,
    TabSessionClosedEvent, TabSessionCommand, TabSessionErrorEvent, TabSessionEvent,
    TabSessionPingCommand, TabSessionPongEvent, browser_session_command::Command as BrowserCommand,
    browser_session_event::Event as BrowserEvent, tab_session_command::Command as TabCommand,
    tab_session_event::Event as TabEvent,
};

static BROWSER_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default)]
pub struct EngineGrpcService {
    browser_sessions: Arc<Mutex<HashSet<String>>>,
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

fn missing_plugin_message(command_name: &str, plugin_id: &str) -> String {
    format!(
        "{command_name} requires the `{plugin_id}` surface plugin. Install it with `allwright plugin install {plugin_id}` and run the plugin-backed CLI server."
    )
}

async fn handle_browser_command(
    browser_sessions: Arc<Mutex<HashSet<String>>>,
    session_id: &str,
    command: BrowserSessionCommand,
) -> Result<CommandOutcome, Status> {
    match command.command {
        Some(BrowserCommand::LaunchChrome(_)) => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                BrowserEvent::Error(BrowserSessionErrorEvent {
                    message: missing_plugin_message("LaunchChromeCommand", "web"),
                }),
            ),
            should_close: false,
        }),
        Some(BrowserCommand::OpenTab(OpenTabCommand { .. })) => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                BrowserEvent::Error(BrowserSessionErrorEvent {
                    message: missing_plugin_message("OpenTabCommand", "web"),
                }),
            ),
            should_close: false,
        }),
        Some(BrowserCommand::Ping(SessionPingCommand { message })) => Ok(CommandOutcome {
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
        }),
        Some(BrowserCommand::Close(CloseBrowserSessionCommand {})) => {
            browser_sessions.lock().await.remove(session_id);
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
        None => Ok(CommandOutcome {
            event: browser_event(
                session_id,
                BrowserEvent::Error(BrowserSessionErrorEvent {
                    message: "browser session command payload is missing".to_string(),
                }),
            ),
            should_close: false,
        }),
    }
}

async fn handle_tab_command(command: TabSessionCommand) -> Result<TabCommandOutcome, Status> {
    let browser_session_id = command.browser_session_id;
    let tab_session_id = if command.tab_session_id.trim().is_empty() {
        "unknown-tab-session".to_string()
    } else {
        command.tab_session_id
    };

    match command.command {
        Some(TabCommand::Ping(TabSessionPingCommand { message })) => Ok(TabCommandOutcome {
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
        }),
        Some(TabCommand::Close(CloseTabSessionCommand {})) => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &tab_session_id,
                TabEvent::Closed(TabSessionClosedEvent {
                    reason: "tab session closed by client".to_string(),
                }),
            )],
            should_close: true,
        }),
        Some(_) => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &tab_session_id,
                TabEvent::Error(TabSessionErrorEvent {
                    message: format!(
                        "tab command for browser session `{browser_session_id}` requires a surface plugin at runtime; the lightweight core does not bundle platform implementations"
                    ),
                }),
            )],
            should_close: false,
        }),
        None => Ok(TabCommandOutcome {
            events: vec![tab_event(
                &tab_session_id,
                TabEvent::Error(TabSessionErrorEvent {
                    message: "tab session command payload is missing".to_string(),
                }),
            )],
            should_close: false,
        }),
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
        self.browser_sessions
            .lock()
            .await
            .insert(session_id.clone());

        let browser_sessions = Arc::clone(&self.browser_sessions);
        let mut commands = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(command) = commands.message().await.transpose() {
                match command {
                    Ok(command) => match handle_browser_command(
                        Arc::clone(&browser_sessions),
                        &session_id,
                        command,
                    )
                    .await
                    {
                        Ok(outcome) => {
                            let should_close = outcome.should_close;
                            if tx.send(Ok(outcome.event)).await.is_err() {
                                break;
                            }
                            if should_close {
                                break;
                            }
                        }
                        Err(status) => {
                            let _ = tx.send(Err(status)).await;
                            break;
                        }
                    },
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
            browser_sessions.lock().await.remove(&session_id);
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::BrowserSessionStream
        ))
    }

    async fn tab_session(
        &self,
        request: Request<tonic::Streaming<TabSessionCommand>>,
    ) -> Result<Response<Self::TabSessionStream>, Status> {
        let mut commands = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            let mut attached = false;
            while let Some(command) = commands.message().await.transpose() {
                match command {
                    Ok(command) => {
                        let tab_session_id = if command.tab_session_id.trim().is_empty() {
                            "unknown-tab-session".to_string()
                        } else {
                            command.tab_session_id.clone()
                        };

                        if !attached {
                            attached = true;
                            if tx
                                .send(Ok(tab_event(
                                    &tab_session_id,
                                    TabEvent::Attached(TabSessionAttachedEvent {
                                        note: "tab session attached to lightweight allwright core"
                                            .to_string(),
                                    }),
                                )))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }

                        match handle_tab_command(command).await {
                            Ok(outcome) => {
                                let should_close = outcome.should_close;
                                for event in outcome.events {
                                    if tx.send(Ok(event)).await.is_err() {
                                        return;
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
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::TabSessionStream
        ))
    }
}

pub async fn serve(listen_addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    let service = EngineGrpcService::default();
    Server::builder()
        .add_service(EngineServiceServer::new(service))
        .serve(listen_addr)
        .await
}
