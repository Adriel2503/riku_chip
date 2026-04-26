use git2::{DiffOptions, Repository};

use crate::core::domain::git_types::{ChangeStatus, ChangedFile, GitError};
use crate::core::git::helpers::resolve_commit;

pub(super) fn get_changed_files(
    repo: &Repository,
    commit_a: &str,
    commit_b: &str,
) -> Result<Vec<ChangedFile>, GitError> {
    let tree_a = resolve_commit(repo, commit_a)?.tree()?;
    let tree_b = resolve_commit(repo, commit_b)?.tree()?;
    let mut options = DiffOptions::new();
    let mut diff =
        repo.diff_tree_to_tree(Some(&tree_a), Some(&tree_b), Some(&mut options))?;
    let mut find_options = git2::DiffFindOptions::new();
    diff.find_similar(Some(&mut find_options))?;

    let mut results = Vec::new();
    for delta in diff.deltas() {
        let status = match delta.status() {
            git2::Delta::Added => ChangeStatus::Added,
            git2::Delta::Deleted => ChangeStatus::Removed,
            git2::Delta::Modified => ChangeStatus::Modified,
            git2::Delta::Renamed => ChangeStatus::Renamed,
            _ => ChangeStatus::Modified,
        };
        let new_path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .ok_or_else(|| GitError::CommitNotFound("delta path missing".to_string()))?;
        results.push(ChangedFile {
            path: new_path.to_string_lossy().to_string(),
            status,
            old_path: delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string()),
        });
    }
    Ok(results)
}
