use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("I/O error during profile management: {0}")]
    Io(#[from] std::io::Error),
    #[error("Profile directory '{0}' already exists")]
    AlreadyExists(PathBuf),
    #[error("Profile '{0}' not found")]
    NotFound(String),
}

/// Profile configuration defining account overrides and storage isolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileConfig {
    pub name: String,
    pub account_override: Option<String>,
    pub isolate_storage: bool,
    pub profile_dir: PathBuf,
}

impl ProfileConfig {
    /// Mode A: Default user profile sharing the existing OS environment.
    pub fn default_mode_a() -> Self {
        let home = directories::BaseDirs::new()
            .map(|b| b.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            name: "default".to_string(),
            account_override: None,
            isolate_storage: false,
            profile_dir: home,
        }
    }
}

/// Manages Mode A and Mode B profiles under `%LOCALAPPDATA%\AgeMax\profiles\`.
pub struct ProfileManager {
    base_dir: PathBuf,
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileManager {
    pub fn new() -> Self {
        let base_dir = directories::ProjectDirs::from("com", "agemax", "AgeMax")
            .map(|p| p.data_local_dir().join("profiles"))
            .unwrap_or_else(|| PathBuf::from(".agemax").join("profiles"));

        Self { base_dir }
    }

    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// List all configured profiles, starting with the default Mode A profile.
    pub fn list_profiles(&self) -> Vec<ProfileConfig> {
        let mut profiles = vec![ProfileConfig::default_mode_a()];

        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        profiles.push(ProfileConfig {
                            name,
                            account_override: None,
                            isolate_storage: true,
                            profile_dir: entry.path(),
                        });
                    }
                }
            }
        }

        profiles
    }

    /// Create a new Mode B profile with isolated storage directory.
    /// If `shared_history` is true, junctions `.gemini\antigravity-cli` to the shared user directory.
    pub fn create_profile(
        &self,
        name: &str,
        account_override: Option<String>,
        shared_history: bool,
    ) -> Result<ProfileConfig, ProfileError> {
        let profile_dir = self.base_dir.join(name);
        if profile_dir.exists() {
            return Err(ProfileError::AlreadyExists(profile_dir));
        }

        std::fs::create_dir_all(&profile_dir)?;

        if shared_history {
            if let Some(base_dirs) = directories::BaseDirs::new() {
                let user_gemini = base_dirs.home_dir().join(".gemini").join("antigravity-cli");
                let profile_gemini_parent = profile_dir.join(".gemini");
                let profile_gemini = profile_gemini_parent.join("antigravity-cli");

                if user_gemini.exists() {
                    std::fs::create_dir_all(&profile_gemini_parent)?;
                    let _ = create_junction(&profile_gemini, &user_gemini);
                }
            }
        }

        Ok(ProfileConfig {
            name: name.to_string(),
            account_override,
            isolate_storage: true,
            profile_dir,
        })
    }
}

/// Create an NTFS directory junction on Windows or a symlink on Unix.
#[cfg(windows)]
pub fn create_junction(junction_path: &Path, target_path: &Path) -> std::io::Result<()> {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(junction_path)
        .arg(target_path)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

#[cfg(not(windows))]
pub fn create_junction(junction_path: &Path, target_path: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target_path, junction_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_a_default_profile() {
        let p = ProfileConfig::default_mode_a();
        assert_eq!(p.name, "default");
        assert!(!p.isolate_storage);
        assert_eq!(p.account_override, None);
    }

    #[test]
    fn test_profile_manager_lifecycle() {
        let temp_dir =
            std::env::temp_dir().join(format!("agemax_test_profiles_{}", uuid::Uuid::new_v4()));
        let pm = ProfileManager::with_base_dir(temp_dir.clone());

        let p1 = pm
            .create_profile("work", Some("acc-123".to_string()), false)
            .unwrap();
        assert_eq!(p1.name, "work");
        assert!(p1.profile_dir.exists());

        let list = pm.list_profiles();
        assert_eq!(list.len(), 2); // default + work

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
