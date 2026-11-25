use agemax_agy::adapter::{AgentLaunchPreset, AntigravityAdapter};
use agemax_agy::profile::ProfileConfig;
use std::path::Path;

#[test]
fn test_preset_sandbox_flag_appended() {
    let profile = ProfileConfig::default_mode_a();
    let adapter = AntigravityAdapter::new();

    let cmd_plain = adapter
        .build_preset_command_with_options(
            AgentLaunchPreset::SafePlanMode,
            &profile,
            Path::new("."),
            false,
        )
        .unwrap();
    assert_eq!(cmd_plain.args, vec!["--mode", "plan"]);

    let cmd_sandbox = adapter
        .build_preset_command_with_options(
            AgentLaunchPreset::SafePlanMode,
            &profile,
            Path::new("."),
            true,
        )
        .unwrap();
    assert_eq!(cmd_sandbox.args, vec!["--sandbox", "--mode", "plan"]);

    assert_eq!(
        AntigravityAdapter::shell_line_for_preset_with_options(
            &AgentLaunchPreset::NewSession,
            true
        ),
        "agy --sandbox"
    );
    assert_eq!(
        AntigravityAdapter::shell_line_for_preset_with_options(
            &AgentLaunchPreset::ContinueLast,
            true
        ),
        "agy --sandbox -c"
    );
}
