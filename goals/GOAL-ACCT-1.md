---
id: GOAL-ACCT-1
title: Config-listed account cards
status: done
created: 2026-09-16
source: discussion
---

# GOAL-ACCT-1: Config-listed account cards

## Intent

A developer running live `tb` sees one remaining-quota card per account listed in the token-balance accounts file so multiple subscriptions per vendor can sit on the risk board while unlisted vendors stay off it and Codex remains a singleton.

## Problem

Live `tb` always builds six vendor adapters from default `$HOME` paths and env vars (`crates/token-balance/src/adapters.rs` `live_registry`), and `Provider::id` is a static vendor slug (`crates/token-balance/src/providers.rs`), so a second Claude or Kimi cannot appear and unsigned vendors still occupy cells.

## Outcome

Without `--fixture`, the registry is exactly the `[[account]]` rows in `{HOME}/.config/token-balance/accounts.toml`. Each row is one card. A missing file is an empty board with a path hint. A second Codex row fails load before any board is shown.

## Scope

### In

- Accounts file schema (vendor, id, label, credential pointer) and load errors
- Live registry built only from that file
- One card per listed account; same-vendor multiples share the vendor glyph
- `tb init` commented template; missing-file empty board
- `--fixture multi` layout oracle for more than six cards
- Design SSOT updates so `docs/design/` matches config-listed membership

### Out

- Do not auto-include official CLI logins that are absent from the accounts file
- Do not add in-TUI add/remove (`a` / `d`) or a config editor overlay
- Do not add profile/tag filters or `--profile` / `--tag`
- Do not add MiniMax, Antigravity, Copilot, OpenRouter, or DeepSeek vendors
- Do not store inline API keys or tokens in the accounts file
- Do not write or refresh `~/.claude/.credentials.json`, `~/.codex/`, `~/.grok/auth.json`, or Muse auth files
- Do not implement cookie stealers, password login, or a keyring crate
- Do not allow a second Codex account
- Do not mix live adapters with fixture providers in one process
- Do not ship `--json`, `--theme`, or statusline in this goal
- Do not add a daemon, SQLite, or Prometheus
- Do not shrink the 7-row card or change the 80 / 140 column breakpoints
- Do not merge two accounts' remaining windows into one bar

## Context

