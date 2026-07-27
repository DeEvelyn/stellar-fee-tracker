use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TestContext {
    pub temp_dir: TempDir,
    pub fixtures_dir: PathBuf,
}

impl TestContext {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let fixtures_dir = temp_dir.path().join("fixtures");
        std::fs::create_dir_all(&fixtures_dir).expect("Failed to create fixtures dir");

        Self {
            temp_dir,
            fixtures_dir,
        }
    }

    pub fn fixture_path(&self, name: &str) -> PathBuf {
        self.fixtures_dir.join(name)
    }

    pub fn write_fixture(&self, name: &str, content: &str) -> PathBuf {
        let path = self.fixture_path(name);
        std::fs::write(&path, content).expect("Failed to write fixture");
        path
    }
}

pub struct TestDatabase {
    path: PathBuf,
}

impl TestDatabase {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("test_{}.db", uuid::Uuid::new_v4()));
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection_string(&self) -> String {
        format!("sqlite:{}", self.path.display())
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(format!("{}-journal", self.path.display()));
        let _ = std::fs::remove_file(format!("{}-wal", self.path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.path.display()));
    }
}
