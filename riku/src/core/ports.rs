use std::path::{Path, PathBuf};

use crate::core::driver::Renderer;
use crate::core::git_service::{
    BranchInfo, ChangedFile, CommitInfo, GitError, GitService, WorkingChange,
};
use crate::core::models::{FileFormat, Schematic};

pub trait GitRepository {
    fn get_blob(&self, commit_ish: &str, file_path: &str) -> Result<Vec<u8>, GitError>;

    fn get_commits(&self, file_path: Option<&str>) -> Result<Vec<CommitInfo>, GitError>;

    fn get_changed_files(
        &self,
        commit_a: &str,
        commit_b: &str,
    ) -> Result<Vec<ChangedFile>, GitError>;

    /// Cambios en working tree vs HEAD. Default `Ok(vec![])` para no romper
    /// implementaciones existentes (mocks de tests, futuros adaptadores).
    fn working_tree_changes(&self) -> Result<Vec<WorkingChange>, GitError> {
        Ok(Vec::new())
    }

    /// Información de la rama actual. Default `Ok(None)` para no forzar a
    /// cada adapter a implementarlo si no aplica (repo en estado inicial).
    fn current_branch(&self) -> Result<Option<BranchInfo>, GitError> {
        Ok(None)
    }
}

impl GitRepository for GitService {
    fn get_blob(&self, commit_ish: &str, file_path: &str) -> Result<Vec<u8>, GitError> {
        GitService::get_blob(self, commit_ish, file_path)
    }

    fn get_commits(&self, file_path: Option<&str>) -> Result<Vec<CommitInfo>, GitError> {
        GitService::get_commits(self, file_path)
    }

    fn get_changed_files(
        &self,
        commit_a: &str,
        commit_b: &str,
    ) -> Result<Vec<ChangedFile>, GitError> {
        GitService::get_changed_files(self, commit_a, commit_b)
    }

    fn working_tree_changes(&self) -> Result<Vec<WorkingChange>, GitError> {
        GitService::working_tree_changes(self)
    }

    fn current_branch(&self) -> Result<Option<BranchInfo>, GitError> {
        GitService::current_branch(self)
    }
}

pub trait SchematicParser {
    fn detect_format(&self, content: &[u8]) -> FileFormat;

    fn parse(&self, content: &[u8]) -> Schematic;
}

pub trait RendererPort: Renderer {
    fn render(&self, content: &[u8], path_hint: &str) -> Option<PathBuf>;
}

impl<T: Renderer + ?Sized> RendererPort for T {
    fn render(&self, content: &[u8], path_hint: &str) -> Option<PathBuf> {
        Renderer::render(self, content, path_hint)
    }
}

pub trait DriverContract: RendererPort {
    fn can_handle(&self, filename: &str) -> bool;
}

pub trait RepoRoot {
    fn root(&self) -> Option<&Path>;
}
