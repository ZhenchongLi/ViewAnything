use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_docmod() -> Option<PathBuf> {
    // Same directory as this executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("docmod");
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    // ~/.local/bin/docmod
    if let Some(home) = dirs_home() {
        let p = home.join(".local/bin/docmod");
        if is_executable(&p) {
            return Some(p);
        }
        let p = home.join(".docmod/bin/docmod");
        if is_executable(&p) {
            return Some(p);
        }
        let p = home.join(".dotnet/tools/docmod");
        if is_executable(&p) {
            return Some(p);
        }
    }

    // $DOCMOD_PATH
    if let Ok(env_path) = std::env::var("DOCMOD_PATH") {
        let p = PathBuf::from(env_path);
        if is_executable(&p) {
            return Some(p);
        }
    }

    // PATH via `which`
    if let Ok(out) = Command::new("which").arg("docmod").output() {
        if out.status.success() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    let p = PathBuf::from(trimmed);
                    if is_executable(&p) {
                        return Some(p);
                    }
                }
            }
        }
    }

    for p in ["/usr/local/bin/docmod", "/usr/bin/docmod"] {
        let path = PathBuf::from(p);
        if is_executable(&path) {
            return Some(path);
        }
    }

    None
}

pub fn render(path: &Path) -> Result<String, String> {
    let docmod = find_docmod().ok_or_else(|| {
        "docmod CLI not found. Install to ~/.local/bin/docmod or add to PATH.".to_string()
    })?;

    let output = Command::new(&docmod)
        .arg("render")
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to run docmod: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
        return Err(format!(
            "docmod render failed (exit {code}) using {}\nstderr: {err}\nstdout: {}",
            docmod.display(),
            &out.chars().take(500).collect::<String>()
        ));
    }

    String::from_utf8(output.stdout).map_err(|_| "docmod output is not UTF-8".to_string())
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0)
        .unwrap_or(false)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
