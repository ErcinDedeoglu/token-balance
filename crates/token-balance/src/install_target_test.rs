use super::*;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn print_target(os: &str, arch: &str) -> (bool, String, String) {
    let script = repo_root().join("tooling/install/install.sh");
    let out = Command::new("bash")
        .args([script.to_str().unwrap(), "--print-target", os, arch])
        .output()
        .expect("bash install.sh --print-target");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    )
}

fn tag_matches(tag: &str) -> bool {
    let script = repo_root().join("tooling/install/tag-matches-version.sh");
    Command::new("bash")
        .args([script.to_str().unwrap(), tag])
        .status()
        .expect("tag-matches-version.sh")
        .success()
}

#[test]
fn darwin_arm64_is_aarch64_apple_darwin() {
    assert_eq!(rustc_target("Darwin", "arm64"), Ok("aarch64-apple-darwin"));
    let (ok, stdout, _) = print_target("Darwin", "arm64");
    assert!(ok, "install.sh --print-target Darwin arm64");
    assert_eq!(stdout, "aarch64-apple-darwin");
}

#[test]
fn darwin_x86_64_is_x86_64_apple_darwin() {
    assert_eq!(rustc_target("Darwin", "x86_64"), Ok("x86_64-apple-darwin"));
    let (ok, stdout, _) = print_target("Darwin", "x86_64");
    assert!(ok);
    assert_eq!(stdout, "x86_64-apple-darwin");
}

#[test]
fn linux_x86_64_is_x86_64_unknown_linux_gnu() {
    assert_eq!(
        rustc_target("Linux", "x86_64"),
        Ok("x86_64-unknown-linux-gnu")
    );
    let (ok, stdout, _) = print_target("Linux", "x86_64");
    assert!(ok);
    assert_eq!(stdout, "x86_64-unknown-linux-gnu");
}

#[test]
fn linux_aarch64_is_aarch64_unknown_linux_gnu() {
    assert_eq!(
        rustc_target("Linux", "aarch64"),
        Ok("aarch64-unknown-linux-gnu")
    );
    assert_eq!(
        rustc_target("Linux", "arm64"),
        Ok("aarch64-unknown-linux-gnu")
    );
    let (ok, stdout, _) = print_target("Linux", "aarch64");
    assert!(ok);
    assert_eq!(stdout, "aarch64-unknown-linux-gnu");
}

#[test]
fn windows_x64_is_msvc() {
    assert_eq!(
        rustc_target("Windows", "X64"),
        Ok("x86_64-pc-windows-msvc")
    );
    let (ok, stdout, _) = print_target("Windows", "X64");
    assert!(ok);
    assert_eq!(stdout, "x86_64-pc-windows-msvc");
}

#[test]
fn windows_arm64_refuses_with_cargo_install_git() {
    assert_eq!(
        rustc_target("Windows", "Arm64"),
        Err(InstallTargetError::WindowsArm64)
    );
    let msg = windows_arm64_message("ErcinDedeoglu/token-balance");
    assert!(msg.contains("cargo install --git"));
    let (ok, stdout, stderr) = print_target("Windows", "Arm64");
    assert!(!ok, "Windows ARM64 must fail");
    assert!(stdout.is_empty());
    assert!(stderr.contains("cargo install --git"), "{stderr}");
    assert!(!stderr.contains(".zip"), "{stderr}");

    let ps1 = std::fs::read_to_string(repo_root().join("tooling/install/install.ps1"))
        .expect("install.ps1");
    let arm_at = ps1.find("Arm64").expect("Arm64 in install.ps1");
    let download_at = ps1
        .find("Invoke-WebRequest")
        .expect("Invoke-WebRequest in install.ps1");
    assert!(
        arm_at < download_at,
        "ARM64 refusal must run before download"
    );
    assert!(ps1.contains("cargo install --git"));
    assert!(ps1.contains(&msg) || ps1.contains("cargo install --git https://github.com/$Repo --locked"));
}

#[test]
fn tag_v0_1_0_matches_workspace_version() {
    assert!(tag_matches("v0.1.0"));
    assert!(tag_matches("0.1.0"));
}

#[test]
fn tag_v0_1_1_does_not_match_workspace_0_1_0() {
    assert!(!tag_matches("v0.1.1"));
}
