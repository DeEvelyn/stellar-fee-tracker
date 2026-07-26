use std::path::Path;

pub fn cleanup_test_artifacts(dir: &Path) {
    if dir.exists() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

pub fn assert_no_temp_files_remaining(dir: &Path) {
    if dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(dir)
            .map(|e| e.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        assert!(
            entries.is_empty(),
            "Temp directory should be clean but has {} entries",
            entries.len()
        );
    }
}
