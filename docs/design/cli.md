# CLI

No HTTP server. The product **is** a CLI.

```text
token-balance [--fixture mixed|unsigned|danger|error]
tb [--fixture mixed|unsigned|danger|error]   # identical; same main.rs
token-balance --version
tb --version
```

| Era | No `--fixture` | `--fixture <set>` |
| --- | --- | --- |
| PR1–PR4 (no live adapters) | Implicit fixture `mixed` | That set, whole registry |
| PR5+ (first live adapter) | **Live registry.** Missing creds → `NotConfigured`. Muse stays `Unsupported` until PR10. | **Replaces the whole registry** with fixtures. No mixed live+fixture in v1. |

`--fixture` is optional in the synopsis; v1 (PR1–4) defaults to `mixed` because that is the only registry. After PR5, omitting it is the live path. Never merge a live Kimi card with a fixture Claude card.

| Flag | v1 (PR1–4) | PR5+ |
| --- | --- | --- |
| (none) | TUI, fixture `mixed` | Live registry |
| `--fixture` | Select fixture set (default mixed if omitted) | Replace registry with fixtures (CI / screenshots) |
| `--json` | **not shipped** | print `Vec<ProviderSnapshot>` |
| `--theme` | **not shipped** | `catppuccin\|nord\|system` |
| `--ascii` | **not shipped** (glyphs already ASCII) | optional unicode ornaments off |
| `--once` | **not shipped** | fetch + print + exit |

Exit codes: `0` clean quit; `1` terminal setup failure; never non-zero because a provider is unsigned.

PR1 does not parse flags: print `{argv0-basename} {version}` on stdout (`tb 0.1.0` when invoked as `tb`). Clap arrives in PR2 with `--fixture`.
