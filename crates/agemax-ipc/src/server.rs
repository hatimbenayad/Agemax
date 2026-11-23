use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("I/O error during IPC communication: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server closed or unavailable")]
    Closed,
}

pub type HandlerFn = Arc<
    dyn Fn(JsonRpcRequest) -> Pin<Box<dyn Future<Output = JsonRpcResponse> + Send>> + Send + Sync,
>;

pub struct IpcServer {
    pipe_name: String,
    handler: HandlerFn,
}

impl IpcServer {
    pub fn new(pipe_name: impl Into<String>, handler: HandlerFn) -> Self {
        Self {
            pipe_name: pipe_name.into(),
            handler,
        }
    }

    #[cfg(windows)]
    pub async fn run(self) -> Result<(), IpcError> {
        let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .create(&self.pipe_name)?;

        tracing::info!("Age Max Named Pipe server listening on {}", self.pipe_name);

        loop {
            server.connect().await?;
            let connected = server;

            // Construct next server instance for subsequent clients before servicing current
            server = tokio::net::windows::named_pipe::ServerOptions::new()
                .first_pipe_instance(false)
                .create(&self.pipe_name)?;

            let handler = self.handler.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = tokio::io::split(connected);
                let mut lines = BufReader::new(reader).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                        Ok(req) => handler(req).await,
                        Err(e) => JsonRpcResponse::error(0, -32700, format!("Parse error: {e}")),
                    };

                    if let Ok(mut out) = serde_json::to_string(&response) {
                        out.push('\n');
                        if writer.write_all(out.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    }

    #[cfg(not(windows))]
    pub async fn run(self) -> Result<(), IpcError> {
        let _ = std::fs::remove_file(&self.pipe_name);
        let listener = tokio::net::UnixListener::bind(&self.pipe_name)?;

        tracing::info!("Age Max Unix domain socket listening on {}", self.pipe_name);

        loop {
            let (stream, _) = listener.accept().await?;
            let handler = self.handler.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut lines = BufReader::new(reader).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                        Ok(req) => handler(req).await,
                        Err(e) => JsonRpcResponse::error(0, -32700, format!("Parse error: {e}")),
                    };

                    if let Ok(mut out) = serde_json::to_string(&response) {
                        out.push('\n');
                        if writer.write_all(out.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    }
}
