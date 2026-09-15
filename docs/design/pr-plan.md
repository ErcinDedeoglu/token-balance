# PR plan

Incremental, independently reviewable. No live network until PR5. Nested `AGENTS.md` before product files in PR1. **Serialize adapter PRs.** PR6+ reuse `reqwest` from PR5. The PR that adds the first adapter creates `src/adapters/`.

This docs-only landing is **not** PR1. PR1 still creates `crates/`.

## PR 1 — Repo/workspace + version binary

- **Title:** `chore: establish Cargo workspace and token-balance version binary`
- **Files:**
  - `crates/AGENTS.md` (**first file under `crates/`**)
  - `crates/token-balance/AGENTS.md` (**before `Cargo.toml` / `.rs`**)
  - `docs/QUALITY.md` (add `Cargo.toml`, `Cargo.lock`, `LICENSE-MIT`, `LICENSE-APACHE` to mandated root files; Path class `LICENSE-MIT` / `LICENSE-APACHE` → `doc`)
  - `.qualityignore` (authorized `Cargo.lock` ignore + reason comment)
  - `.gitignore` (`/target/`)
  - `LICENSE-MIT`, `LICENSE-APACHE`
  - `Cargo.toml` (workspace, resolver 3, edition 2024, `license = "MIT OR Apache-2.0"`)
  - `Cargo.lock`
  - `crates/token-balance/Cargo.toml` (`license`; two `[[bin]]` entries `token-balance` and `tb`, both `path = "src/main.rs"`; no extra deps)
  - `crates/token-balance/src/main.rs` — print `{argv0-basename} {version}`; **no TUI, no clap**
  - Root `AGENTS.md` command cells: fill `Dev`/`Build` only after those commands have been run once; update cohesion-review sha256
  - **Delete** `tooling/agent-kit/trivial.md` after Dev/Build are filled
  - `memory/daily/YYYY-MM-DD.md`
- **Depends on:** none
- **Done when:** `cargo run -p token-balance --bin token-balance -- --version` and `--bin tb` work. Green `tooling/agent-kit/check.sh` and `.githooks/check-quality.sh`.

## PR 2 — Domain types + fixtures + unit tests (no TUI)

- **Title:** `feat: quota domain types, risk sort, and fixture providers`
- **Files:** `domain.rs`, `providers.rs`, `fixtures.rs`, `main.rs` (clap `--fixture`, still no TUI)
- **Depends on:** PR 1
- **Done when:** `cargo test -p token-balance` run once → fill `Test` command cell. Tests: used/remaining, apply_fetch no-nest, error fixture sorts above healthy Available, unique `glyph_ascii`, FrozenClock offsets, `secondary_window` (15m+60m+weekly → hero 15m, secondary weekly), `effective_available` returns `AvailableRef`.
- **Deps:** `chrono` (`std`,`clock`,`serde`), `serde`, `clap`.

## PR 3 — Ember Ledger + card layout with fixtures (visual PR)

- **Title:** `feat: Ember Ledger theme and remaining-quota card grid`
- **Files:** `theme.rs`, `layout.rs`, `tui.rs` (render only; quit on `q`), `main.rs` (alt screen), TestBackend snapshots, `README.md` one-liner
- **Depends on:** PR 2
- **Oracle:** TestBackend + `layout.rs` Fill(1) / 7-row constraints, not character-matching [mockups.md](mockups.md). Claude 80-col hero is 72%. z.ai / Muse are 7 rows with a dashed unknown bar and **no `0%`**. `(80, 24)` footer is `1–3 / 6   r refresh   …`; other sizes are keys only.
- **Deps:** `ratatui`, `crossterm`, `unicode-width`.

## PR 4 — Interaction

- **Title:** `feat: selection, detail overlay, refresh, and sort toggle`
- **Files:** `tui.rs` (`App::new` seed-then-refresh for `--fixture error`), `main.rs` (tokio `select!` + `EventStream`), `fixtures.rs` (150 ms sleep; `ZERO` under test / FrozenClock)
- **Depends on:** PR 3
- **Tests:** overlay swallows hjkl; Space does not close; **2-col mixed, selection `codex`, `j` → `claude`**; 1-col `h`/`l` no-op; wrap off; snapshots must not sleep.
- **Deps:** `tokio`.

## PR 5 — Live Kimi Coding adapter

- **Title:** `feat: Kimi Coding remaining-quota adapter`
- **Files:** `src/adapters.rs` + `src/adapters/kimi.rs`, registry, redacted JSON fixture, `reqwest` + `serde_json`
- **Depends on:** PR 4
- **Notes:** Creates `src/adapters/`. `GET https://api.kimi.com/coding/v1/usages`. Do **not** call Moonshot PAYG balance. Missing key → `NotConfigured`. **Default CLI becomes live registry**; `--fixture` still replaces the whole board. `reqwest` is added **once** here.

## PR 6 — Live z.ai GLM Coding adapter

- **Title:** `feat: z.ai GLM Coding remaining-quota adapter`
- **Files:** `src/adapters/zai.rs`
- **Depends on:** PR 5
- **Notes:** `GET api.z.ai/api/monitor/usage/quota/limit` with `ZAI_API_KEY`. `percentage` is used. Do not fall back to `ZHIPUAI_API_KEY` / `open.bigmodel.cn`. No list-price `$` on the card.

## PR 7 — Live Codex app-server adapter

- **Title:** `feat: Codex Plus/Pro remaining via app-server rateLimits`
- **Files:** `src/adapters/codex.rs`
- **Depends on:** PR 6
- **Notes:** `account/rateLimits/read`. Classify by `windowDurationMins`. `usedPercent` through `from_used_percent`. Card shows **at most two** windows. Weekly-only accounts: no fake 5h bar. Do not scrape `/status`.

## PR 8 — Live Claude Pro/Max adapter

- **Title:** `feat: Claude Pro/Max remaining via oauth/usage`
- **Files:** `src/adapters/claude.rs`
- **Depends on:** PR 7
- **Notes:** `GET https://api.anthropic.com/api/oauth/usage` with `~/.claude/.credentials.json`. Do **not** call Admin Usage & Cost API. Read-only credentials.

## PR 9 — Live Grok SuperGrok adapter

- **Title:** `feat: SuperGrok remaining via CLI billing endpoint`
- **Files:** `src/adapters/grok.rs`
- **Depends on:** PR 8
- **Notes:** Read `~/.grok/auth.json` (read-only). Weekly % + extra credits. Schema miss → `Error` + `apply_fetch`. **Do not** merge xAI API prepaid into this card.

## PR 10 — Muse last (opt-in, on-demand)

- **Title:** `feat: Muse unsupported-to-on-demand remaining (opt-in)`
- **Files:** `src/adapters/muse.rs`, TUI copy, `RefreshPolicy::OnDemand`
- **Depends on:** PR 9
- **Notes:** Default remains `Unsupported`. Opt-in flag/env required before any `POST /v1/responses`. Each fetch may burn an Everyday request; never 60s. `META_API_KEY` PAYG must not be treated as subscription remaining.

## Follow-ons (not v1)

- `--json` headless (domain already serde-ready)
- `--theme catppuccin|nord|system`
- Unicode glyphs if CI proves width 1
- xAI prepaid as a **teal ledger-2 card** (not hero amber)
- Statusline mode
- MiniMax / Antigravity cards
- Token refresh that writes credential files (only if matching the official CLI)

Each live-adapter PR is mergeable with the others failing independently: unsigned providers stay dim hints. They are **serialized** so `reqwest` and `src/adapters/` land once.
