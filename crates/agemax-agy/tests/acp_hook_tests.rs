use agemax_agy::acp::{parse_acp_json_line, AcpCardState, AcpEvent};
use agemax_agy::adapter::AntigravityAdapter;
use agemax_agy::profile::ProfileConfig;
use std::path::Path;

#[test]
fn test_build_acp_command_and_parse_events() {
    let profile = ProfileConfig::default_mode_a();
    let cmd = AntigravityAdapter::build_acp_command(&profile, Path::new("."));
    assert_eq!(cmd.args, vec!["acp".to_string(), "--stdio".to_string()]);

    let mut state = AcpCardState::default();
    let ev1 = parse_acp_json_line(r#"{"type":"tool_call_started","tool":"run_command","id":"t1"}"#)
        .unwrap();
    assert!(matches!(ev1, AcpEvent::ToolCallStarted { .. }));
    state.apply(&ev1);

    let ev2 =
        parse_acp_json_line(r#"{"type":"artifact_created","title":"Report","path":"report.md"}"#)
            .unwrap();
    state.apply(&ev2);

    let ev3 = parse_acp_json_line(r#"{"type":"tool_call_finished","id":"t1","ok":true}"#).unwrap();
    state.apply(&ev3);

    assert_eq!(state.active_tools.len(), 0);
    assert_eq!(state.recent_artifacts.len(), 1);
    assert_eq!(state.recent_artifacts[0].title, "Report");
}
