use std::collections::HashMap;
use std::path::Path;

/// Attempt to resolve the active conversation ID for a workspace using official cache and database files.
pub fn resolve_conversation_id(working_dir: &Path, gemini_cli_dir: &Path) -> Option<String> {
    // Tier 1: Check cache/last_conversations.json
    let cache_file = gemini_cli_dir.join("cache").join("last_conversations.json");
    if let Ok(content) = std::fs::read_to_string(&cache_file) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
            let cwd_str = working_dir.to_string_lossy().to_string();
            // Try exact match or case-insensitive match
            if let Some(id) = map.get(&cwd_str) {
                return Some(id.clone());
            }
            for (path, id) in map {
                if path.eq_ignore_ascii_case(&cwd_str) {
                    return Some(id);
                }
            }
        }
    }

    // Tier 2: Check newest .db file in conversations directory
    let conv_dir = gemini_cli_dir.join("conversations");
    if let Ok(entries) = std::fs::read_dir(conv_dir) {
        let mut newest: Option<(std::time::SystemTime, String)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("db") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            match &newest {
                                Some((t, _)) if modified > *t => {
                                    newest = Some((modified, stem.to_string()));
                                }
                                None => {
                                    newest = Some((modified, stem.to_string()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        if let Some((_, id)) = newest {
            return Some(id);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_conversation_id_from_cache() {
        let temp_dir =
            std::env::temp_dir().join(format!("agemax_test_cache_{}", uuid::Uuid::new_v4()));
        let cache_dir = temp_dir.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let json =
            r#"{"C:\\Workspace\\ProjectA": "conv-1111", "C:\\Workspace\\ProjectB": "conv-2222"}"#;
        std::fs::write(cache_dir.join("last_conversations.json"), json).unwrap();

        let id = resolve_conversation_id(Path::new("C:\\Workspace\\ProjectA"), &temp_dir);
        assert_eq!(id, Some("conv-1111".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
