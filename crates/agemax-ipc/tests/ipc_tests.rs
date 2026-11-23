use agemax_ipc::client::IpcClient;
use agemax_ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use agemax_ipc::server::IpcServer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_ipc_roundtrip_named_pipe() {
    let pipe_name = if cfg!(windows) {
        format!(r"\\.\pipe\agemax-test-{}", uuid::Uuid::new_v4())
    } else {
        format!("/tmp/agemax-test-{}.sock", uuid::Uuid::new_v4())
    };

    let handler = Arc::new(|req: JsonRpcRequest| {
        Box::pin(async move {
            match req.method.as_str() {
                "ping" => JsonRpcResponse::success(req.id, serde_json::json!({ "reply": "pong" })),
                "split" => JsonRpcResponse::success(
                    req.id,
                    serde_json::json!({ "pane_id": "pane-123", "direction": req.params["direction"] }),
                ),
                _ => JsonRpcResponse::error(req.id, -32601, "Method not found"),
            }
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = JsonRpcResponse> + Send>>
    });

    let server = IpcServer::new(pipe_name.clone(), handler);
    tokio::spawn(async move {
        let _ = server.run().await;
    });

    sleep(Duration::from_millis(50)).await;

    let client = IpcClient::new(pipe_name);
    let ping_res = client.call("ping", serde_json::json!({})).await.unwrap();
    assert_eq!(ping_res["reply"], "pong");

    let split_res = client
        .call("split", serde_json::json!({ "direction": "right" }))
        .await
        .unwrap();
    assert_eq!(split_res["pane_id"], "pane-123");
    assert_eq!(split_res["direction"], "right");
}
