use std::process::Command;

fn main() {
    // Capture git commit SHA
    let commit_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Capture build timestamp
    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    // Capture rustc version
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Capture target triple
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // Determine debug build
    let debug = std::env::var("PROFILE")
        .map(|p| p == "debug")
        .unwrap_or(true);

    println!("cargo:rustc-env=DEVKIT_COMMIT_SHA={}", commit_sha);
    println!("cargo:rustc-env=DEVKIT_BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=DEVKIT_RUSTC_VERSION={}", rustc_version);
    println!("cargo:rustc-env=DEVKIT_TARGET={}", target);
    println!("cargo:rustc-env=DEVKIT_DEBUG={}", debug);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
