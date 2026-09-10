use crate::git_repository::GitRepository;
use anyhow::Result;
use std::path::{Path, PathBuf};

const INCLUDED_EXTENSIONS: &[&str] = &["java", "properties", "json"];

pub struct DiffService;

impl DiffService {
    pub fn changed_files(
        repo_path: &Path,
        branch1: &str,
        branch2: &str,
    ) -> Result<Vec<PathBuf>> {
        println!(
            "diffing repo {} between branches {} and {}",
            repo_path.display(),
            branch1,
            branch2
        );

        let repo = GitRepository::open(repo_path)?;
        let files = repo.changed_files(branch1, branch2)?;

        Ok(files
            .into_iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| INCLUDED_EXTENSIONS.contains(&ext))
            })
            .collect())
    }
}
