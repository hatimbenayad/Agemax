use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::IpcError;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct IpcClient {
    pipe_name: String,
}

impl IpcClient {
    pub fn new(pipe_name: impl Into<String>) -> Self {
        Self {
            pipe_name: pipe_name.into(),
        }
    }

    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, IpcError> {
        #[cfg(windows)]
        {
            let client =
                tokio::net::windows::named_pipe::ClientOptions::new().open(&self.pipe_name)?;
            let (reader, mut writer) = tokio::io::split(client);
            let mut lines = BufReader::new(reader).lines();

            let request = JsonRpcRequest::new(1, method, params);
            let mut req_str = serde_json::to_string(&request)?;
            req_str.push('\n');

            writer.write_all(req_str.as_bytes()).await?;

            if let Some(line) = lines.next_line().await? {
                let res: JsonRpcResponse = serde_json::from_str(&line)?;
                if let Some(err) = res.error {
                    return Err(IpcError::Io(std::io::Error::other(err.message)));
                }
                return Ok(res.result.unwrap_or(serde_json::Value::Null));
            }
            Err(IpcError::Closed)
        }

        #[cfg(not(windows))]
        {
            let stream = tokio::net::UnixStream::connect(&self.pipe_name).await?;
            let (reader, mut writer) = tokio::io::split(stream);
            let mut lines = BufReader::new(reader).lines();

            let request = JsonRpcRequest::new(1, method, params);
            let mut req_str = serde_json::to_string(&request)?;
            req_str.push('\n');

            writer.write_all(req_str.as_bytes()).await?;

            if let Some(line) = lines.next_line().await? {
                let res: JsonRpcResponse = serde_json::from_str(&line)?;
                if let Some(err) = res.error {
                    return Err(IpcError::Io(std::io::Error::other(err.message)));
                }
                return Ok(res.result.unwrap_or(serde_json::Value::Null));
            }
            Err(IpcError::Closed)
        }
    }
}
