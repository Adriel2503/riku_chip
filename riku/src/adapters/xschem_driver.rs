use std::fs;
use std::path::PathBuf;

use dirs::cache_dir;
use sha2::{Digest, Sha256};

use crate::core::driver::{DiffEntry, DriverDiffReport, DriverInfo, RikuDriver};
use crate::core::models::{ChangeKind, DriverKind, FileFormat};
use crate::core::semantic_diff::diff as semantic_diff;
use crate::parsers::xschem::detect_format;

pub struct XschemDriver {
    cached_info: std::sync::OnceLock<DriverInfo>,
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

        // Rendering is now native — no xschem binary needed.
        let info = DriverInfo {
            name: DriverKind::Xschem,
            available: true,
            version: String::new(),
            extensions: vec![".sch".to_string()],
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
        let text = std::str::from_utf8(content).ok()?;

        let key = {
            let digest = Sha256::digest(content);
            digest.iter().map(|b| format!("{:02x}", b)).collect::<String>()
        };

        let cached = Self::cache_dir().join(&key).join("render.svg");
        if cached.exists() {
            return Some(cached);
        }

        let opts = xschem_viewer::RenderOptions::dark()
            .with_sym_paths_from_xschemrc();

        let result = xschem_viewer::Renderer::new(opts).render(text).ok()?;

        fs::create_dir_all(cached.parent()?).ok()?;
        fs::write(&cached, &result.svg).ok()?;

        Some(cached)
    }
}
