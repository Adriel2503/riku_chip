use std::collections::HashMap;
use std::path::Path;

use crate::core::domain::driver::Renderer;
use crate::core::domain::git_types::{
    BranchInfo, ChangedFile, CommitInfo, CommitWithParents, GitError, LogQuery, WorkingChange,
};
use crate::core::domain::models::{FileFormat, Schematic};

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

    /// Versión enriquecida de `get_commits` con filtros y padres por commit.
    /// Default delega a `get_commits` y sintetiza padres vacíos para no romper
    /// adapters existentes.
    fn get_commits_with_options(
        &self,
        query: &LogQuery<'_>,
    ) -> Result<Vec<CommitWithParents>, GitError> {
        let mut commits = self.get_commits(query.file_path)?;
        if let Some(limit) = query.limit {
            commits.truncate(limit);
        }
        Ok(commits
            .into_iter()
            .map(|info| CommitWithParents { info, parents: Vec::new() })
            .collect())
    }

    /// Mapa `oid → [refs]` para anotar el log. Default vacío.
    fn refs_by_oid(&self) -> Result<HashMap<String, Vec<String>>, GitError> {
        Ok(HashMap::new())
    }
}

pub trait SchematicParser {
    fn detect_format(&self, content: &[u8]) -> FileFormat;

    fn parse(&self, content: &[u8]) -> Schematic;
}

pub trait RendererPort: Renderer {
    fn render(&self, content: &[u8], path_hint: &str) -> Option<String>;
}

impl<T: Renderer + ?Sized> RendererPort for T {
    fn render(&self, content: &[u8], path_hint: &str) -> Option<String> {
        Renderer::render(self, content, path_hint)
    }
}

pub trait DriverContract: RendererPort {
    fn can_handle(&self, filename: &str) -> bool;
}

pub trait RepoRoot {
    fn root(&self) -> Option<&Path>;
}
