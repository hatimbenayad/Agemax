//! Workspace directory containment and safe startup path resolution (`REQ-080`, `REQ-081`).

use crate::schema::GeneralConfig;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PathSafetyError {
    #[error("Workspace path cannot be empty")]
    EmptyPath,
    #[error("Directory '{0}' is outside your allowed safe folder '{1}'", display_clean_path(.requested), display_clean_path(.allowed_root))]
    OutsideAllowedRoot {
        requested: PathBuf,
        allowed_root: PathBuf,
    },
}

/// Strip Windows extended-length `\\?\` prefix for readable display and comparison.
pub fn display_clean_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

fn strip_extended_prefix(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(stripped) = raw.strip_prefix(r"\\?\") {
        // Keep UNC server paths like \\?\UNC\server\share unchanged, strip drive-letter \\?\C:\...
        if !stripped.starts_with(r"UNC\") {
            return PathBuf::from(stripped);
        }
    }
    path
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// Canonicalize `path` (or its deepest existing ancestor + remaining segments) safely.
pub fn resolve_normalized_path(path: &Path) -> PathBuf {
    if let Ok(canon) = std::fs::canonicalize(path) {
        return strip_extended_prefix(canon);
    }

    let lexical = if path.is_absolute() {
        normalize_lexical(path)
    } else if let Ok(cwd) = std::env::current_dir() {
        normalize_lexical(&cwd.join(path))
    } else {
        normalize_lexical(path)
    };

    let mut ancestor = lexical.as_path();
    let mut tail = Vec::new();
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            break;
        };
        tail.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            break;
        };
        ancestor = parent;
    }

    let mut base = strip_extended_prefix(
        std::fs::canonicalize(ancestor).unwrap_or_else(|_| ancestor.to_path_buf()),
    );
    for seg in tail.into_iter().rev() {
        base.push(seg);
    }
    base
}

fn path_starts_with_case_aware(candidate: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let c_comps: Vec<String> = candidate
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();
        let r_comps: Vec<String> = root
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();
        c_comps.len() >= r_comps.len() && c_comps[..r_comps.len()] == r_comps[..]
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(root)
    }
}

/// Verify that `candidate` stays inside `allowed_root` (if configured).
/// Returns the resolved canonical/normalized path on success.
pub fn validate_workspace_path(
    candidate: &Path,
    allowed_root: Option<&Path>,
) -> Result<PathBuf, PathSafetyError> {
    if candidate.as_os_str().is_empty() || candidate.to_string_lossy().trim().is_empty() {
        return Err(PathSafetyError::EmptyPath);
    }

    let resolved_candidate = resolve_normalized_path(candidate);

    if let Some(root) = allowed_root.filter(|p| !p.as_os_str().is_empty()) {
        let resolved_root = resolve_normalized_path(root);
        if !path_starts_with_case_aware(&resolved_candidate, &resolved_root) {
            return Err(PathSafetyError::OutsideAllowedRoot {
                requested: candidate.to_path_buf(),
                allowed_root: root.to_path_buf(),
            });
        }
    }

    Ok(resolved_candidate)
}

/// Choose the safest initial working directory at app startup (`REQ-081`).
pub fn resolve_startup_directory(
    explicit_candidate: Option<&Path>,
    general: &GeneralConfig,
) -> PathBuf {
    let allowed = general.allowed_root_dir.as_deref();

    if let Some(cand) = explicit_candidate.filter(|p| p.is_dir()) {
        if let Ok(resolved) = validate_workspace_path(cand, allowed) {
            return resolved;
        }
    }

    if let Some(default_dir) = general
        .default_workspace_dir
        .as_deref()
        .filter(|p| p.is_dir())
    {
        if let Ok(resolved) = validate_workspace_path(default_dir, allowed) {
            return resolved;
        }
    }

    if let Some(root) = allowed.filter(|p| p.is_dir()) {
        return resolve_normalized_path(root);
    }

    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(resolved) = validate_workspace_path(&cwd, allowed) {
            return resolved;
        }
    }

    allowed
        .map(resolve_normalized_path)
        .unwrap_or_else(|| PathBuf::from("."))
}
