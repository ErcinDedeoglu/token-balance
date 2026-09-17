# crates/token-balance/

Bins `token-balance` and `tb` share `src/main.rs`. Ledger-1 remaining-quota board.

- Dev: `cargo run -p token-balance --bin token-balance -- --fixture mixed`
- Version: `cargo run -p token-balance --bin token-balance -- --version`; same with `--bin tb`
- Test: `cargo test -p token-balance`
- Never log secrets or credential file contents
- Never fake numeric `0%` for `NotConfigured` / `Unsupported`
- Live registry is `{HOME}/.config/token-balance/accounts.toml` only; never auto-include unlisted CLI logins
- Never mix live adapters with `--fixture` in one process
- Never call Moonshot PAYG balance, Anthropic Admin Usage, or xAI prepaid on plan cards
- Never call Exa Team Management `/usage` (`total_cost_usd`) as remaining
- Never commit `exa.json` dashboard cookies
