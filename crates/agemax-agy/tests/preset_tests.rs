use agemax_agy::adapter::{AgentLaunchPreset, AntigravityAdapter};
use agemax_agy::profile::ProfileConfig;
use std::path::Path;

#[test]
fn test_build_preset_commands_and_shell_lines() {
    let adapter = AntigravityAdapter::new();
    let profile = ProfileConfig::default_mode_a();
    let cwd = Path::new(".");

    let new_cmd = adapter
        .build_preset_command(AgentLaunchPreset::NewSession, &profile, cwd)
        .expect("new session cmd");
    assert!(new_cmd.args.is_empty());
    assert_eq!(
        new_cmd.env.get("AGY_CLI_OSC7501").map(String::as_str),
        Some("1")
    );

    let plan_cmd = adapter
        .build_preset_command(AgentLaunchPreset::SafePlanMode, &profile, cwd)
        .expect("plan mode cmd");
    assert_eq!(plan_cmd.args, vec!["--mode", "plan"]);

    let cont_cmd = adapter
        .build_preset_command(AgentLaunchPreset::ContinueLast, &profile, cwd)
        .expect("continue last cmd");
    assert_eq!(cont_cmd.args, vec!["-c"]);

    let resume_cmd = adapter
        .build_preset_command(
            AgentLaunchPreset::ResumeConversation("conv-abc-99".to_string()),
            &profile,
            cwd,
        )
        .expect("resume cmd");
    assert_eq!(resume_cmd.args, vec!["--conversation", "conv-abc-99"]);

    assert_eq!(
        AntigravityAdapter::shell_line_for_preset(&AgentLaunchPreset::SafePlanMode),
        "agy --mode plan"
    );
    assert_eq!(
        AntigravityAdapter::shell_line_for_preset(&AgentLaunchPreset::ContinueLast),
        "agy -c"
    );
}
