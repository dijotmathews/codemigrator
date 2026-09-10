use anyhow::Result;
use gix::bstr::ByteSlice;
use gix::object::tree::diff::ChangeDetached;
use std::path::{Path, PathBuf};

pub struct GitRepository {
    repo: gix::Repository,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = gix::open(path)?;
        Ok(Self { repo })
    }

    pub fn changed_files(
        &self,
        target_branch: &str,
        source_branch: &str,
    ) -> Result<Vec<PathBuf>> {
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

        let mut paths = Vec::new();
        for change in changes {
            let location = match change {
                ChangeDetached::Addition { location, .. } => location,
                ChangeDetached::Deletion { location, .. } => location,
                ChangeDetached::Modification { location, .. } => location,
                ChangeDetached::Rewrite { location, .. } => location,
            };
            paths.push(PathBuf::from(location.to_str_lossy().into_owned()));
        }

        Ok(paths)
    }
}
