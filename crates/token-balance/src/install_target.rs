//! Platform → rustc target used by `tooling/install/install.sh` and `install.ps1`.
//! Scripts stay the download path (no Rust required). This module is the
//! mapping those scripts must match; tests call it and exec `--print-target`.

#[derive(Debug, PartialEq, Eq)]
pub enum InstallTargetError {
    WindowsArm64,
    Unsupported,
}

#[allow(dead_code)]
pub fn rustc_target(os: &str, arch: &str) -> Result<&'static str, InstallTargetError> {
    match (os, arch) {
        ("Darwin", "arm64") => Ok("aarch64-apple-darwin"),
        ("Darwin", "x86_64") => Ok("x86_64-apple-darwin"),
        ("Linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("Linux", "aarch64") | ("Linux", "arm64") => Ok("aarch64-unknown-linux-gnu"),
        ("Windows", "x86_64") | ("Windows", "X64") => Ok("x86_64-pc-windows-msvc"),
        ("Windows", "Arm64") | ("Windows", "ARM64") => Err(InstallTargetError::WindowsArm64),
        _ => Err(InstallTargetError::Unsupported),
    }
}

#[allow(dead_code)]
pub fn windows_arm64_message(repo: &str) -> String {
    format!(
        "token-balance install: Windows ARM64 is not in GitHub releases yet. Use: cargo install --git https://github.com/{repo} --locked"
    )
}

#[cfg(test)]
#[path = "install_target_test.rs"]
mod tests;
