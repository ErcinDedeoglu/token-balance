# Product

## Three ledgers (hard rule)

| # | Ledger | What it measures | This product |
| --- | --- | --- | --- |
| **1** | **Plan remaining** | Session / weekly % left + reset + extra credits | **Hero. v1.** |
| 2 | API prepaid $ | Wallet credits (xAI `remaining_balance`, Moonshot PAYG, …) | Secondary. Never the hero. Not a v1 card. |
| 3 | Local consumed tokens | ccusage-style JSONL cost reconstruction | **Out of scope for v1.** Mixing with ledger 1 produces fake “$400 Max” lies. |

`LedgerKind` exists in the domain so a later prepaid card cannot paint as an amber remaining-quota gauge.

## Why now

Closest remaining-quota TUI is **quotas 0.11** (Rust/ratatui): auto-detects Claude, Codex, Kimi, Z.ai, Grok/xAI, MiniMax, Antigravity; statusline + JSON. Study it; **do not fork**. We want a smaller remaining-quota **risk board** and a distinct visual language (Ember Ledger, not Catppuccin; eighth-block fuel gauges, not default `Gauge` widgets).

OpenUsage.sh is the other pole: Go, ~dozens of providers, daemon, SQLite, Prometheus. Muse is a pending concern there. Too much product.

Single-purpose tools (Tokdash, usagebar, grok-credits-tracker, aistat, codex-status-command) prove demand for remaining % but are GUI, one-vendor, or mix spend into the same pane.

## Pain this board removes

- “Will this session survive the next hour?” is a **min remaining window** question, not a token-sum question.
- Unsigned providers currently crash or render **0%** in some tools — that is a lie. We dim them and print a login hint.
- Codex reports **`usedPercent`**. Displaying that number as “remaining” inverts the board. We store both; the card face is remaining.
- A 401 after a good fetch must **keep the last bars** (`stale`), not blank the card.

## Goals (v1)

- Beautiful remaining-quota TUI that is screenshotable on fixture data.
- Distinct **Ember Ledger** theme (named tokens + 16-color fallback).
- Card grid that is a **risk board**: lowest remaining window first, then soonest reset — including stale Error cards that still have windows.
- Domain types and object-safe `Provider` trait compiled and unit-tested so live adapters slot in without rewriting layout.
- Fail one provider without taking down the board (timeout, panic, HTTP error).
- Unsigned / unsupported / error states that never fake 0% and never panic the process.
- Keep agent-kit + quality contract green while establishing the Cargo workspace (including `Cargo.lock` size).

## Non-goals (v1)

- Live HTTP, gRPC-web, or `codex app-server` calls.
- ccusage clone, JSONL historians, model breakdowns, ledger-3 spend analytics on the card face.
- Daemon, SQLite, Prometheus, statusline, `--json` CLI (domain is serde-ready; flag is a later PR).
- Password login, cookie stealers, writing credential files, token refresh that rewrites `~/.claude/.credentials.json` (read-only in live adapters too unless a later PR explicitly opts into the same rotation the official CLI uses).
- Muse dummy-request polling (Everyday meters **requests**; a refresh would burn quota). v1 Muse card is `Unsupported`.
- MiniMax, Antigravity, Copilot, OpenRouter, DeepSeek as v1 cards.
- Prepaid-only cards (xAI API wallet). Extra credits **on a plan card** may appear as one teal tertiary line (including `$12.40` extra-usage remaining — that is ledger-1 extra, not ledger-3 spend).
- Custom fonts / Nerd Fonts requirement.
- `--theme catppuccin|nord|system` (later).
- Forking quotas or OpenUsage.
- Mixing live adapters with fixture providers in one process.
