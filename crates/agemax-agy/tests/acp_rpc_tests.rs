use agemax_agy::acp::{
    build_acp_initialize_request, build_acp_prompt_request, build_acp_session_new_request,
    parse_acp_json_line, AcpCardState, AcpEvent,
};
use std::path::Path;

#[test]
fn test_acp_json_rpc_initialize_and_session_update() {
    let init_req = build_acp_initialize_request(1);
    assert!(init_req.contains("\"method\":\"initialize\""));
    assert!(init_req.contains("\"protocolVersion\":1"));

    let new_sess_req = build_acp_session_new_request(2, Path::new("C:\\repo"));
    assert!(new_sess_req.contains("\"method\":\"session/new\""));

    let prompt_req = build_acp_prompt_request(3, "sess-99", "Explain main.rs");
    assert!(prompt_req.contains("\"method\":\"session/prompt\""));
    assert!(prompt_req.contains("\"sessionId\":\"sess-99\""));
    assert!(prompt_req.contains("Explain main.rs"));

    let mut state = AcpCardState::default();

    let ev_init = parse_acp_json_line(
        r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentInfo":{"name":"Antigravity ACP Server","version":"1.5.16"}}}"#,
    )
    .unwrap();
    assert!(matches!(ev_init, AcpEvent::Initialized { .. }));
    state.apply(&ev_init);
    assert_eq!(
        state.server_banner.as_deref(),
        Some("Antigravity ACP Server v1.5.16")
    );

    let ev_sess =
        parse_acp_json_line(r#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"sess-99"}}"#)
            .unwrap();
    state.apply(&ev_sess);
    assert_eq!(state.session_id.as_deref(), Some("sess-99"));

    let ev_msg = parse_acp_json_line(
        r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess-99","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hello from ACP!"}}}}"#,
    )
    .unwrap();
    state.apply(&ev_msg);
    assert!(state.latest_message.contains("Hello from ACP!"));

    let ev_tool = parse_acp_json_line(
        r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess-99","update":{"sessionUpdate":"tool_call","toolCallId":"call-1","title":"Read Cargo.toml","status":"in_progress"}}}"#,
    )
    .unwrap();
    state.apply(&ev_tool);
    assert_eq!(state.active_tools.len(), 1);
    assert_eq!(state.active_tools[0].tool, "Read Cargo.toml");

    let ev_tool_done = parse_acp_json_line(
        r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess-99","update":{"sessionUpdate":"tool_call_update","toolCallId":"call-1","status":"completed"}}}"#,
    )
    .unwrap();
    state.apply(&ev_tool_done);
    assert_eq!(state.active_tools.len(), 0);
}
