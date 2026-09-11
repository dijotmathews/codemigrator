use crate::git_repository::{ChangedFile, GitRepository};
use anyhow::Result;
use std::path::Path;

const INCLUDED_EXTENSIONS: &[&str] = &["java", "properties", "json"];

pub struct DiffService;

impl DiffService {
    pub fn changed_files(
        repo_path: &Path,
        branch1: &str,
        branch2: &str,
    ) -> Result<Vec<ChangedFile>> {
        println!(
            "Diffing repository {} between '{}' (target) and '{}' (source)",
            repo_path.display(),
            branch1,
            branch2
        );

        let repo = GitRepository::open(repo_path)?;
        let files = repo.changed_files(branch1, branch2)?;
        let total = files.len();
        println!("Found {total} changed file(s) between the two branches");

        let filtered: Vec<ChangedFile> = files
            .into_iter()
            .filter(|file| {
                file.path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| INCLUDED_EXTENSIONS.contains(&ext))
            })
            .collect();

        println!(
            "{} of {total} changed file(s) match included extensions ({}); {} excluded",
            filtered.len(),
            INCLUDED_EXTENSIONS.join(", "),
            total - filtered.len()
        );

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TestRepo;
    use std::path::PathBuf;

    #[test]
    fn changed_files_only_includes_files_with_configured_extensions() {
        let repo = TestRepo::init();
        repo.write_file("a.java", "base");
        repo.write_file("b.txt", "base");
        repo.commit_all("base");

        repo.checkout_new_branch("feature");
        repo.write_file("a.java", "changed");
        repo.write_file("b.txt", "changed");
        repo.write_file("c.json", "new");
        repo.commit_all("change");

        let files = DiffService::changed_files(repo.path(), "main", "feature").unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();

        assert!(paths.contains(&PathBuf::from("a.java")));
        assert!(paths.contains(&PathBuf::from("c.json")));
        assert!(!paths.contains(&PathBuf::from("b.txt")));
    }
}
