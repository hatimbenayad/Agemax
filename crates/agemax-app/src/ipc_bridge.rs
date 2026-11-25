use agemax_core::layout::{PaneId, SplitDirection, WorkspaceId};
use agemax_core::status::AgentStatus;
use agemax_ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use agemax_ipc::server::IpcServer;
use eframe::egui;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::oneshot;

pub enum IpcAppCommand {
    WorkspaceList {
        reply: oneshot::Sender<Value>,
    },
    WorkspaceCreate {
        name: Option<String>,
        cwd: Option<String>,
        reply: oneshot::Sender<Value>,
    },
    WorkspaceClose {
        id: WorkspaceId,
        reply: oneshot::Sender<Value>,
    },
    SplitPane {
        direction: SplitDirection,
        reply: oneshot::Sender<Value>,
    },
    FocusPane {
        pane_id: PaneId,
        reply: oneshot::Sender<Value>,
    },
    SendKeys {
        pane_id: Option<PaneId>,
        keys: String,
        reply: oneshot::Sender<Value>,
    },
    ReadScreen {
        pane_id: Option<PaneId>,
        reply: oneshot::Sender<Value>,
    },
    Notify {
        title: String,
        message: String,
        status: Option<AgentStatus>,
        reply: oneshot::Sender<Value>,
    },
    StatusSet {
        pane_id: Option<PaneId>,
        status: AgentStatus,
        message: Option<String>,
        reply: oneshot::Sender<Value>,
    },
    ResumeSession {
        pane_id: Option<PaneId>,
        conversation_id: Option<String>,
        reply: oneshot::Sender<Value>,
    },
    ProfileList {
        reply: oneshot::Sender<Value>,
    },
    Activate {
        cwd: Option<String>,
        reply: oneshot::Sender<Value>,
    },
    BrowserOpen {
        url: String,
        pane_id: Option<PaneId>,
        reply: oneshot::Sender<Value>,
    },
    BrowserScreenshot {
        pane_id: Option<PaneId>,
        reply: oneshot::Sender<Value>,
    },
    BrowserEval {
        pane_id: Option<PaneId>,
        script: String,
        reply: oneshot::Sender<Value>,
    },
}

