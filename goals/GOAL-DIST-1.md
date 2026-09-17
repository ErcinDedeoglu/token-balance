---
id: GOAL-DIST-1
title: CI, GitHub releases, and cross-platform install
status: in-progress
created: 2026-09-17
source: discussion
---

# GOAL-DIST-1: CI, GitHub releases, and cross-platform install

## Intent

A person on macOS, Linux, or Windows can install `token-balance` and `tb` from a GitHub Release (or `cargo install --git`) after CI has tested the same tree on those three operating systems.

## Problem

The public README only showed `cargo run` from a clone; there was no GitHub Actions workflow, no tagged binary release, and no documented install path for macOS, Linux, or Windows (user: workflows, releases, install on those OSes; `README.md` before this goal).

## Outcome

Pushes and PRs to `main` run `cargo test -p token-balance --locked` on Ubuntu, macOS, and Windows. A git tag `vX.Y.Z` equal to workspace `version` publishes five platform archives plus checksums. README lists curl, PowerShell, and cargo-git install commands.

## Scope

### In

- GitHub Actions CI on `ubuntu-latest`, `macos-latest`, `windows-latest`
- Tag-triggered GitHub Release with archives for five rustc targets and both bins
- `tooling/install/install.sh` and `tooling/install/install.ps1`
- README install section with those commands and `cargo install --git`
- Crate `repository` / `homepage` metadata for later publish (not the publish itself)

### Out

- Do not publish the crate to crates.io in this goal
- Do not add Homebrew, Scoop, winget, deb, or rpm packages
- Do not add Windows ARM64 or Linux musl release artifacts
- Do not change TUI, adapters, or quota mapping
- Do not add an auto-update network check
- Do not generate cargo-dist workflow files

## Context

- Constraints: GitHub repo `ErcinDedeoglu/token-balance`; workspace version `0.1.0`; rust-version 1.85; QUALITY `.github/**` already a kit folder; `.gitignore` has `dist/` so pack under `target/pack/`; both bins share `src/main.rs`
- Files / systems: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `tooling/install/install.sh`, `tooling/install/install.ps1`, `README.md`, `Cargo.toml`, `crates/token-balance/Cargo.toml`
- Commands: `cargo test -p token-balance --locked`; `cargo build -p token-balance --locked`; `cargo install --path crates/token-balance --locked`; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`; Lint `n/a`; Typecheck `n/a`

## Decisions

| Decision | Provenance |
| --- | --- |
| GitHub Actions workflows (CI + release) | user: "it should has workflows" |
| GitHub Releases from tags `v*.*.*` | user: "releases" |
| Install on macOS, Linux, and Windows | user: "let ppl easy install, doesnt matter if macos or linux or windows" |
| Unix `curl \| sh`, Windows `irm \| iex`, plus `cargo install --git` | this discussion; documented in `README.md` |
| Five targets: linux gnu x64/arm64, macOS intel/arm64, Windows msvc x64 | this discussion (covers the three OS request; Windows ARM64 parked in Out) |
| Pack both `token-balance` and `tb`; assets `token-balance-<target>.tar.gz` or `.zip` | this discussion; `release.yml` pack step |

## Assumptions

- `GITHUB_TOKEN` on this repo can create releases
- `ubuntu-24.04-arm` is available for `aarch64-unknown-linux-gnu`
- GitHub `releases/latest/download` serves the named assets after the first tag
- Users who pipe the install scripts trust this GitHub repo over HTTPS

## Open questions

n/a

## Work

- [x] **T1** Add `.github/workflows/ci.yml` running locked test and build on three OS runners → AC-1
- [x] **T2** Add `.github/workflows/release.yml` that builds five targets, packs both bins, writes `sha256sums.txt`, and fails when the tag does not match `Cargo.toml` version → AC-2, AC-6
- [x] **T3** Add `tooling/install/install.sh` mapping Darwin/Linux uname to the four unix targets → AC-4
- [x] **T4** Add `tooling/install/install.ps1` for Windows x64 and an ARM64 error that names `cargo install --git` → AC-4, AC-5
- [x] **T5** Write README install commands for curl, PowerShell, and cargo-git → AC-3
- [ ] **T6** Push git tag `v0.1.0` to `origin` after the workflow files are on `main` → AC-2

## Acceptance criteria

### Must

- [ ] **AC-1** Given a push or pull request to `main`, when CI runs, then the `test` job executes `cargo test -p token-balance --locked` and `cargo build -p token-balance --locked` on `ubuntu-latest`, `macos-latest`, and `windows-latest`.
      Verify: `rg -n "ubuntu-latest, macos-latest, windows-latest" .github/workflows/ci.yml` and `rg -n "cargo test -p token-balance --locked" .github/workflows/ci.yml`; after merge, the Actions run for that SHA is green
- [ ] **AC-2** Given `main` contains the release workflow and git tag `v0.1.0` (matching workspace `version`), when the release workflow finishes, then a GitHub Release exists whose assets include `token-balance-x86_64-unknown-linux-gnu.tar.gz`, `token-balance-aarch64-unknown-linux-gnu.tar.gz`, `token-balance-x86_64-apple-darwin.tar.gz`, `token-balance-aarch64-apple-darwin.tar.gz`, `token-balance-x86_64-pc-windows-msvc.zip`, and `sha256sums.txt`, and each archive contains both `token-balance` and `tb` (`.exe` on Windows).
      Verify: `rg -n "target:" .github/workflows/release.yml`; after tag push, `gh release view v0.1.0 --json assets --jq '.assets[].name'`
- [ ] **AC-3** Given the repository README, when a reader opens the Install section, then it contains the exact `curl -fsSL` line for `tooling/install/install.sh`, the `irm` / `iex` line for `tooling/install/install.ps1`, and `cargo install --git https://github.com/ErcinDedeoglu/token-balance --locked`.
      Verify: `rg -n "install.sh|install.ps1|cargo install --git" README.md`
