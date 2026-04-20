use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dirs::cache_dir;
use sha2::{Digest, Sha256};

fn cache_root() -> PathBuf {
    cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("riku")
        .join("ops")
}

pub fn available() -> (bool, String) {
    let Some(xschem) = which::which("xschem").ok() else {
        return (false, String::new());
    };

    match Command::new(xschem).arg("--version").output() {
        Ok(output) => {
            let combined = [output.stdout, output.stderr].concat();
            let text = String::from_utf8_lossy(&combined);
            let version = text
                .lines()
                .find(|line| line.contains("XSCHEM V"))
                .map(|line| line.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            (true, version)
        }
        Err(_) => (false, String::new()),
    }
}

fn cache_key(content: &[u8], version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(version.as_bytes());
    hasher.update(b"::");
    hasher.update(content);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn render_svg(sch_path: &Path) -> Option<PathBuf> {
    let xschem = which::which("xschem").ok()?;
    let (ok, version) = available();
    if !ok {
        return None;
    }

    let content = fs::read(sch_path).ok()?;
    let key = cache_key(&content, &version);
    let cached = cache_root().join(key).join("render.svg");
    if cached.exists() {
        return Some(cached);
    }

    fs::create_dir_all(cached.parent()?).ok()?;
    let tmp_dir = cached.parent()?.to_path_buf();
    let origins_path = tmp_dir.join("origins.txt");
    let mut tmp = tempfile::NamedTempFile::new().ok()?;
    std::io::Write::write_all(&mut tmp, &content).ok()?;
    let tmp_path = tmp.into_temp_path();

    let command = format!(
        "xschem zoom_full; set _f [open $env(RIKU_ORIGINS_PATH) w]; puts $_f [xschem get xorigin]; puts $_f [xschem get yorigin]; close $_f; xschem print svg {}",
        cached.display()
    );

    let status = Command::new(xschem)
        .arg("--tcl")
        .arg("wm iconify .")
        .arg("--command")
        .arg(command)
        .arg("--quit")
        .arg(tmp_path.as_os_str())
        .env("RIKU_ORIGINS_PATH", &origins_path)
        .status()
        .ok()?;

    if status.success() && cached.exists() {
        Some(cached)
    } else {
        None
    }
}

pub fn diff_visual(sch_a: &Path, sch_b: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    (render_svg(sch_a), render_svg(sch_b))
}
