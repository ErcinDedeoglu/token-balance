# Alternatives considered

## 1. Language / TUI stack

| Option | Pros | Cons | Verdict |
| --- | --- | --- | --- |
| **Rust + ratatui + crossterm + tokio** | Static-ish binary; widget/layout/color control; quotas is study-able prior art | Edition 2024 needs rustc 1.85 | **Lock** |
| Go + Bubble Tea | Fine TUIs; OpenUsage already did this | We would be a thinner OpenUsage; less color/layout control for analog gauges | Reject |
| Python + Textual | Pretty CSS-like | Packaging/runtime for an always-on dashboard is poor | Reject |
| Node + Ink | Fast to sketch | Node runtime; not a one-binary tool | Reject |
| Fork `clankercode/quotas` | Remaining-quota already works | We inherit their visual language, provider soup, statusline/JSON scope, and license surface | Reject |

## 2. Provider dispatch

| Option | Pros | Cons | Verdict |
| --- | --- | --- | --- |
| **Boxed `FetchFuture` + `dyn Provider`** | Heterogeneous `Vec<Arc<dyn Provider>>`; adapter PRs only push a new `Arc`; `'static` spawn; no extra crate | Manual `Box::pin` in each `fetch` | **Lock** |
| RPITIT `impl Future` + `dyn Provider` | Pretty signatures | **Does not compile** (not dyn-compatible); implied lifetime is not `'static` | Reject |
| Closed `enum AnyProvider` | No dyn, no boxing | Every adapter PR edits the enum (merge conflicts); TUI match must grow | Reject for v1 |
| `async-trait` | Ergonomic `async fn fetch` | Extra crate; expands to the same boxed future | Reject |

## 3. Theme

| Option | Pros | Cons | Verdict |
| --- | --- | --- | --- |
| **Ember Ledger (custom)** | Distinct; fuel-gauge metaphor; teal separates extra / ledger 2 | New tokens to implement | **Lock** |
| Catppuccin Mocha | Zero design work; users recognize it | Every TUI uses it; we would look like quotas/k9s clones | Reject for v1 default |
| Nord | Cool, readable | Blue-black fights the “near-black not blue-black” brief | Reject default; optional later `--theme` |

## 4. Information architecture

| Option | Pros | Cons | Verdict |
| --- | --- | --- | --- |
| **Risk-sorted card grid** | Answers “who dies first” in one glance | Less density than a table; 80×24 needs scroll | **Lock** |
| quotas-style TUI + statusline + JSON day one | Broader CLI | Scope explosion; statusline wants a cache daemon-ish fork | Later `--json` / statusline only |
| OpenUsage kitchen sink (daemon, SQLite, Prometheus, 36 providers) | History, burn rate | Explicitly not this product | Reject |
| Table of used% | Easy | Hides risk; used-vs-remaining footgun | Reject |
| ccusage blocks view | Familiar | Ledger 3, not remaining | Reject |

## 5. Repo shape

| Option | Pros | Cons | Verdict |
| --- | --- | --- | --- |
| **Workspace + `crates/token-balance`** | Room for a future adapter crate without moving the binary; root stays agent-kit | Root `Cargo.toml` needs QUALITY mandated-list + lockfile ignore | **Lock** |
| Single crate at repo root (`src/` next to AGENTS.md) | Fewer folders | Mixes kit files with product `src/`; GROWTH wants product under a named tree | Reject |
| Many crates on day one (`tb-domain`, `tb-tui`, `tb-kimi`, …) | Pure SoC | Empty forest; violates “single crate in v1” | Reject |
