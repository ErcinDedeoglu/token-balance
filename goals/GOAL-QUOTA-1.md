---
id: GOAL-QUOTA-1
title: Remaining-quota risk board
status: ready
created: 2026-09-15
source: discussion
---

# GOAL-QUOTA-1: Remaining-quota risk board

## Intent

A developer running `token-balance` or `tb` sees remaining 5-hour and weekly coding-plan quota for Claude, Codex, Kimi, Grok, z.ai, and Muse as a risk-sorted Ember Ledger card grid so they can tell who runs out first without mixing spend JSONL.

## Problem

No binary exists; `docs/design/` is approved but `crates/` is empty, so remaining plan quota still lives in vendor `/status` pages and spend tools like ccusage (discussion; `docs/design/README.md`).

## Outcome

`token-balance` and `tb` ship from `crates/token-balance`: fixture TUI first, then live adapters for Kimi, z.ai, Codex, Claude, Grok, and Muse, matching `docs/design/pr-plan.md` PR1–PR10.

## Scope

### In

- Cargo workspace `crates/token-balance` with bins `token-balance` and `tb`
- Domain types, fixtures, Ember Ledger TUI, interaction
- Live adapters: Kimi, z.ai, Codex, Claude, Grok, Muse (Unsupported then opt-in OnDemand)
- QUALITY PR1 gates (mandated Cargo/LICENSE files, `Cargo.lock` ignore, nested `AGENTS.md`, delete `trivial.md`)

### Out

- Do not add ccusage JSONL spend analytics or ledger-3 token counts on the card face
- Do not add a daemon, SQLite, or Prometheus
- Do not implement password login or cookie stealers
- Do not call Moonshot PAYG `api.moonshot.ai/v1/users/me/balance` on the Kimi card
- Do not call Anthropic Admin Usage & Cost API
- Do not merge xAI API prepaid into the SuperGrok card
- Do not poll Muse on the 60s timer
- Do not fork quotas or OpenUsage
- Do not add MiniMax, Antigravity, Copilot, OpenRouter, or DeepSeek cards
- Do not ship `--json`, `--theme`, or statusline in this goal
- Do not mix live adapters with fixture providers in one process
- Do not name folders `utils`, `helpers`, `common`, or `misc`
- Do not put product files under `crates/` before nested `AGENTS.md`

## Context

- Constraints: Rust edition 2024 / rust-version 1.85; ratatui + crossterm; tokio from PR4; boxed `FetchFuture` + `dyn Provider`; Ember Ledger amber `#E8A838`; license MIT OR Apache-2.0; read-only CLI credentials; QUALITY `doc`/`source` budgets; agent-kit nested `AGENTS.md`
- Files / systems: `docs/design/` (SSOT), `docs/QUALITY.md`, `AGENTS.md`, `crates/token-balance/` (to create), `src/adapters/` from first live adapter
- Commands: `tooling/agent-kit/check.sh`; `.githooks/check-quality.sh`; `cargo run -p token-balance --bin token-balance -- --version`; `cargo run -p token-balance --bin tb -- --version`; `cargo test -p token-balance`; Lint `n/a`; Typecheck `n/a`

## Decisions

| Decision | Provenance |
| --- | --- |
| Ledger 1 remaining-quota risk board only | `docs/design/product.md`; user: show token balance of AI coding plans |
| Rust 2024 + ratatui; one crate; bins `token-balance` and `tb` | `docs/design/decisions.md`; user: `tb` in v1 |
| Ember Ledger amber hero, teal extra | `docs/design/theme.md`; user: keep amber |
| Fixture TUI first; live adapters PR5–10 serialized | `docs/design/pr-plan.md` |
| `MIT OR Apache-2.0` with `LICENSE-MIT` + `LICENSE-APACHE` | user: MIT OR Apache-2.0 |
| Muse Unsupported in v1; OnDemand last | `docs/design/providers.md` |
| Store used and remaining; constructors only | `docs/design/domain.md` |
| TestBackend + Fill(1) is the layout oracle | `docs/design/layout.md`; `docs/design/mockups.md` |

## Assumptions

- rustc 1.85+ is available on the implementer machine
- Undocumented remaining-quota endpoints in `docs/design/providers.md` still respond with the documented shapes
- Official CLIs already store credentials on disk when a live adapter is enabled

## Open questions

n/a

## Work

