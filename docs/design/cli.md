# CLI

No HTTP server. The product **is** a CLI.

```text
token-balance [--fixture mixed|unsigned|danger|error|multi]
tb [--fixture mixed|unsigned|danger|error|multi]
token-balance init
tb init
token-balance --version
tb --version
```

| Era | No `--fixture` | `--fixture <set>` |
| --- | --- | --- |
| PR1–PR4 (no live adapters) | Implicit fixture `mixed` | That set, whole registry |
| PR5+ live adapters | **Live registry = `[[account]]` rows in `{HOME}/.config/token-balance/accounts.toml` only.** Missing file → empty board (path hint). Invalid file (second Codex, inline key) → stderr + exit 1, no board. Unlisted vendors are absent. `r` re-reads the file. `tb init` writes a commented template. | **Replaces the whole registry** with fixtures. Does not read the accounts file. No mixed live+fixture. |

`--fixture` is optional in the synopsis; v1 (PR1–4) defaults to `mixed` because that is the only registry. After PR5, omitting it is the live path. Never merge a live Kimi card with a fixture Claude card.

| Flag | v1 (PR1–4) | PR5+ |
| --- | --- | --- |
| (none) | TUI, fixture `mixed` | Live registry from accounts.toml |
| `--fixture` | Select fixture set (default mixed if omitted) | Replace registry with fixtures (CI / screenshots); `multi` is >6 account cards |
| `init` | n/a | Write commented `{HOME}/.config/token-balance/accounts.toml` |
| `--muse-on-demand` | n/a | Fixture Muse only. A Muse **row in accounts.toml** already fetches on launch/`r` (not the 60s timer) and burns an Everyday request. |
| `--json` | **not shipped** | print `Vec<ProviderSnapshot>` |
| `--theme` | **not shipped** | `catppuccin\|nord\|system` |
| `--ascii` | **not shipped** (glyphs already ASCII) | optional unicode ornaments off |
| `--once` | **not shipped** | fetch + print + exit |

Exit codes: `0` clean quit or `init`; `1` terminal setup failure **or accounts.toml load error**; never non-zero because a listed provider is unsigned.

PR1 does not parse flags: print `{argv0-basename} {version}` on stdout (`tb 0.1.0` when invoked as `tb`). Clap arrives in PR2 with `--fixture`.
