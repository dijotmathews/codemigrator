use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// A throwaway git repository on disk, used to exercise `GitRepository`,
/// `DiffService` and `CopyService` against real git plumbing instead of mocks.
pub struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    pub fn init() -> Self {
        let repo = Self::empty();
        repo.git(&["init", "-q", "-b", "main"]);
        repo.configure();
        repo
    }

    pub fn init_bare() -> Self {
        let repo = Self::empty();
        repo.git(&["init", "-q", "--bare"]);
        repo
    }

    pub fn clone_from(origin: &TestRepo) -> Self {
        let dir = TempDir::new().expect("create temp dir");
        let status = Command::new("git")
            .arg("clone")
            .arg("-q")
            .arg(origin.path())
            .arg(dir.path())
            .status()
            .expect("run git clone");
        assert!(status.success(), "git clone failed");

        let repo = Self { dir };
        repo.configure();
        repo
    }

    fn empty() -> Self {
        Self {
            dir: TempDir::new().expect("create temp dir"),
        }
    }

    fn configure(&self) {
        self.git(&["config", "user.email", "test@example.com"]);
        self.git(&["config", "user.name", "Test"]);
        self.git(&["config", "commit.gpgsign", "false"]);
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(args)
            .status()
            .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
        assert!(status.success(), "git {args:?} failed");
    }

    pub fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.path().join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    pub fn remove_file(&self, relative_path: &str) {
        std::fs::remove_file(self.path().join(relative_path)).unwrap();
    }

    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    pub fn checkout_new_branch(&self, name: &str) {
        self.git(&["checkout", "-q", "-b", name]);
    }

    pub fn status_short(&self) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(["status", "--short"])
            .output()
            .expect("run git status");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}
