use std::fs;
use std::path::PathBuf;
use std::process::Command;

use dirs::cache_dir;
use sha2::{Digest, Sha256};
use serde::Serialize;

use crate::core::driver::{DiffEntry, DriverDiffReport, DriverInfo, RikuDriver};
use crate::core::models::{ChangeKind, DriverKind, FileFormat};
use crate::core::semantic_diff::diff as semantic_diff;
use crate::parsers::xschem::detect_format;

pub struct XschemDriver {
    cached_info: std::sync::OnceLock<DriverInfo>,
}

#[derive(Debug, Serialize)]
struct RenderManifest<'a> {
    driver: &'a str,
    version: &'a str,
    source_sha256: &'a str,
}

impl XschemDriver {
    pub fn new() -> Self {
        Self {
            cached_info: std::sync::OnceLock::new(),
        }
    }

    fn cache_dir() -> PathBuf {
        cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("riku")
            .join("ops")
    }

    fn find_xschem() -> Option<PathBuf> {
        which::which("xschem").ok()
    }

    fn render_key(version: &str, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(version.as_bytes());
        hasher.update(b"::");
        hasher.update(content);
        let digest = hasher.finalize();
        digest.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn render_paths(version: &str, content: &[u8]) -> (PathBuf, PathBuf, String) {
        let key = Self::render_key(version, content);
        let root = Self::cache_dir().join(&key);
        (root.join("render.svg"), root.join("origins.txt"), key)
    }
}

impl Default for XschemDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl RikuDriver for XschemDriver {
    fn info(&self) -> DriverInfo {
        if let Some(info) = self.cached_info.get() {
            return info.clone();
        }

        let info = match Self::find_xschem() {
            None => DriverInfo {
                name: DriverKind::Xschem,
                available: false,
                version: String::new(),
                extensions: vec![".sch".to_string()],
            },
            Some(xschem) => match Command::new(xschem).arg("--version").output() {
                Ok(output) => {
                    let combined = [output.stdout, output.stderr].concat();
                    let text = String::from_utf8_lossy(&combined);
                    let version = text
                        .lines()
                        .find(|line| line.contains("XSCHEM V"))
                        .map(|line| line.trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    DriverInfo {
                        name: DriverKind::Xschem,
                        available: true,
                        version,
                        extensions: vec![".sch".to_string()],
                    }
                }
                Err(_) => DriverInfo {
                    name: DriverKind::Xschem,
                    available: false,
                    version: String::new(),
                    extensions: vec![".sch".to_string()],
                },
            },
        };

        let _ = self.cached_info.set(info.clone());
        info
    }

    fn diff(&self, content_a: &[u8], content_b: &[u8], path_hint: &str) -> DriverDiffReport {
        let mut report = DriverDiffReport {
            file_type: FileFormat::Xschem,
            ..Default::default()
        };

        if detect_format(content_a) != FileFormat::Xschem {
            report
                .warnings
                .push(format!("{path_hint}: no es formato Xschem, usando diff de texto."));
            return report;
        }

        let result = semantic_diff(content_a, content_b);
        for component in result.components {
            report.changes.push(DiffEntry {
                kind: component.kind,
                element: component.name,
                before: component.before,
                after: component.after,
                cosmetic: component.cosmetic,
            });
        }

        for net in result.nets_added {
            report.changes.push(DiffEntry {
                kind: ChangeKind::Added,
                element: format!("net:{net}"),
                before: None,
                after: None,
                cosmetic: false,
            });
        }

        for net in result.nets_removed {
            report.changes.push(DiffEntry {
                kind: ChangeKind::Removed,
                element: format!("net:{net}"),
                before: None,
                after: None,
                cosmetic: false,
            });
        }

        if result.is_move_all {
            report.changes.push(DiffEntry {
                kind: ChangeKind::Modified,
                element: "layout".to_string(),
                before: None,
                after: Some(
                    [(
                        "note".to_string(),
                        "reorganizacion cosmetica (Move All)".to_string(),
                    )]
                    .into_iter()
                    .collect(),
                ),
                cosmetic: true,
            });
        }

        report
    }

    fn normalize(&self, content: &[u8], _path_hint: &str) -> Vec<u8> {
        content.to_vec()
    }

    fn render(&self, content: &[u8], _path_hint: &str) -> Option<PathBuf> {
        let xschem = Self::find_xschem()?;
        let info = self.info();
        if !info.available {
            return None;
        }

        let (cached, origins_path, key) = Self::render_paths(&info.version, content);
        if cached.exists() {
            return Some(cached);
        }

        fs::create_dir_all(cached.parent()?).ok()?;
        let manifest_path = cached.parent()?.join("render.json");

        let mut tmp = tempfile::NamedTempFile::new().ok()?;
        std::io::Write::write_all(&mut tmp, content).ok()?;
        let tmp_path = tmp.into_temp_path();

        let tcl_command = format!(
            "xschem zoom_full; set _f [open $env(RIKU_ORIGINS_PATH) w]; puts $_f [xschem get xorigin]; puts $_f [xschem get yorigin]; close $_f; xschem print svg {}",
            cached.display()
        );

        // --tcl recibe codigo TCL inline (no un archivo); sin shell=True no hay
        // interpretacion de $ por bash, asi que RIKU_ORIGINS_PATH llega intacto a TCL.
        let status = Command::new(&xschem)
            .arg("--tcl")
            .arg("wm iconify .")
            .arg("--command")
            .arg(&tcl_command)
            .arg("--quit")
            .arg(tmp_path.as_os_str())
            .env("RIKU_ORIGINS_PATH", &origins_path)
            .status()
            .ok()?;

        if status.success() && cached.exists() {
            let manifest = RenderManifest {
                driver: "xschem",
                version: &info.version,
                source_sha256: &key,
            };
            if let Ok(json) = serde_json::to_string_pretty(&manifest) {
                let _ = fs::write(manifest_path, json);
            }
            Some(cached)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::XschemDriver;

    #[test]
    fn render_key_changes_with_version_and_content() {
        let a = XschemDriver::render_key("XSCHEM V1", b"foo");
        let b = XschemDriver::render_key("XSCHEM V1", b"bar");
        let c = XschemDriver::render_key("XSCHEM V2", b"foo");

        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn render_paths_are_stable() {
        let (svg_a, origins_a, key_a) = XschemDriver::render_paths("XSCHEM V1", b"foo");
        let (svg_b, origins_b, key_b) = XschemDriver::render_paths("XSCHEM V1", b"foo");

        assert_eq!(svg_a, svg_b);
        assert_eq!(origins_a, origins_b);
        assert_eq!(key_a, key_b);
        assert!(svg_a.ends_with("render.svg"));
        assert!(origins_a.ends_with("origins.txt"));
    }
}
