# Key decisions

Locks. If a later note disagrees with this table, the table wins until a new product decision.

| Decision | Choice | Rationale |
| --- | --- | --- |
| Language | Rust edition 2024, rust-version 1.85 | One binary; no JS/Python runtime |
| TUI crate | ratatui + crossterm + tokio (tokio from PR4) | Layout/color control; quotas is prior art we can study without sharing a look |
| Provider dispatch | Boxed `FetchFuture` + `dyn Provider` | RPITIT is not object-safe; no `async-trait` crate; enum would conflict on every adapter PR |
| Product | Ledger **1** remaining-quota risk board | Mixing spend JSONL produces fake Max-plan dollar lies |
| Visual language | **Ember Ledger** (near-black + **amber** fuel + teal extra) | User locked amber; not steel/cyan; not Catppuccin |
| Short alias | **`tb`** second `[[bin]]` on the same `main.rs` from PR1 | `tb` works as soon as the binary exists |
| Bars | Eighth-block `▏▎▍▌▋▊▉█` + large remaining %; clip to `Rect`; ambiguous width 1 | Claude Code usage-pane pattern; not ratatui `Gauge` |
| Glyphs | Unique ASCII `L X K G Z M` | Claude/Codex both cannot be `C`; CJK-safe 1-col |
| Layout | Default scan surface is the remaining table (`t view:table`); optional 7-row cards via `t` toggle; 1/2/3 cols on the card view at 80/140; vertical scroll | GOAL-SCAN-1: comparison at `MAX_ACCOUNTS` 24; v1 card-grid-only lock reopened |
| Card face | Optional `t` view: 7 rows for every status; at most two window rows; one teal tertiary; dashed unknown bar, never `0%` | Frozen card anatomy so adapters cannot force a rewrite; not the default scan |
| Sort | Available **and** Error-with-stale share remaining-sort; then Error; unsigned sinks | Stale danger is still on fire |
| Hero vs header | Hero = shortest session-class (`duration_mins <= 360`); header `worst` uses min window | Session fuel stays the analog meter; Codex 15m does not lose to weekly |
| Paint view | `effective_available` → `AvailableRef`; `last_available` → boxed snapshot | `Available` is not a `ProviderSnapshot` |
| `--fixture error` | Seed mixed Available, no paint, one refresh, then first frame | `apply_fetch(None, Error)` would drop bars |
| Mockups | Schematic; TestBackend + Fill(1) are the oracle | ASCII drawings will not stay column-accurate in markdown |
| Display vs color | Both from `remaining_percent.round()` as `u8` | 19.6% cannot be danger on the bar and `20%` on the label |
| Interaction | Spatial `hjkl`; overlay swallows movement; Space opens, does not close | Card grid, not a linear list; 2-col `j` from Codex is Claude |
| Footer | One row: keys always; prefix `1–3 / 6` only when `visible_end < n` | `(80, 24)` is the clipped oracle |
| Events | `crossterm::EventStream` in `tokio::select!` | Std `mpsc::recv` would block the clock tick |
| Refresh | In-flight guard; queue one `r`; 10s timeout; Join panic → Error | Hung Codex cannot freeze the spinner forever |
| Stale merge | `apply_fetch` attaches last Available | Adapters return `Error { stale: None }` |
| Fixtures | Offsets + `Clock`; not absolute `resets_at` | Live demo must not print `-Nd` after 2026-09-15 |
| Repo | Cargo workspace, **one** crate `crates/token-balance`, nested AGENTS.md **before Cargo.toml** | Agent-kit + QUALITY gates |
| Root Cargo + license files | Mandated list + Path class `doc` for LICENSE-* + `.qualityignore` `Cargo.lock` | `[no-loose-files]` + `[unclassified-path]` + `[size-hard]` |
| `trivial.md` | Delete in PR1 after Dev/Build filled | `quality/` already sets HAS_PRODUCT |
| Adapters on disk | `src/adapters/` from PR5; serialize PR5→10 | Avoid promote-rule races; `reqwest` once |
| Default CLI | PR1–4 implicit `--fixture mixed`; PR5+ live registry; `--fixture` replaces all | No mixed live+fixture |
| v1 data | Fixture providers only; trait compiled | Screenshot the TUI without network |
| Muse | `Unsupported` in v1; later `OnDemand` last | Polling burns Everyday requests |
| Prepaid xAI | Not a v1 card | Ledger 2; never amber hero |
| Auth | Reuse CLI-stored tokens / env keys; read-only | No password login, no cookie stealers |
| Persistence | In-memory last snapshot | No daemon, no SQLite |
| `--json` | Domain serde-ready; flag later | Avoids a second data model |
| License | `MIT OR Apache-2.0`; `LICENSE-MIT` + `LICENSE-APACHE` at repo root | User decision 2026-09-15 |
| Not forking | Study quotas + OpenUsage; implement our board | Distinct visual language and smaller scope |
| Used vs remaining | Store both; only the two constructors compute the pair | Codex `usedPercent` / Claude `utilization` footgun |
| Design in repo | `docs/design/` split by concern | QUALITY `doc` class is 600 lines / 32KB |