- Constraints: Rust edition 2024; existing `dyn Provider` + boxed `FetchFuture`; read-only credential pointers (env name or extra file path); glyph stays the vendor letter (`L X K G Z M`); Ember Ledger; QUALITY + agent-kit; tests inject `Credentials.home` (no live vendor HTTP)
- Files / systems: `crates/token-balance/src/adapters.rs`, `providers.rs`, `credentials.rs`, `tui.rs`, `main.rs`, `cards.rs`, `docs/design/cli.md`, `docs/design/providers.md`, `docs/design/architecture.md`, `docs/design/layout.md`, `{HOME}/.config/token-balance/accounts.toml`
- Commands: `cargo test -p token-balance`; `cargo build -p token-balance`; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`; Lint `n/a`; Typecheck `n/a`

## Decisions

| Decision | Provenance |
| --- | --- |
| One card per account, not per vendor | user: picked "One card per account" |
| Membership is only what the config file lists; no auto-include | user: picked "Only what a config file lists — no auto-include" |
| Config we own; pointers to env vars or extra credential files | user: picked "A config file we own, pointing at env vars or extra credential files" |
| Multiple accounts per vendor except Codex | user: "i can have multiple account for each"; "i can only have one codex account" |
| TOML at `{HOME}/.config/token-balance/accounts.toml` | agent: recommended default; user invoked `/goal-file` without changing it |
| Missing file enters TUI with zero provider cards and a path hint (does not exit 1) | agent: recommended default; user invoked `/goal-file` without changing it |
| `tb init` writes a commented template with zero enabled `[[account]]` rows | agent: recommended default; user invoked `/goal-file` without changing it |
| Labels unique across the file; title is glyph + label | agent: header `worst` would be ambiguous if two vendors share `work` |
| Pager allowed when account count > 6; 7-row cards and 80/140 breakpoints stay | agent: 80×24 no-pager was GOAL-QUOTA-1 for six mixed fixtures |
| Soft cap 24 accounts | agent: recommended default; parked as Should |

## Assumptions

- The implementer may add a TOML parser dependency; the user did not name a crate
- Extra Claude/Grok/Muse files are copies the user maintains; this goal does not refresh OAuth
- `Credentials.home` isolation in tests is enough to avoid writing the real XDG config

## Open questions

n/a

## Work

- [x] **T1** Parse `{HOME}/.config/token-balance/accounts.toml` and reject a second Codex row, an inline key field, an unknown vendor, a duplicate id, and a duplicate label → AC-4, AC-6
- [x] **T2** Return an empty provider list and surface the config path when the accounts file is missing → AC-3
- [x] **T3** Add `tb init` that writes a commented template with zero enabled `[[account]]` rows → AC-3
- [x] **T4** Change `Provider::id` / `display_name` to non-static strings, add `vendor()`, and build the live registry only from parsed accounts → AC-1, AC-2
- [x] **T5** Render two same-vendor cards with a shared glyph and distinct labels → AC-2
- [x] **T6** Paint `NotConfigured` on a listed account whose pointer is missing while other listed accounts still appear → AC-5
- [x] **T7** Keep `--fixture mixed` as six vendor fixtures that do not read the accounts file → AC-8
- [x] **T8** Add `--fixture multi` with more than six accounts and lock `(80, 24)` as 2-column 7-row cards with a pager prefix → AC-7
- [x] **T9** Re-read the accounts file on `r` and apply membership changes without adding unlisted vendors → AC-9
- [x] **T10** Update `docs/design/cli.md`, `providers.md`, `architecture.md`, and `layout.md` for config-listed account cards → AC-1

## Acceptance criteria

### Must

- [x] **AC-1** Given no `--fixture` and an accounts file that lists Claude `work`, Kimi `team`, and no Muse row, when live `tb` builds the registry, then the snapshot ids are exactly those two accounts and no Muse card exists.
      Verify: `cargo test -p token-balance` (live registry from an isolated accounts file)
- [x] **AC-2** Given two Claude rows with ids `claude-work` / `claude-home` and labels `work` / `home`, when the board renders, then two cards share glyph `L` and the titles include `work` and `home`.
      Verify: `cargo test -p token-balance` (same-vendor card title assertions)
- [x] **AC-3** Given `{HOME}/.config/token-balance/accounts.toml` does not exist, when live `tb` starts, then the process exit code is 0, zero provider cards render, and the screen text contains `.config/token-balance/accounts.toml`.
      Verify: `cargo test -p token-balance` (missing-file empty board); `cargo run -p token-balance --bin tb -- init` then file exists with no uncommented `[[account]]`
- [x] **AC-4** Given an accounts file with two `vendor = "codex"` rows, when live `tb` loads it, then load fails, stderr names Codex, and no provider snapshots are shown.
      Verify: `cargo test -p token-balance` (Codex singleton load error)
- [x] **AC-5** Given a listed z.ai row whose `api_key_env` is unset and a listed Kimi row whose pointer exists, when the registry is built, then z.ai is `NotConfigured` and Kimi is still present.
      Verify: `cargo test -p token-balance` (missing-pointer `NotConfigured` isolation)
- [x] **AC-6** Given an accounts file that contains an inline key field (not an env or path pointer), when live `tb` loads it, then load fails and no provider snapshots are shown.
      Verify: `cargo test -p token-balance` (inline key rejected)
- [x] **AC-7** Given `--fixture multi` with more than six accounts, when TestBackend renders `(80, 24)`, then the grid is 2 columns, every card is 7 rows, and the footer contains a pager prefix matching `[0-9]+–[0-9]+ / [0-9]+`.
      Verify: `cargo test -p token-balance` (multi fixture 80×24 snapshot)
- [x] **AC-8** Given `--fixture mixed`, when the board renders, then six fixture vendor cards appear and the accounts file is not read.
      Verify: `cargo test -p token-balance` (existing mixed snapshots still pass; fixture path ignores accounts.toml)
- [x] **AC-9** Given live `tb` already showing two listed accounts, when a third valid row is added to the file and the user presses `r`, then three cards are present and unlisted vendors still do not appear.
      Verify: `cargo test -p token-balance` (refresh re-reads accounts file)

### Should

- [x] **AC-S1** Given 25 `[[account]]` rows with unique ids and labels and at most one Codex, when live `tb` loads the file, then load fails and the error text contains `24`.
      Verify: `cargo test -p token-balance` (account cap)

## Definition of done

- [x] Every Must AC is `[x]` with evidence under Evidence
- [x] Work items that those AC require are `[x]`
- [x] Applicable gates pass: `cargo test -p token-balance` / `tooling/agent-kit/check.sh` / `.githooks/check-quality.sh` / Lint `n/a` / Typecheck `n/a`
- [x] No secrets in the file or the change
- [x] Leftovers filed as a later `GOAL-ACCT-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pass | `live_registry_lists_only_file_rows`; `two_claude_cards_share_glyph_and_labels` snapshot ids; no Muse |
| AC-2 | pass | `live_registry_two_claude_share_glyph`; `two_claude_cards_share_glyph_and_labels` (glyph `L`, titles `work`/`home`) |
| AC-3 | pass | `missing_accounts_file_empty_board_names_path`; `init_template_has_no_enabled_account`; `init_template_uncommented_parses_to_rows`; `./target/debug/tb init` wrote `# [[account]]` examples |
| AC-4 | pass | `live_registry_two_codex_is_error`; `accounts::tests::two_codex_rows_fail` |
| AC-5 | pass | `live_registry_missing_pointer_isolates_not_configured` |
| AC-6 | pass | `live_registry_inline_key_is_error`; `accounts::tests::inline_key_field_fails` |
| AC-7 | pass | `fixture_multi_80x24_pager_and_seven_row_cards` |
| AC-8 | pass | `mixed_snapshots_four_sizes`; `open_registry_fixture_ignores_accounts_file`; `fixture_mixed_is_whole_registry` |
| AC-9 | pass | `refresh_rereads_accounts_file` |

## Risks

- Config-only membership breaks GOAL-QUOTA-1 auto-detect of six CLI logins; accepted by the no-auto-include picker
- Copied Claude/Grok OAuth files expire; card becomes `Error` with last bars; this goal does not refresh those files
- N Muse cards × `r` can burn N Everyday requests; keep `OnDemand` and overlay copy
- `--fixture mixed` 80×24 no-pager AC from GOAL-QUOTA-1 must stay green (AC-8)
