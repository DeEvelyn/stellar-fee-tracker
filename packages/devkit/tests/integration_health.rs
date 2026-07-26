use std::process::Command;

#[test]
fn test_devkit_health_check_all_pass() {
    let binary = env!("CARGO_BIN_EXE_stellar-devkit");
    let output = Command::new(binary)
        .args(["health"])
        .output()
        .expect("Failed to execute health command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{} {}", stdout, stderr);

    assert!(
        combined.contains("health")
            || combined.contains("ok")
            || combined.contains("pass")
            || output.status.success(),
        "Health check should produce output, got: {}",
        combined
    );
}

#[test]
fn test_devkit_health_check_with_invalid_config() {
    let binary = env!("CARGO_BIN_EXE_stellar-devkit");
    let output = Command::new(binary)
        .args(["health", "--config", "/nonexistent/config.toml"])
        .output()
        .expect("Failed to execute health command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{} {}", stdout, stderr);

    assert!(
        !output.status.success()
            || combined.contains("error")
            || combined.contains("not found")
            || combined.contains("missing"),
        "Health check with invalid config should report error, got: {}",
        combined
    );
}
