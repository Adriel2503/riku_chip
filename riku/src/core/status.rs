//! Orquestación de `riku status`.
//!
//! Composición de `GitRepository` (working tree + HEAD) con `RikuDriver` para
//! producir una lista de `FileSummary` clasificados.
//!
//! Este módulo no formatea — entrega `StatusReport` y la capa CLI decide cómo
//! presentarlo (texto, JSON, ...).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::git_service::{
    BranchInfo, ChangeStatus, GitError, GitService, WorkingChange,
};
use crate::core::ports::{GitRepository, RepoRoot};
use crate::core::registry::get_driver_for;
use crate::core::summary::{FileSummary, SummaryCategory};

// ─── Errores ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum StatusError {
    #[error(transparent)]
    Git(#[from] GitError),
}

// ─── Modelo de salida ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusReport {
    pub branch: Option<BranchInfo>,
    pub files: Vec<FileSummary>,
    /// Mensajes informativos no fatales (blob omitido por tamaño, etc.).
    pub warnings: Vec<String>,
}

impl StatusReport {
    pub fn has_semantic_changes(&self) -> bool {
        self.files
            .iter()
            .any(|f| matches!(f.category, SummaryCategory::Semantic))
    }

    pub fn count_by_category(&self, cat: SummaryCategory) -> usize {
        self.files
            .iter()
            .filter(|f| f.category == cat)
            .count()
    }
}

// ─── Entry points ────────────────────────────────────────────────────────────

/// Helper que abre el repo y delega en `analyze_with_repo`.
pub fn analyze(repo_path: &Path) -> Result<StatusReport, StatusError> {
    let svc = GitService::open(repo_path)?;
    let workdir = svc.root().map(|p| p.to_path_buf());
    analyze_with_repo(&svc, workdir.as_deref())
}

/// Versión inyectable: recibe un `GitRepository` y la raíz del working tree.
///
/// `workdir` es opcional: si es `None`, no se leen archivos de disco — solo
/// se reportan paths sin contenido (útil para tests sin filesystem).
pub fn analyze_with_repo<R: GitRepository + ?Sized>(
    repo: &R,
    workdir: Option<&Path>,
) -> Result<StatusReport, StatusError> {
    let branch = repo.current_branch()?;
    let changes = repo.working_tree_changes()?;

    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for change in changes {
        let summary = summarize_change(repo, workdir, &change, &mut warnings);
        files.push(summary);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(StatusReport { branch, files, warnings })
}

// ─── Resumen por archivo ─────────────────────────────────────────────────────

fn summarize_change<R: GitRepository + ?Sized>(
    repo: &R,
    workdir: Option<&Path>,
    change: &WorkingChange,
    warnings: &mut Vec<String>,
) -> FileSummary {
    let driver = match get_driver_for(&change.path) {
        Some(d) => d,
        None => return FileSummary::unknown(&change.path),
    };

    // Contenido "antes": HEAD si el archivo existía allí; vacío si nuevo.
    let content_before = match change.status {
        ChangeStatus::Added => Vec::new(),
        _ => match repo.get_blob("HEAD", change.path.as_str()) {
            Ok(bytes) => bytes,
            Err(GitError::BlobNotFound { .. }) => Vec::new(),
            Err(GitError::LargeBlob { path, size }) => {
                warnings.push(format!(
                    "{path} ({size} bytes) demasiado grande en HEAD; se asume vacío."
                ));
                Vec::new()
            }
            Err(e) => return FileSummary::error(&change.path, e.to_string()),
        },
    };

    // Contenido "después": working tree desde disco (a menos que el archivo
    // esté eliminado, en cuyo caso es vacío).
    let content_after = match change.status {
        ChangeStatus::Removed => Vec::new(),
        _ => match read_workdir(workdir, &change.path) {
            Ok(bytes) => bytes,
            Err(msg) => {
                warnings.push(format!("{}: {msg}", change.path));
                Vec::new()
            }
        },
    };

    let report = driver.diff(&content_before, &content_after, &change.path);
    FileSummary::from_report(&report, &change.path)
}

fn read_workdir(workdir: Option<&Path>, rel_path: &str) -> Result<Vec<u8>, String> {
    let base = match workdir {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let full: PathBuf = base.join(rel_path);
    std::fs::read(&full).map_err(|e| format!("no se pudo leer {}: {e}", full.display()))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::git_service::{ChangedFile, CommitInfo};

    /// Repo mock que solo provee working_tree_changes y get_blob — suficiente
    /// para ejercitar `analyze_with_repo` sin tocar disco real.
    struct MockRepo {
        changes: Vec<WorkingChange>,
        head_blobs: std::collections::HashMap<String, Vec<u8>>,
        branch: Option<BranchInfo>,
    }

    impl GitRepository for MockRepo {
        fn get_blob(&self, _commit_ish: &str, file_path: &str) -> Result<Vec<u8>, GitError> {
            self.head_blobs
                .get(file_path)
                .cloned()
                .ok_or_else(|| GitError::BlobNotFound {
                    commit: "HEAD".to_string(),
                    path: file_path.to_string(),
                })
        }
        fn get_commits(&self, _file_path: Option<&str>) -> Result<Vec<CommitInfo>, GitError> {
            Ok(Vec::new())
        }
        fn get_changed_files(&self, _: &str, _: &str) -> Result<Vec<ChangedFile>, GitError> {
            Ok(Vec::new())
        }
        fn working_tree_changes(&self) -> Result<Vec<WorkingChange>, GitError> {
            Ok(self.changes.clone())
        }
        fn current_branch(&self) -> Result<Option<BranchInfo>, GitError> {
            Ok(self.branch.clone())
        }
    }

    #[test]
    fn archivo_sin_driver_se_marca_unknown() {
        let repo = MockRepo {
            changes: vec![WorkingChange {
                path: "Makefile".to_string(),
                status: ChangeStatus::Modified,
                old_path: None,
            }],
            head_blobs: Default::default(),
            branch: None,
        };
        let report = analyze_with_repo(&repo, None).unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].category, SummaryCategory::Unknown);
        assert!(!report.has_semantic_changes());
    }

    #[test]
    fn lista_se_ordena_por_path() {
        let repo = MockRepo {
            changes: vec![
                WorkingChange { path: "z.txt".into(), status: ChangeStatus::Modified, old_path: None },
                WorkingChange { path: "a.txt".into(), status: ChangeStatus::Modified, old_path: None },
            ],
            head_blobs: Default::default(),
            branch: None,
        };
        let report = analyze_with_repo(&repo, None).unwrap();
        assert_eq!(report.files[0].path, "a.txt");
        assert_eq!(report.files[1].path, "z.txt");
    }

    #[test]
    fn rama_se_propaga_al_reporte() {
        let repo = MockRepo {
            changes: vec![],
            head_blobs: Default::default(),
            branch: Some(BranchInfo {
                name: "feature-amp".into(),
                head_oid: "0".repeat(40),
                head_short: "0000000".into(),
                upstream: None,
                ahead: 0,
                behind: 0,
            }),
        };
        let report = analyze_with_repo(&repo, None).unwrap();
        assert_eq!(report.branch.as_ref().map(|b| b.name.as_str()), Some("feature-amp"));
    }
}
