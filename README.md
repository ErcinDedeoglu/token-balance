# token-balance

Remaining-quota risk board for AI coding plans. Same program: `token-balance` and `tb`.

[![ci](https://github.com/ErcinDedeoglu/token-balance/actions/workflows/ci.yml/badge.svg)](https://github.com/ErcinDedeoglu/token-balance/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

## Install

macOS and Linux (latest GitHub release):

```bash
curl -fsSL https://raw.githubusercontent.com/ErcinDedeoglu/token-balance/main/tooling/install/install.sh | sh
```

Puts `token-balance` and `tb` in `~/.local/bin`. Override with `TOKEN_BALANCE_BIN_DIR` or `PREFIX`.

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/ErcinDedeoglu/token-balance/main/tooling/install/install.ps1 | iex
```

From source (any OS with Rust):

```bash
cargo install --git https://github.com/ErcinDedeoglu/token-balance --locked
```

Binaries for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), and Windows (x64) are attached to each [GitHub release](https://github.com/ErcinDedeoglu/token-balance/releases). Tag `vX.Y.Z` matching `Cargo.toml` to cut one.

## Use

```bash
tb init
tb
tb --fixture mixed
```

`tb init` writes a commented `~/.config/token-balance/accounts.toml`. Live cards are listed rows only. `--fixture mixed` is the screenshot board (no vendor HTTP).

Approved design: [`docs/design/`](docs/design/). Agent instructions: `AGENTS.md`.

## License

MIT OR Apache-2.0
