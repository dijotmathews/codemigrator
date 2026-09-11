use crate::git_repository::{ChangedFile, GitRepository};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CopyService;

impl CopyService {
    /// Copies `files` from `source_repo` (as they exist on `source_branch`) into
    /// `dest_repo`, preserving relative paths. Files that no longer exist on
    /// `source_branch` (deletions) are removed from `dest_repo` instead.
    ///
    /// Returns the `dest_repo`-relative paths that were actually written or
    /// removed (nothing is returned under `dry_run`, since nothing changed on
    /// disk).
    pub fn apply(
        source_repo: &Path,
        source_branch: &str,
        dest_repo: &Path,
        files: &[ChangedFile],
        dry_run: bool,
    ) -> Result<Vec<PathBuf>> {
        let repo = GitRepository::open(source_repo)?;
        let mut touched = Vec::new();

        for file in files {
            let path = &file.path;
            let dest_path = dest_repo.join(path);

            match repo.read_blob(source_branch, path)? {
                Some(data) => {
                    if dry_run {
                        println!("would copy {} -> {}", path.display(), dest_path.display());
                    } else {
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(&dest_path, data)?;
                        println!("copied {} -> {}", path.display(), dest_path.display());
                        touched.push(path.clone());
                    }
                }
                None if dest_path.exists() => {
                    if dry_run {
                        println!("would delete {}", dest_path.display());
                    } else {
                        fs::remove_file(&dest_path)?;
                        println!("deleted {}", dest_path.display());
                        touched.push(path.clone());
                    }
                }
                None => {}
            }
        }

        Ok(touched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_repository::ChangeStatus;
    use crate::test_util::TestRepo;

    fn changed(status: ChangeStatus, path: &str) -> ChangedFile {
        ChangedFile {
            status,
            path: PathBuf::from(path),
            previous_path: None,
        }
    }

    #[test]
    fn apply_copies_added_and_modified_files_and_deletes_removed_ones() {
        let source = TestRepo::init();
        source.write_file("a.txt", "base a");
        source.write_file("keep.txt", "keep");
        source.commit_all("base");

        source.checkout_new_branch("feature");
        source.write_file("a.txt", "changed a");
        source.write_file("b.txt", "new b");
        source.remove_file("keep.txt");
        source.commit_all("change");

        let dest = TestRepo::init();
        dest.write_file("keep.txt", "stale copy present in dest");
        dest.commit_all("dest base");

        let files = vec![
            changed(ChangeStatus::Modified, "a.txt"),
            changed(ChangeStatus::Added, "b.txt"),
            changed(ChangeStatus::Deleted, "keep.txt"),
        ];

        let mut touched =
            CopyService::apply(source.path(), "feature", dest.path(), &files, false).unwrap();
        touched.sort();

        assert_eq!(fs::read_to_string(dest.path().join("a.txt")).unwrap(), "changed a");
        assert_eq!(fs::read_to_string(dest.path().join("b.txt")).unwrap(), "new b");
        assert!(!dest.path().join("keep.txt").exists());

        assert_eq!(
            touched,
            vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("keep.txt"),
            ]
        );
    }

    #[test]
    fn apply_dry_run_makes_no_changes_and_returns_no_touched_paths() {
        let source = TestRepo::init();
        source.write_file("a.txt", "base a");
        source.commit_all("base");

        source.checkout_new_branch("feature");
        source.write_file("a.txt", "changed a");
        source.commit_all("change");

        let dest = TestRepo::init();
        dest.write_file("placeholder.txt", "unrelated");
        dest.commit_all("dest base");

        let files = vec![changed(ChangeStatus::Modified, "a.txt")];
        let touched =
            CopyService::apply(source.path(), "feature", dest.path(), &files, true).unwrap();

        assert!(touched.is_empty());
        assert!(!dest.path().join("a.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("placeholder.txt")).unwrap(),
            "unrelated"
        );
    }
}
