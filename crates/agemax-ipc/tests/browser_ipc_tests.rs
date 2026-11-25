use agemax_ipc::client::IpcClient;
use agemax_ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use agemax_ipc::server::IpcServer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_browser_protocol_payload_roundtrip() {
    let pipe_name = if cfg!(windows) {
        format!(r"\\.\pipe\agemax-browser-test-{}", uuid::Uuid::new_v4())
    } else {
        format!("/tmp/agemax-browser-test-{}.sock", uuid::Uuid::new_v4())
    };

    let handler = Arc::new(|req: JsonRpcRequest| {
        Box::pin(async move {
            match req.method.as_str() {
                "browser.open" => JsonRpcResponse::success(
                    req.id,
                    serde_json::json!({
                        "pane_id": "pane-web-1",
                        "url": req.params["url"]
                    }),
                ),
                "browser.eval" => JsonRpcResponse::success(
                    req.id,
                    serde_json::json!({
                        "result": "document.title => Age Max"
                    }),
                ),
                "browser.screenshot" => JsonRpcResponse::success(
                    req.id,
                    serde_json::json!({
                        "path": "C:\\captures\\pane-web-1.png",
                        "bytes": 1024
                    }),
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
    let open_res = client
        .call(
            "browser.open",
            serde_json::json!({ "url": "http://localhost:5173" }),
        )
        .await
        .unwrap();
    assert_eq!(open_res["url"], "http://localhost:5173");

    let eval_res = client
        .call(
            "browser.eval",
            serde_json::json!({ "pane_id": "pane-web-1", "script": "document.title" }),
        )
        .await
        .unwrap();
    assert!(eval_res["result"].as_str().unwrap().contains("Age Max"));

    let shot_res = client
        .call(
            "browser.screenshot",
            serde_json::json!({ "pane_id": "pane-web-1" }),
        )
        .await
        .unwrap();
    assert_eq!(shot_res["bytes"], 1024);
}
