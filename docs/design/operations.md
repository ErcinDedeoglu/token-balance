# Operations

Threat model is a **local, single-user** TUI. No server, no telemetry, no update-check network in v1 (quotas forks a crates.io check — we will not).

## Security & privacy

| Threat | Severity | Mitigation |
| --- | --- | --- |
| Credential file contents printed in TUI / logs / `--json` | High | Never include tokens in `ProviderSnapshot`. Redact on any debug path. `Debug` impls must not dump keys. |
| Writing/rotating `~/.claude/.credentials.json` or `~/.grok/auth.json` and racing the official CLI | High | v1 fixtures: no files. Live adapters: **read-only** unless a later PR explicitly implements the vendor’s own refresh and documents it. |
| Cookie stealers / password login | High | Out of policy. Auth = env keys + CLI-stored files only. |
| Undocumented endpoint schema change → panic | Medium | `serde` optional fields; map failure → `Error`. No `unwrap` on response bodies. Join panics → `Error`, alt screen stays. |
| Hitting undocumented usage endpoints (ToS) | Medium | Same class of call quotas/OpenUsage already make against **the user’s own account**. Isolate per adapter. Degrade to `Error` / `Unsupported`. Do not scrape HTML. |
| Muse poll burns Everyday requests | High | v1 `Unsupported`. Later `OnDemand` only. Never 60s. Card copy states the cost. |
| Mixing `META_API_KEY` PAYG with subscription | Medium | Adapter must refuse PAYG key for subscription remaining, or label ledger correctly. Prefer Muse CLI auth file. |
| Cross-region z.ai vs Zhipu | Medium | Separate provider ids if both ever ship; never fallback `api.z.ai` → `open.bigmodel.cn`. |
| Secrets in repo / AGENTS.md | High | Existing kit grep + `.gitignore` `.env`. No fixture tokens that look real (`sk-ant-…`). |
| Clipboard copy of raw JSON | Low | Not in v1. If added, strip auth. |
| Hung adapter overlapping the next poll | Medium | In-flight guard + 10s timeout + stale merge. |

## Observability

v1 is a local TUI. stderr logging fights the alt screen.

| Signal | v1 | Later |
| --- | --- | --- |
| Per-card footer | fetch age / `stale` / `auth missing` / `unsupported` / error message (truncated) | unchanged |
| Header | worst remaining, refresh age, spinner | unchanged |
| Tracing | none by default | optional `RUST_LOG` to a file **off** the alt screen; never tokens |
| Metrics | none | no Prometheus |
| Panic | restore terminal in `Drop` / `panic hook`; adapter panics become card `Error` | same |
| Tests | domain + layout + theme + fixture unit tests; PR3 TestBackend snapshots at the four locked sizes | adapter contract tests with recorded JSON **fixtures** (redacted) |

Alerting: N/A (no daemon). The board **is** the alert — danger red `< 20`.

No CI job is required for a hook-enforced local CLI. PR3 updates README from the stub to a one-liner plus `cargo run -p token-balance`.

## Rollout

This is a CLI, not a multi-tenant service.

| Stage | What ships | Rollback |
| --- | --- | --- |
| PR1 | Workspace + version string, no TUI | `git revert`; no users |
| PR2 | Domain + fixtures + tests | revert |
| PR3 | Ember Ledger + cards (screenshot gate) | revert; theme is a struct, not a flag |
| PR4 | Interaction | revert |
| PR5+ | One live adapter per PR, serialized; feature-detect credentials | adapter failure is a card `Error`, not a binary rollback |

**Feature flags:** v1 (PR1–4) is fixtures-only. Live adapters later: compile all, **enable a provider when credentials exist**; otherwise `NotConfigured`. `--fixture` replaces the whole registry. No compile-time feature forest in v1.

**Release:** `cargo install --path crates/token-balance` until crates.io. Pin `Cargo.lock` in the repo (binary application); it is quality-ignored for size, not gitignored.

**Screenshot gate for PR3:** TestBackend ASCII fixtures at `(80, 48)`, `(80, 24)`, `(120, 24)`, `(140, 24)` under `crates/token-balance/src/` (e.g. `layout_80_test.rs`). Visual regression is ASCII, not PNG. Those snapshots plus `Fill(1)` / 7-row math are the **layout oracle**. Unsigned cards in snapshots must be 7 rows with a dashed unknown bar and no `0%`. `(80, 24)` footer prefixes `1–3 / 6`; the other three sizes are keys only.

## Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Undocumented APIs change (Claude oauth/usage, Grok billing, z.ai monitor, Kimi usages) | High | Isolate adapters; `Error` not panic; one provider per PR; pin response fixtures |
| Muse request-metered polling | High | Unsupported in v1; `RefreshPolicy::OnDemand` later; never auto-poll |
| `usedPercent` vs remaining % (Codex, Claude utilization, z.ai `percentage`) | High | `from_used_percent` / `from_remaining_percent`; store both; tests; card face remaining only |
| Codex `primary` is 15m not 5h | High | Classify by `duration_mins`; hero = shortest session-class ≤ 360m |
| Mixing ledgers (ccusage $ / prepaid / plan %) | High | `LedgerKind`; hero bars only for `PlanRemaining`; teal for extra |
| QUALITY `[no-loose-files]` rejects root `Cargo.toml` / LICENSE files | Medium | Update mandated list **and** Path class in the **same commit** as the files |
| QUALITY `[size-hard]` rejects `Cargo.lock` | High | Authorized `.qualityignore` for `Cargo.lock` in PR1; never size-baseline |
| Nested `AGENTS.md` missing under `crates/` | Medium | PR1 first files are the two `AGENTS.md`s, before `Cargo.toml` |
| `trivial.md` leaves commands `n/a` forever | Medium | Delete it in PR1 after Dev/Build are filled |
| Source file exceeds 300/500 lines (`tui.rs`) | Medium | Split by concern (`cards.rs`) when promoting |
| Hung / panicking adapter | Medium | 10s timeout, in-flight guard, `apply_fetch` stale, JoinError → Error |
| Truecolor missing in tmux/SSH | Low | 16-color constructor; danger Red vs hero Yellow; warn = Yellow+bold |
| 139→140 3-col cliff | Low | Titles truncate; bars shrink; 45-col anatomy still fits |
| ToS / account flags on unofficial usage endpoints | Medium | Own-account, read-only, degrade; document; no HTML scrape |
| Fixture TUI looks done; live adapters stall | Low | Trait + registry + serialized PR plan 5+ are explicit; UI frozen |
| ratatui major bump | Low | Pin in `Cargo.lock`; upgrade in its own PR |
