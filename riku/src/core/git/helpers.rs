use git2::{Commit, Oid, Repository};

use crate::core::domain::git_types::{CommitInfo, GitError};

pub(super) fn short_oid(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

pub(super) fn commit_info_from(commit: &Commit<'_>) -> CommitInfo {
    CommitInfo {
        oid: commit.id().to_string(),
        short_id: short_oid(commit.id()),
        message: commit.message().unwrap_or("").trim().to_string(),
        author: commit.author().name().unwrap_or("").to_string(),
        timestamp: commit.author().when().seconds(),
    }
}

pub(super) fn resolve_commit<'r>(
    repo: &'r Repository,
    commit_ish: &str,
) -> Result<Commit<'r>, GitError> {
    let obj = repo.revparse_single(commit_ish)?;
    let commit = obj.peel_to_commit()?;
    Ok(commit)
}
