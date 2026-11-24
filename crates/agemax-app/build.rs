use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let ico_path = manifest_dir
        .join("..")
        .join("..")
        .join("assets")
        .join("agemax.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" || !ico_path.exists() {
        return;
    }

    let Some(rc_exe) = find_rc_exe() else {
        println!("cargo:warning=Windows SDK rc.exe not found; skipping .ico/.res embedding");
        return;
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string()));
    let rc_file = out_dir.join("agemax.rc");
    let res_file = out_dir.join("agemax.res");

    let canonical_ico = std::fs::canonicalize(&ico_path).unwrap_or(ico_path);
    // Strip Windows extended-length prefix \\?\ before embedding into .rc string literal.
    let ico_str = canonical_ico
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");

    let rc_content = format!(
        r#"1 ICON "{ico_str}"

1 VERSIONINFO
FILEVERSION 0,1,0,0
PRODUCTVERSION 0,1,0,0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "Age Max Contributors\0"
            VALUE "FileDescription", "Age Max - Native Agent Multiplexer\0"
            VALUE "FileVersion", "0.1.0\0"
            VALUE "InternalName", "agemax-app.exe\0"
            VALUE "LegalCopyright", "Dual-licensed MIT OR Apache-2.0\0"
            VALUE "OriginalFilename", "agemax-app.exe\0"
            VALUE "ProductName", "Age Max\0"
            VALUE "ProductVersion", "0.1.0\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    if std::fs::write(&rc_file, rc_content).is_err() {
        return;
    }

    let status = Command::new(rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_file)
        .arg(&rc_file)
        .status();

    match status {
        Ok(s) if s.success() && res_file.exists() => {
            println!("cargo:rustc-link-arg-bin=agemax-app={}", res_file.display());
        }
        Ok(s) => {
            println!("cargo:warning=rc.exe exited with status {s}; icon embedding skipped");
        }
        Err(e) => {
            println!("cargo:warning=Failed to invoke rc.exe ({e}); icon embedding skipped");
        }
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("rc.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    let arch_dir = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };

    for env_key in ["ProgramFiles(x86)", "ProgramFiles"] {
        let Ok(base) = env::var(env_key) else {
            continue;
        };
        let kits_bin = Path::new(&base).join("Windows Kits").join("10").join("bin");
        if let Ok(entries) = std::fs::read_dir(&kits_bin) {
            let mut versions: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|ent| ent.path()))
                .filter(|p| p.is_dir())
                .collect();
            versions.sort();
            versions.reverse();
            for ver_dir in versions {
                let candidate = ver_dir.join(arch_dir).join("rc.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}
