# cargo-run-home-is-rustup-home

- **Domain:** cli
- **Date:** 2026-09-16
- **Pattern-Key:** cargo-run-home-is-rustup-home
- **Confidence:** high
- **Evidence:** `HOME=$scratch cargo run -p token-balance --bin tb -- init` → rustup: no default toolchain
- **Status:** active

## Situation

Proving `tb init` writes `{HOME}/.config/token-balance/accounts.toml` tempts `HOME=<scratch> cargo run ... init`.

## Decision

Build with the real `HOME`, then invoke `./target/debug/tb init` with `HOME` pointed at the isolated directory. Rejected: setting `HOME` for `cargo`/`rustup`.

## Reasoning

`cargo` and `rustup` read `$HOME` for toolchains. Scratch `HOME` has no rustup default, so `cargo run` never reaches `tb`. The binary only needs `HOME` to choose the accounts path (`Credentials::from_process`).

## Outcome

`HOME=$scratch ./target/debug/tb init` wrote the commented template; `cargo run` with that `HOME` did not.

## Lesson

Do not override `HOME` for `cargo` or `rustup`. Isolate `tb`'s home only on the already-built binary.
