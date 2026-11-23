use serde::{Deserialize, Serialize};

/// Standard JSON payload delivered to stdin by Antigravity CLI lifecycle hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(rename = "workspacePaths")]
    pub workspace_paths: Option<Vec<String>>,
    pub cwd: Option<String>,
}

/// Extract conversation ID from a raw hook payload string.
pub fn parse_session_start_hook_payload(json_str: &str) -> Option<String> {
    serde_json::from_str::<HookPayload>(json_str)
        .ok()
        .and_then(|p| p.conversation_id)
}

/// Generate a valid `hooks.json` content for registering a `SessionStart` hook with Age Max.
pub fn generate_session_start_hook_config(command: &str, args: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "command": command,
                "args": args
            }]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hook_payload_valid() {
        let json = r#"{
            "conversationId": "conv-uuid-1234-abcd",
            "workspacePaths": ["C:\\Projects\\test"],
            "cwd": "C:\\Projects\\test"
        }"#;

        let conv_id = parse_session_start_hook_payload(json);
        assert_eq!(conv_id, Some("conv-uuid-1234-abcd".to_string()));
    }

    #[test]
    fn test_parse_hook_payload_missing_id() {
        let json = r#"{"cwd": "C:\\Projects\\test"}"#;
        assert_eq!(parse_session_start_hook_payload(json), None);
    }
}
