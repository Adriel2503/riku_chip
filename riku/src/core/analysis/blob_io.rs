//! Helpers para leer blobs de Git tolerando errores no fatales.
//!
//! Centraliza la política `BlobNotFound | LargeBlob → vacío + warning`
//! que se repetía en analyzer/diff_view/log.

use crate::core::domain::git_types::GitError;
use crate::core::domain::ports::GitRepository;

/// Lee un blob tolerando errores no-fatales:
/// - `BlobNotFound`: devuelve `Ok(None)` silenciosamente.
/// - `LargeBlob`: añade warning y devuelve `Ok(None)`.
/// - Cualquier otro error: se propaga.
///
/// Usar cuando el caller distingue "no había blob" de "lo había y lo procesamos".
pub fn read_blob_lenient<R: GitRepository + ?Sized>(
    repo: &R,
    commit: &str,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<Vec<u8>>, GitError> {
    match repo.get_blob(commit, path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(GitError::BlobNotFound { .. }) => Ok(None),
        Err(GitError::LargeBlob { path, size }) => {
            warnings.push(format!(
                "{path} ({size} bytes) demasiado grande; omitiendo."
            ));
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Lee un blob; cualquier error se convierte en warning + `Vec::new()`.
/// Para flujos donde fallar el resultado entero por un blob suelto no aporta valor.
pub fn read_blob_silent<R: GitRepository + ?Sized>(
    repo: &R,
    commit: &str,
    path: &str,
    warnings: &mut Vec<String>,
) -> Vec<u8> {
    match repo.get_blob(commit, path) {
        Ok(bytes) => bytes,
        Err(GitError::BlobNotFound { .. }) => Vec::new(),
        Err(e) => {
            warnings.push(format!("{path} en {commit}: {e}"));
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::git_types::{ChangedFile, CommitInfo};
    use std::collections::HashMap;

    /// Mock con respuestas configurables por (commit, path).
    struct MockRepo {
        responses: HashMap<(String, String), Result<Vec<u8>, GitError>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }
        fn set(&mut self, commit: &str, path: &str, result: Result<Vec<u8>, GitError>) {
            self.responses
                .insert((commit.to_string(), path.to_string()), result);
        }
    }

    impl GitRepository for MockRepo {
        fn get_blob(&self, commit: &str, path: &str) -> Result<Vec<u8>, GitError> {
            match self.responses.get(&(commit.to_string(), path.to_string())) {
                Some(Ok(bytes)) => Ok(bytes.clone()),
                Some(Err(GitError::BlobNotFound { commit, path })) => Err(GitError::BlobNotFound {
                    commit: commit.clone(),
                    path: path.clone(),
                }),
                Some(Err(GitError::LargeBlob { path, size })) => Err(GitError::LargeBlob {
                    path: path.clone(),
                    size: *size,
                }),
                Some(Err(GitError::CommitNotFound(s))) => Err(GitError::CommitNotFound(s.clone())),
                Some(Err(_)) | None => Err(GitError::CommitNotFound("mock-default".to_string())),
            }
        }
        fn get_commits(&self, _: Option<&str>) -> Result<Vec<CommitInfo>, GitError> {
            Ok(Vec::new())
        }
        fn get_changed_files(&self, _: &str, _: &str) -> Result<Vec<ChangedFile>, GitError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn lenient_ok_devuelve_some_sin_warning() {
        let mut repo = MockRepo::new();
        repo.set("HEAD", "a.sch", Ok(b"hello".to_vec()));
        let mut warnings = Vec::new();
        let out = read_blob_lenient(&repo, "HEAD", "a.sch", &mut warnings).unwrap();
        assert_eq!(out.as_deref(), Some(&b"hello"[..]));
        assert!(warnings.is_empty());
    }

    #[test]
    fn lenient_blob_not_found_devuelve_none_sin_warning() {
        let mut repo = MockRepo::new();
        repo.set(
            "HEAD",
            "a.sch",
            Err(GitError::BlobNotFound {
                commit: "HEAD".into(),
                path: "a.sch".into(),
            }),
        );
        let mut warnings = Vec::new();
        let out = read_blob_lenient(&repo, "HEAD", "a.sch", &mut warnings).unwrap();
        assert!(out.is_none());
        assert!(warnings.is_empty());
    }

    #[test]
    fn lenient_large_blob_devuelve_none_con_warning() {
        let mut repo = MockRepo::new();
        repo.set(
            "HEAD",
            "big.gds",
            Err(GitError::LargeBlob {
                path: "big.gds".into(),
                size: 99_000_000,
            }),
        );
        let mut warnings = Vec::new();
        let out = read_blob_lenient(&repo, "HEAD", "big.gds", &mut warnings).unwrap();
        assert!(out.is_none());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("99000000"));
        assert!(warnings[0].contains("big.gds"));
    }

    #[test]
    fn lenient_otro_error_se_propaga() {
        let mut repo = MockRepo::new();
        repo.set(
            "HEAD",
            "x.sch",
            Err(GitError::CommitNotFound("HEAD".into())),
        );
        let mut warnings = Vec::new();
        let result = read_blob_lenient(&repo, "HEAD", "x.sch", &mut warnings);
        assert!(matches!(result, Err(GitError::CommitNotFound(_))));
        assert!(warnings.is_empty());
    }

    #[test]
    fn silent_ok_devuelve_bytes_sin_warning() {
        let mut repo = MockRepo::new();
        repo.set("HEAD", "a.sch", Ok(b"data".to_vec()));
        let mut warnings = Vec::new();
        let out = read_blob_silent(&repo, "HEAD", "a.sch", &mut warnings);
        assert_eq!(out, b"data".to_vec());
        assert!(warnings.is_empty());
    }

    #[test]
    fn silent_blob_not_found_devuelve_vacio_sin_warning() {
        let mut repo = MockRepo::new();
        repo.set(
            "HEAD",
            "a.sch",
            Err(GitError::BlobNotFound {
                commit: "HEAD".into(),
                path: "a.sch".into(),
            }),
        );
        let mut warnings = Vec::new();
        let out = read_blob_silent(&repo, "HEAD", "a.sch", &mut warnings);
        assert!(out.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn silent_otro_error_devuelve_vacio_con_warning() {
        let mut repo = MockRepo::new();
        repo.set(
            "HEAD",
            "a.sch",
            Err(GitError::CommitNotFound("HEAD".into())),
        );
        let mut warnings = Vec::new();
        let out = read_blob_silent(&repo, "HEAD", "a.sch", &mut warnings);
        assert!(out.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("a.sch"));
        assert!(warnings[0].contains("HEAD"));
    }
}
