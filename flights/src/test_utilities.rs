pub struct TestPath {
    _guard: tempfile::TempDir,
    pub path: std::path::PathBuf,
}

#[rstest::fixture]
pub fn test_path() -> TestPath {
    let guard = tempfile::tempdir().expect("Failed to create temporary directory");
    let path = guard.path().to_path_buf();
    TestPath {
        _guard: guard,
        path,
    }
}

#[rstest::fixture]
pub fn test_data_path() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data");
    path
}