- [ ] **T1** Create `crates/AGENTS.md` and `crates/token-balance/AGENTS.md`, then the Cargo workspace, `LICENSE-MIT`, `LICENSE-APACHE`, QUALITY mandated list, LICENSE Path class, and `.qualityignore` for `Cargo.lock` → AC-1
- [ ] **T2** Add `token-balance` and `tb` bins that print `{argv0} {version}`, run both `--version` once, fill Dev/Build, delete `tooling/agent-kit/trivial.md` → AC-1
- [ ] **T3** Add domain types, both percent constructors, `apply_fetch`, Clock, fixtures, and clap `--fixture` → AC-2
- [ ] **T4** Add domain tests for used/remaining, apply_fetch no-nest, risk sort, `secondary_window`, unique `glyph_ascii`, FrozenClock offsets → AC-2
- [ ] **T5** Add Ember Ledger theme and 7-row Fill(1) card grid with header `worst` and one-row footer → AC-3, AC-5
- [ ] **T6** Add TestBackend snapshots at `(80, 48)`, `(80, 24)`, `(120, 24)`, `(140, 24)` → AC-3, AC-5
- [ ] **T7** Add spatial hjkl, detail overlay, JoinSet refresh, sort toggle, and `--fixture error` seed-then-refresh → AC-4
- [ ] **T8** Switch default CLI to the live registry when the first adapter lands; keep `--fixture` replacing the whole board → AC-6
- [ ] **T9** Add Kimi Coding adapter `GET https://api.kimi.com/coding/v1/usages` with a redacted JSON fixture → AC-6
- [ ] **T10** Add z.ai adapter `GET https://api.z.ai/api/monitor/usage/quota/limit` using `ZAI_API_KEY` → AC-7
- [ ] **T11** Add Codex adapter via `codex app-server` `account/rateLimits/read` mapping `usedPercent` and `windowDurationMins` → AC-8
- [ ] **T12** Add Claude adapter `GET https://api.anthropic.com/api/oauth/usage` reading `~/.claude/.credentials.json` read-only → AC-9
- [ ] **T13** Add Grok SuperGrok adapter from `~/.grok/auth.json` billing endpoint → AC-10
- [ ] **T14** Keep Muse `Unsupported` on the 60s timer; add opt-in `OnDemand` `POST /v1/responses` only when flagged → AC-10
- [ ] **T15** Run `cargo test -p token-balance` once and fill the root `Test` command cell → AC-2

## Acceptance criteria

### Must

- [ ] **AC-1** Given nested `crates/AGENTS.md` and `crates/token-balance/AGENTS.md` exist before any crate `Cargo.toml` or `.rs`, when `cargo run -p token-balance --bin token-balance -- --version` and `cargo run -p token-balance --bin tb -- --version` run, then each prints `{argv0-basename} {CARGO_PKG_VERSION}` and exits 0, `tooling/agent-kit/trivial.md` is gone, and root `LICENSE-MIT` plus `LICENSE-APACHE` exist.
      Verify: `cargo run -p token-balance --bin token-balance -- --version`; `cargo run -p token-balance --bin tb -- --version`; `test ! -e tooling/agent-kit/trivial.md`; `test -f LICENSE-MIT && test -f LICENSE-APACHE`; `tooling/agent-kit/check.sh`; `.githooks/check-quality.sh`
- [ ] **AC-2** Given domain tests, when `cargo test -p token-balance` runs, then `from_used_percent(25)` yields remaining 75, `apply_fetch` on `Error { stale: None }` keeps the last Available windows, Codex 18% sorts above Kimi 55% Available, and 15m+60m+weekly hero is 15m with weekly secondary.
      Verify: `cargo test -p token-balance`
- [ ] **AC-3** Given fixture set `mixed`, when TestBackend renders `(80, 48)`, `(80, 24)`, `(120, 24)`, and `(140, 24)`, then every card is 7 rows, Claude hero is 72% remaining, `(80, 24)` footer starts with `1–3 / 6`, and other sizes omit that pager prefix.
      Verify: `cargo test -p token-balance` (TestBackend snapshot tests named in `docs/design/layout.md`)
- [ ] **AC-4** Given 2-col mixed fixtures with selection `codex`, when `j` is pressed, then selection becomes `claude`; `--fixture error` first painted frame shows Codex bars plus footer `stale`; overlay swallows hjkl; Space does not close the overlay.
      Verify: `cargo test -p token-balance` (key-handling tests in `docs/design/pr-plan.md` PR4)
