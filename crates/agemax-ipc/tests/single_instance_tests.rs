use agemax_ipc::activation::{try_activate_existing_instance, LaunchArgs};
use agemax_ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use agemax_ipc::server::IpcServer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

#[test]
fn test_launch_args_parsing() {
    let bare = LaunchArgs::parse_from(["agemax-app.exe"]);
    assert_eq!(bare.cwd, None);
    assert_eq!(bare.pipe, None);

    let positional = LaunchArgs::parse_from(["agemax-app.exe", r"C:\projects\demo"]);
    assert_eq!(positional.cwd, Some(PathBuf::from(r"C:\projects\demo")));

    let flagged = LaunchArgs::parse_from([
        "agemax-app.exe",
        "--cwd",
        r"C:\work\app",
        "--pipe",
        r"\\.\pipe\custom-pipe",
    ]);
    assert_eq!(flagged.cwd, Some(PathBuf::from(r"C:\work\app")));
    assert_eq!(flagged.pipe, Some(r"\\.\pipe\custom-pipe".to_string()));
}

#[test]
fn test_single_instance_activation_over_named_pipe() {
    let pipe_name = if cfg!(windows) {
        format!(r"\\.\pipe\agemax-single-test-{}", uuid::Uuid::new_v4())
    } else {
        format!("/tmp/agemax-single-test-{}.sock", uuid::Uuid::new_v4())
    };

    // Calling before any server is listening must return false cleanly without panicking.
    assert!(!try_activate_existing_instance(
        &pipe_name,
        Some(std::path::Path::new(r"C:\repo"))
    ));

    let received_cwd: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let received_clone = Arc::clone(&received_cwd);

    let server_pipe = pipe_name.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test tokio runtime");
        rt.block_on(async move {
            let handler = Arc::new(move |req: JsonRpcRequest| {
                let captured = Arc::clone(&received_clone);
                Box::pin(async move {
                    if req.method == "app.activate" {
                        let cwd = req.params["cwd"].as_str().map(|s| s.to_string());
                        *captured.lock().unwrap() = cwd;
                        JsonRpcResponse::success(req.id, serde_json::json!({ "activated": true }))
                    } else {
                        JsonRpcResponse::error(req.id, -32601, "Method not found")
                    }
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = JsonRpcResponse> + Send>>
            });
            let server = IpcServer::new(server_pipe, handler);
            tokio::select! {
                _ = server.run() => {},
                _ = sleep(Duration::from_secs(3)) => {},
            }
        });
    });

    std::thread::sleep(Duration::from_millis(80));

    let ok = try_activate_existing_instance(&pipe_name, Some(std::path::Path::new(r"C:\my\repo")));
    assert!(ok, "Expected existing instance pipe activation to succeed");
    assert_eq!(received_cwd.lock().unwrap().as_deref(), Some(r"C:\my\repo"));
}
