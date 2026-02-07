use std::path::PathBuf;
use tempfile::TempDir;

/// Test storage configuration.
pub struct TestStorage {
    pub temp_dir: TempDir,
    pub base_path: PathBuf,
    pub base_url: String,
}

impl TestStorage {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let base_path = temp_dir.path().to_path_buf();
        let base_url = "http://localhost:3000/media".to_string();
        Self {
            temp_dir,
            base_path,
            base_url,
        }
    }

    pub fn base_path_str(&self) -> String {
        self.base_path.to_string_lossy().to_string()
    }
}

impl Default for TestStorage {
    fn default() -> Self {
        Self::new()
    }
}
