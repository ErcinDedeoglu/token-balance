# crates/

Cargo workspace for the remaining-quota TUI. One member: `token-balance`.

- Version: `cargo run -p token-balance --bin token-balance -- --version` and `--bin tb`
- Test: `cargo test -p token-balance` (recorded JSON only; no live vendor HTTP)
- Never commit `target/`
- Nested `token-balance/AGENTS.md` is the crate delta
