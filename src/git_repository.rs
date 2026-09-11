use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use gix::object::tree::diff::ChangeDetached;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitRepository {
    repo: gix::Repository,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub status: ChangeStatus,
    pub path: PathBuf,
    pub previous_path: Option<PathBuf>,
}

impl std::fmt::Display for ChangedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.status {
            ChangeStatus::Added => "new file:",
            ChangeStatus::Modified => "modified:",
            ChangeStatus::Deleted => "deleted:",
            ChangeStatus::Renamed => "renamed:",
            ChangeStatus::Copied => "copied:",
        };
        write!(f, "{label:<12}")?;
        match &self.previous_path {
            Some(previous_path) => write!(f, "{} -> {}", previous_path.display(), self.path.display()),
            None => write!(f, "{}", self.path.display()),
        }
    }
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = gix::open(path)?;
        Ok(Self { repo })
    }

    /// Fast-forwards the local `branch` ref to match `origin/branch`, whether or
    /// not `branch` is currently checked out.
    pub fn pull_branch(repo_path: &Path, branch: &str) -> Result<()> {
        // Accept either "dev" or "origin/dev" - the remote only knows the branch
        // by its plain name, so a caller-supplied remote-tracking prefix would
        // otherwise turn into an invalid remote ref.
        let branch = branch.strip_prefix("origin/").unwrap_or(branch);

        let status = if Self::current_branch(repo_path)?.as_deref() == Some(branch) {
            // `git fetch origin branch:branch` refuses to touch a checked-out
            // branch, so update it (and the working tree) via a fast-forward pull.
            Command::new("git")
                .arg("-C")
                .arg(repo_path)
                .arg("pull")
                .arg("--ff-only")
                .arg("origin")
                .arg(branch)
                .status()
                .with_context(|| format!("failed to run git pull --ff-only origin {branch}"))?
        } else {
            let refspec = format!("{branch}:{branch}");
            Command::new("git")
                .arg("-C")
                .arg(repo_path)
                .arg("fetch")
                .arg("origin")
                .arg(&refspec)
                .status()
                .with_context(|| format!("failed to run git fetch origin {refspec}"))?
        };

        if !status.success() {
            anyhow::bail!(
                "failed to update branch {branch} in {} from origin",
                repo_path.display()
            );
        }

        Ok(())
    }

    /// Returns the name of the currently checked-out branch, or `None` if `HEAD`
    /// is detached.
    fn current_branch(repo_path: &Path) -> Result<Option<String>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("symbolic-ref")
            .arg("--quiet")
            .arg("--short")
            .arg("HEAD")
            .output()
            .with_context(|| format!("failed to determine current branch in {}", repo_path.display()))?;

        if !output.status.success() {
            return Ok(None);
        }

        Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
    }

    /// Creates and checks out a new `branch` in the repository at `repo_path`,
    /// starting from its current `HEAD`.
    pub fn create_branch(repo_path: &Path, branch: &str) -> Result<()> {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("checkout")
            .arg("-b")
            .arg(branch)
            .status()
            .with_context(|| format!("failed to create branch {branch} in {}", repo_path.display()))?;

        if !status.success() {
            anyhow::bail!(
                "git checkout -b {branch} failed in {}",
                repo_path.display()
            );
        }

        Ok(())
    }

    /// Stages `paths` (relative to `repo_path`) and commits them with `message`.
    pub fn commit_changes(repo_path: &Path, paths: &[PathBuf], message: &str) -> Result<()> {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("add")
            .arg("--")
            .args(paths)
            .status()
            .with_context(|| format!("failed to stage changes in {}", repo_path.display()))?;

        if !status.success() {
            anyhow::bail!("git add failed in {}", repo_path.display());
        }

        let status = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .status()
            .with_context(|| format!("failed to commit changes in {}", repo_path.display()))?;

        if !status.success() {
            anyhow::bail!("git commit failed in {}", repo_path.display());
        }

        Ok(())
    }

    pub fn changed_files(
        &self,
        target_branch: &str,
        source_branch: &str,
    ) -> Result<Vec<ChangedFile>> {
        let target_id = self.repo.rev_parse_single(target_branch.as_bytes())?;
        let source_id = self.repo.rev_parse_single(source_branch.as_bytes())?;

        // Three-dot semantics (`git diff --name-only target...source`): diff from the
        // merge-base of the two branches to the source tip, so changes made on `target`
        // after the branches diverged aren't included.
        let merge_base_id = self.repo.merge_base(target_id, source_id)?;

        let base_tree = self.repo.find_commit(merge_base_id)?.tree()?;
        let source_tree = self.repo.find_commit(source_id)?.tree()?;

        let changes = self
            .repo
            .diff_tree_to_tree(Some(&base_tree), Some(&source_tree), None)?;

        let mut files = Vec::new();
        for change in changes {
            let file = match change {
                ChangeDetached::Addition { location, .. } => ChangedFile {
                    status: ChangeStatus::Added,
                    path: PathBuf::from(location.to_str_lossy().into_owned()),
                    previous_path: None,
                },
                ChangeDetached::Deletion { location, .. } => ChangedFile {
                    status: ChangeStatus::Deleted,
                    path: PathBuf::from(location.to_str_lossy().into_owned()),
                    previous_path: None,
                },
                ChangeDetached::Modification { location, .. } => ChangedFile {
                    status: ChangeStatus::Modified,
                    path: PathBuf::from(location.to_str_lossy().into_owned()),
                    previous_path: None,
                },
                ChangeDetached::Rewrite {
                    source_location,
                    location,
                    copy,
                    ..
                } => ChangedFile {
                    status: if copy {
                        ChangeStatus::Copied
                    } else {
                        ChangeStatus::Renamed
                    },
                    path: PathBuf::from(location.to_str_lossy().into_owned()),
                    previous_path: Some(PathBuf::from(source_location.to_str_lossy().into_owned())),
                },
            };
            files.push(file);
        }

        Ok(files)
    }

    /// Returns the content of `path` as it exists on `branch`, or `None` if the
    /// path does not exist there (e.g. it was deleted).
    pub fn read_blob(&self, branch: &str, path: &Path) -> Result<Option<Vec<u8>>> {
        let commit_id = self.repo.rev_parse_single(branch.as_bytes())?;
        let tree = self.repo.find_commit(commit_id)?.tree()?;

        match tree.lookup_entry_by_path(path)? {
            Some(entry) => Ok(Some(entry.object()?.data.clone())),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TestRepo;

    #[test]
    fn changed_files_reports_additions_modifications_and_deletions() {
        let repo = TestRepo::init();
        repo.write_file("a.txt", "base a");
        repo.write_file("keep.txt", "keep");
        repo.commit_all("base");

        repo.checkout_new_branch("feature");
        repo.write_file("a.txt", "changed a");
        repo.write_file("b.txt", "new b");
        repo.remove_file("keep.txt");
        repo.commit_all("change");

        let git_repo = GitRepository::open(repo.path()).unwrap();
        let mut changes = git_repo.changed_files("main", "feature").unwrap();
        changes.sort_by(|a, b| a.path.cmp(&b.path));

        assert_eq!(changes.len(), 3);

        assert_eq!(changes[0].path, PathBuf::from("a.txt"));
        assert_eq!(changes[0].status, ChangeStatus::Modified);

        assert_eq!(changes[1].path, PathBuf::from("b.txt"));
        assert_eq!(changes[1].status, ChangeStatus::Added);

        assert_eq!(changes[2].path, PathBuf::from("keep.txt"));
        assert_eq!(changes[2].status, ChangeStatus::Deleted);
    }

    #[test]
    fn changed_files_ignores_changes_made_on_target_after_branches_diverged() {
        let repo = TestRepo::init();
        repo.write_file("shared.txt", "base");
        repo.commit_all("base");

        repo.checkout_new_branch("feature");
        repo.write_file("feature_only.txt", "feature");
        repo.commit_all("feature change");

        repo.git(&["checkout", "-q", "main"]);
        repo.write_file("main_only.txt", "main");
        repo.commit_all("main change");

        let git_repo = GitRepository::open(repo.path()).unwrap();
        let changes = git_repo.changed_files("main", "feature").unwrap();
        let paths: Vec<_> = changes.iter().map(|c| c.path.clone()).collect();

        assert_eq!(paths, vec![PathBuf::from("feature_only.txt")]);
    }

    #[test]
    fn read_blob_returns_content_and_none_for_missing_or_deleted_paths() {
        let repo = TestRepo::init();
        repo.write_file("a.txt", "hello");
        repo.commit_all("base");

        repo.checkout_new_branch("feature");
        repo.remove_file("a.txt");
        repo.write_file("b.txt", "world");
        repo.commit_all("change");

        let git_repo = GitRepository::open(repo.path()).unwrap();

        assert_eq!(
            git_repo.read_blob("feature", Path::new("b.txt")).unwrap(),
            Some(b"world".to_vec())
        );
        assert_eq!(git_repo.read_blob("feature", Path::new("a.txt")).unwrap(), None);
        assert_eq!(
            git_repo.read_blob("main", Path::new("missing.txt")).unwrap(),
            None
        );
    }

    #[test]
    fn pull_branch_fetches_a_non_checked_out_branch_and_strips_origin_prefix() {
        let origin = TestRepo::init_bare();

        let seed = TestRepo::clone_from(&origin);
        seed.git(&["checkout", "-q", "-b", "dev"]);
        seed.write_file("base.txt", "base");
        seed.commit_all("base");
        seed.git(&["push", "-q", "-u", "origin", "dev"]);

        let repo = TestRepo::clone_from(&origin);
        repo.git(&["checkout", "-q", "dev"]);
        repo.git(&["checkout", "-q", "-b", "scratch"]);

        let other = TestRepo::clone_from(&origin);
        other.git(&["checkout", "-q", "dev"]);
        other.write_file("advance.txt", "advance");
        other.commit_all("advance");
        other.git(&["push", "-q", "origin", "dev"]);

        GitRepository::pull_branch(repo.path(), "origin/dev").unwrap();

        let git_repo = GitRepository::open(repo.path()).unwrap();
        assert_eq!(
            git_repo.read_blob("dev", Path::new("advance.txt")).unwrap(),
            Some(b"advance".to_vec())
        );
    }

    #[test]
    fn pull_branch_fast_forwards_a_checked_out_branch() {
        let origin = TestRepo::init_bare();

        let seed = TestRepo::clone_from(&origin);
        seed.git(&["checkout", "-q", "-b", "dev"]);
        seed.write_file("base.txt", "base");
        seed.commit_all("base");
        seed.git(&["push", "-q", "-u", "origin", "dev"]);

        let repo = TestRepo::clone_from(&origin);
        repo.git(&["checkout", "-q", "dev"]);

        let other = TestRepo::clone_from(&origin);
        other.git(&["checkout", "-q", "dev"]);
        other.write_file("advance.txt", "advance");
        other.commit_all("advance");
        other.git(&["push", "-q", "origin", "dev"]);

        GitRepository::pull_branch(repo.path(), "dev").unwrap();

        assert_eq!(
            std::fs::read_to_string(repo.path().join("advance.txt")).unwrap(),
            "advance"
        );
    }

    #[test]
    fn create_branch_checks_out_a_new_branch() {
        let repo = TestRepo::init();
        repo.write_file("a.txt", "base");
        repo.commit_all("base");

        GitRepository::create_branch(repo.path(), "feature").unwrap();

        assert_eq!(
            GitRepository::current_branch(repo.path()).unwrap().as_deref(),
            Some("feature")
        );
    }

    #[test]
    fn commit_changes_stages_only_the_given_paths() {
        let repo = TestRepo::init();
        repo.write_file("a.txt", "base");
        repo.commit_all("base");

        repo.write_file("a.txt", "changed");
        repo.write_file("untouched.txt", "should stay unstaged");

        GitRepository::commit_changes(repo.path(), &[PathBuf::from("a.txt")], "update a").unwrap();

        let status = repo.status_short();
        assert!(status.contains("?? untouched.txt"), "status was: {status}");
        assert!(!status.contains("a.txt"), "status was: {status}");
    }
}
