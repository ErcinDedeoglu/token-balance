# crates/token-balance/

Bins `token-balance` and `tb` share `src/main.rs`. Ledger-1 remaining-quota board.

## Chain

- Up: `../AGENTS.md`
- Down: `src/AGENTS.md`

## Delta

- Dev: `cargo run -p token-balance --bin token-balance -- --fixture mixed`
- Version: `cargo run -p token-balance --bin token-balance -- --version`; same with `--bin tb`
- Test: `cargo test -p token-balance`
- Never log secrets or credential file contents
- Never fake numeric `0%` for `NotConfigured` / `Unsupported`
- Live registry is `{HOME}/.config/token-balance/accounts.toml` only; never auto-include unlisted CLI logins
- Never mix live adapters with `--fixture` in one process
- Vendor HTTP nevers: `src/adapters/AGENTS.md`