## Resolved questions (2026-09-15)

Do not reopen without a new product decision.

| # | Question | Resolution |
| --- | --- | --- |
| 1 | Ship a short alias **`tb`** in v1, later, or never? | **v1.** Second `[[bin]]` named `tb` on the same `src/main.rs` from PR1. |
| 2 | Ember Ledger **amber hero** vs steel / cyan? | **Keep amber** (`#E8A838`). Optional `--theme` later is unrelated. |
| 3 | License MIT OR Apache-2.0 vs MIT-only? | **`MIT OR Apache-2.0`.** PR1: `license` field + root `LICENSE-MIT` / `LICENSE-APACHE` + QUALITY mandated list + Path class `doc`. |

## References

### This repo

- [`AGENTS.md`](../../AGENTS.md)
- [`docs/QUALITY.md`](../QUALITY.md)
- [`docs/GROWTH.md`](../GROWTH.md)
- [`tooling/agent-kit/check.sh`](../../tooling/agent-kit/check.sh)
- [`memory/lessons/quality/control-files-need-nested-agents.md`](../../memory/lessons/quality/control-files-need-nested-agents.md)

### Competitors (study; do not clone the wrong one)

- [clankercode/quotas](https://github.com/clankercode/quotas)
- [ccusage/ccusage](https://github.com/ccusage/ccusage)
- [janekbaraniewski/openusage](https://github.com/janekbaraniewski/openusage) / [capability matrix](https://openusage.sh/docs/capability-matrix/)
- [JingbiaoMei/tokdash](https://github.com/JingbiaoMei/tokdash)
- [luisleineweber/usagebar](https://github.com/luisleineweber/usagebar)
- [akitaonrails/ai-usagebar](https://github.com/akitaonrails/ai-usagebar)
- [pinkpixel-dev/quota](https://github.com/pinkpixel-dev/quota)

### Provider remaining-quota evidence

- Codex `account/rateLimits/read`, field **`usedPercent`**, example `windowDurationMins: 15`: [OpenAI Codex app-server](https://developers.openai.com/codex/app-server/)
- Claude `GET https://api.anthropic.com/api/oauth/usage` + `anthropic-beta: oauth-2025-04-20` (unofficial)
- Kimi `GET https://api.kimi.com/coding/v1/usages` — [Kimi Code benefits](https://www.kimi.com/en/help/kimi-code/benefits)
- z.ai `GET https://api.z.ai/api/monitor/usage/quota/limit` — [usage policy](https://docs.z.ai/devpack/usage-policy)
- xAI prepaid: [management prepaid balance](https://docs.x.ai/developers/rest-api-reference/management/billing)
- Grok SuperGrok: unofficial `~/.grok/auth.json` + `cli-chat-proxy.grok.com/v1/billing`. Official UI: Settings → Usage
- Muse: no poll API; SSE `response.subscription_usage` — [Muse subscriptions](https://ai.developer.meta.com/docs/muse-code/subscriptions/)