- [ ] **AC-4** Given a published latest release with those asset names, when `install.sh` runs on Darwin or Linux, then it downloads `token-balance-<mapped-target>.tar.gz` and writes executable `token-balance` and `tb` into `TOKEN_BALANCE_BIN_DIR` or `$HOME/.local/bin`; when `install.ps1` runs on Windows x64, then it writes `token-balance.exe` and `tb.exe` into `%LOCALAPPDATA%\token-balance\bin` unless `TOKEN_BALANCE_BIN_DIR` is set.
      Verify: `rg -n "aarch64-apple-darwin|x86_64-apple-darwin|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu" tooling/install/install.sh`; `rg -n "x86_64-pc-windows-msvc" tooling/install/install.ps1`; after AC-2, run the matching installer and `tb --version`
- [ ] **AC-5** Given Windows ARM64, when `install.ps1` runs, then it throws and the message contains `cargo install --git` and does not download the x64 zip.
      Verify: `rg -n "Arm64" tooling/install/install.ps1` and `rg -n "cargo install --git" tooling/install/install.ps1`
- [ ] **AC-6** Given a tag `v0.1.1` while `Cargo.toml` workspace `version` is `0.1.0`, when the release workflow runs, then the `Tag matches Cargo.toml` step exits non-zero and no GitHub Release is published for that tag.
      Verify: `rg -n "Tag matches Cargo.toml" .github/workflows/release.yml`; `rg -n "tag-matches-version.sh" .github/workflows/release.yml`; `bash tooling/install/tag-matches-version.sh v0.1.1` exits non-zero while workspace version is 0.1.0

### Should

- [ ] **AC-S1** Given a successful release, when assets are listed, then `sha256sums.txt` is present next to the five archives.
      Verify: after AC-2, `gh release view v0.1.0 --json assets --jq '.assets[].name'` includes `sha256sums.txt`

## Definition of done

- [ ] Every Must AC is `[x]` with evidence under Evidence
- [ ] Work items that those AC require are `[x]`
- [ ] Applicable gates pass: `cargo test -p token-balance --locked` / `n/a` lint / `n/a` typecheck; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`
- [ ] No secrets in the file or the change
- [ ] Leftovers filed as a later `GOAL-DIST-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pending | |
| AC-2 | pending | |
| AC-3 | pending | |
| AC-4 | pending | |
| AC-5 | pending | |
| AC-6 | pending | |

## Risks

- `install.sh` / `install.ps1` return HTTP 404 until T6 publishes `v0.1.0`
- `ubuntu-24.04-arm` runner quota or availability can block the linux aarch64 archive; mitigation is fail-fast false so other targets still upload
- Piping remote scripts to `sh` / `iex` is a trust decision; HTTPS GitHub is the only channel this goal uses
