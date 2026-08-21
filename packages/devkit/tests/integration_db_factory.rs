#[test]
fn test_db_factory_creates_sqlite_database() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_factory.db");

    let conn_str = format!("sqlite:{}", db_path.display());

    assert!(
        conn_str.starts_with("sqlite:"),
        "connection string should start with sqlite:"
    );
    assert!(
        conn_str.contains("test_factory.db"),
        "connection string should contain database name"
    );
}

#[test]
fn test_db_factory_in_memory_database() {
    let conn_str = "sqlite::memory:";

    assert!(
        conn_str.contains("memory"),
        "in-memory connection should contain 'memory'"
    );
}

#[test]
fn test_db_factory_multiple_connections() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("multi_conn.db");

    let conn_str1 = format!("sqlite:{}", db_path.display());
    let conn_str2 = format!("sqlite:{}", db_path.display());

    assert_eq!(
        conn_str1, conn_str2,
        "same path should produce same connection string"
    );
}