pub fn start_ipc_server(
    pipe_name: String,
    cmd_tx: crossbeam_channel::Sender<IpcAppCommand>,
    egui_ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create Tokio runtime for IPC server: {e}");
                return;
            }
        };

        rt.block_on(async move {
            let cmd_sender = cmd_tx.clone();
            let ctx = egui_ctx.clone();

            let handler = Arc::new(move |req: JsonRpcRequest| {
                let sender = cmd_sender.clone();
                let ctx_clone = ctx.clone();

                Box::pin(async move {
                    let req_id = req.id;
                    let (reply_tx, reply_rx) = oneshot::channel();

                    let cmd = match req.method.as_str() {
                        "workspace.list" => IpcAppCommand::WorkspaceList { reply: reply_tx },
                        "workspace.create" => {
                            let name = req.params["name"].as_str().map(|s| s.to_string());
                            let cwd = req.params["cwd"].as_str().map(|s| s.to_string());
                            IpcAppCommand::WorkspaceCreate {
                                name,
                                cwd,
                                reply: reply_tx,
                            }
                        }
                        "workspace.close" => {
                            let id_str = req.params["id"].as_str().unwrap_or("");
                            if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                                IpcAppCommand::WorkspaceClose {
                                    id: WorkspaceId(uuid),
                                    reply: reply_tx,
                                }
                            } else {
                                return JsonRpcResponse::error(
                                    req_id,
                                    -32602,
                                    "Invalid workspace UUID",
                                );
                            }
                        }
                        "pane.split" => {
                            let dir_str = req.params["direction"].as_str().unwrap_or("right");
                            let direction = match dir_str {
                                "down" | "vertical" => SplitDirection::Vertical,
                                _ => SplitDirection::Horizontal,
                            };
                            IpcAppCommand::SplitPane {
                                direction,
                                reply: reply_tx,
                            }
                        }
                        "pane.focus" => {
                            let id_str = req.params["pane_id"].as_str().unwrap_or("");
                            if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                                IpcAppCommand::FocusPane {
                                    pane_id: PaneId(uuid),
                                    reply: reply_tx,
                                }
                            } else {
                                return JsonRpcResponse::error(req_id, -32602, "Invalid pane UUID");
                            }
                        }
                        "pane.send_keys" => {
                            let keys = req.params["keys"].as_str().unwrap_or("").to_string();
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::SendKeys {
                                pane_id,
                                keys,
                                reply: reply_tx,
                            }
                        }
                        "pane.read_screen" => {
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::ReadScreen {
                                pane_id,
                                reply: reply_tx,
                            }
                        }
                        "notify" => {
                            let title = req.params["title"]
                                .as_str()
                                .unwrap_or("Notification")
                                .to_string();
                            let message = req.params["message"].as_str().unwrap_or("").to_string();
                            let status = req.params["status"].as_str().map(|s| match s {
                                "waiting" => AgentStatus::WaitingForInput,
                                "done" => AgentStatus::Done,
                                "failed" => AgentStatus::Failed,
                                _ => AgentStatus::Idle,
                            });
                            IpcAppCommand::Notify {
                                title,
                                message,
                                status,
                                reply: reply_tx,
                            }
                        }
                        "status.set" => {
                            let status_str = req.params["status"].as_str().unwrap_or("idle");
                            let status = match status_str {
                                "waiting" => AgentStatus::WaitingForInput,
                                "working" => AgentStatus::Working,
                                "done" => AgentStatus::Done,
                                "failed" => AgentStatus::Failed,
                                _ => AgentStatus::Idle,
                            };
                            let message = req.params["message"].as_str().map(|s| s.to_string());
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::StatusSet {
                                pane_id,
                                status,
                                message,
                                reply: reply_tx,
                            }
                        }
                        "session.resume" => {
                            let conversation_id = req.params["conversation_id"]
                                .as_str()
                                .map(|s| s.to_string());
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::ResumeSession {
                                pane_id,
                                conversation_id,
                                reply: reply_tx,
                            }
                        }
                        "profile.list" => IpcAppCommand::ProfileList { reply: reply_tx },
                        "app.activate" => {
                            let cwd = req.params["cwd"].as_str().map(|s| s.to_string());
                            IpcAppCommand::Activate {
                                cwd,
                                reply: reply_tx,
                            }
                        }
                        "browser.open" => {
                            let url = req.params["url"]
                                .as_str()
                                .unwrap_or("https://example.com")
                                .to_string();
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::BrowserOpen {
                                url,
                                pane_id,
                                reply: reply_tx,
                            }
                        }
                        "browser.screenshot" => {
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::BrowserScreenshot {
                                pane_id,
                                reply: reply_tx,
                            }
                        }
                        "browser.eval" => {
                            let script = req.params["script"].as_str().unwrap_or("").to_string();
                            let pane_id = req.params["pane_id"]
                                .as_str()
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .map(PaneId);
                            IpcAppCommand::BrowserEval {
                                pane_id,
                                script,
                                reply: reply_tx,
                            }
                        }
                        _ => return JsonRpcResponse::error(req_id, -32601, "Method not found"),
                    };

                    if sender.send(cmd).is_ok() {
                        ctx_clone.request_repaint();
                        match reply_rx.await {
                            Ok(val) => JsonRpcResponse::success(req_id, val),
                            Err(_) => JsonRpcResponse::error(req_id, -32603, "Internal app error"),
                        }
                    } else {
                        JsonRpcResponse::error(req_id, -32603, "App channel closed")
                    }
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = JsonRpcResponse> + Send>>
            });

            let server = IpcServer::new(pipe_name, handler);
            if let Err(e) = server.run().await {
                tracing::error!("Named pipe server encountered error: {e}");
            }
        });
    });
}