- [ ] **AC-5** Given z.ai `NotConfigured` and Muse `Unsupported` on the mixed board, when a card is rendered, then the card shows a dashed unknown bar and the strings `auth missing` or `unsupported`, and the snapshot text contains no numeric `0%` on those cards.
      Verify: `cargo test -p token-balance` (unsigned/unsupported snapshot assertions)
- [ ] **AC-6** Given no `--fixture` after the Kimi adapter exists, when a Kimi coding key is missing, then the Kimi card is `NotConfigured`; when `--fixture mixed` is passed, then the whole registry is fixtures (no live+fixture mix); Kimi remaining comes from `GET https://api.kimi.com/coding/v1/usages` and not from Moonshot PAYG balance.
      Verify: `cargo test -p token-balance` (Kimi adapter + CLI era tests); `rg -n 'moonshot.ai/v1/users/me/balance' crates/token-balance` exits 1
- [ ] **AC-7** Given `ZAI_API_KEY` set, when z.ai fetches, then remaining uses `GET https://api.z.ai/api/monitor/usage/quota/limit` and treats `percentage` as used; given only `ZHIPUAI_API_KEY`, when z.ai fetches, then the card is `NotConfigured` (no silent `open.bigmodel.cn` fallback).
      Verify: `cargo test -p token-balance` (z.ai adapter tests)
- [ ] **AC-8** Given Codex `account/rateLimits/read` JSON with `usedPercent` and `windowDurationMins`, when the adapter maps windows, then remaining is `100 - usedPercent`, session-class is `duration_mins <= 360`, and a weekly-only payload paints no invented 5h bar.
      Verify: `cargo test -p token-balance` (Codex adapter tests with recorded JSON)
- [ ] **AC-9** Given `~/.claude/.credentials.json` readable, when Claude fetches, then remaining uses `GET https://api.anthropic.com/api/oauth/usage` with header `anthropic-beta: oauth-2025-04-20`; utilization maps through `from_used_percent`; the Admin Usage & Cost API is not called.
      Verify: `cargo test -p token-balance` (Claude adapter tests); `rg -n 'usage-cost-api|/v1/organizations/.*/usage' crates/token-balance/src/adapters` exits 1
- [ ] **AC-10** Given Grok CLI auth, when Grok fetches, then weekly remaining plus extra credits come from the SuperGrok billing path and not xAI prepaid `remaining_balance`; given default Muse, when the 60s timer fires, then Muse is not fetched; given Muse opt-in, when the user presses `r`, then one `POST https://api.meta.ai/v1/responses` may run.
      Verify: `cargo test -p token-balance` (Grok + Muse `RefreshPolicy` tests)

### Should

- [ ] **AC-S1** Given the repo README, when a user opens `README.md`, then it names `token-balance` / `tb` and a `cargo run -p token-balance` command (not only "Agent instructions: AGENTS.md").
      Verify: `rg -n 'token-balance|tb|cargo run' README.md`

## Definition of done

- [ ] Every Must AC is `[x]` with evidence under Evidence
- [ ] Work items that those AC require are `[x]`
- [ ] Applicable gates pass: `cargo test -p token-balance` / `tooling/agent-kit/check.sh` / `.githooks/check-quality.sh` / Lint `n/a` / Typecheck `n/a`
- [ ] No secrets in the file or the change
- [ ] Leftovers filed as a later `GOAL-QUOTA-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pending | |
| AC-2 | pending | |
| AC-3 | pending | |
| AC-4 | pending | |
| AC-5 | pending | |
| AC-6 | pending | |
| AC-7 | pending | |
| AC-8 | pending | |
| AC-9 | pending | |
| AC-10 | pending | |

## Risks

- Undocumented remaining-quota APIs (Claude oauth/usage, Grok billing, Kimi usages, z.ai monitor) can change shape; mitigate with `Error` + recorded fixtures, one adapter per work item
- Muse Everyday meters requests; mitigate with `Unsupported` default and `OnDemand` only
- `Cargo.lock` exceeds QUALITY config size; mitigate with authorized `.qualityignore` in T1
- Nested `AGENTS.md` missing under `crates/` fails agent-kit; mitigate by writing those files before `Cargo.toml`
