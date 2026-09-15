# token-balance design

| Field | Value |
| --- | --- |
| Title | remaining AI coding-plan quota TUI |
| Status | **Approved** (2026-09-15) |
| Binary | `token-balance` and `tb` (same `main.rs`) |
| License | MIT OR Apache-2.0 |
| Stack | Rust 2024 + ratatui (not implemented yet) |

This folder is the product design. QUALITY class `doc` is 600 lines / 32KB, so the approved document is split **by concern**, not by line-count suffixes. Do not reassemble it into one file.

v1 is a screenshotable TUI on **fixture** data. Live adapters are later PRs against the frozen trait and card anatomy. Do not start `crates/` until [pr-plan.md](pr-plan.md) PR1.

## Product in one paragraph

Developers on several AI coding plans cannot see **how much quota is left** without hopping through `/status` and vendor consoles. `ccusage` reconstructs historical spend; OpenUsage is a kitchen-sink warehouse. Mixing those ledgers produces fake “$400 Max plan cost” headlines.

**token-balance** (`tb`) is a remaining-quota **risk board**: session and weekly windows, reset countdowns, extra credits — one card per provider, sorted by who runs out first. Theme is **Ember Ledger** (near-black, amber fuel, teal extra).

## Files

| File | Concern |
| --- | --- |
| [product.md](product.md) | Ledgers, goals, non-goals |
| [architecture.md](architecture.md) | Process, mermaid, language, crates, runtime loop |
| [repo.md](repo.md) | Workspace layout, QUALITY gates for PR1 |
| [theme.md](theme.md) | Ember Ledger tokens |
| [layout.md](layout.md) | Card grid, keys, sort, refresh |
| [mockups.md](mockups.md) | Schematic ASCII (not the PR3 oracle) |
| [domain.md](domain.md) | Types, `apply_fetch`, fixtures |
| [cli.md](cli.md) | Flags and era table |
| [providers.md](providers.md) | `Provider` trait and adapter contracts |
| [operations.md](operations.md) | Security, observability, rollout, risks |
| [alternatives.md](alternatives.md) | Rejected stacks, themes, repo shapes |
| [decisions.md](decisions.md) | Key decisions, resolved questions, references |
| [pr-plan.md](pr-plan.md) | PR1–PR10 |

## Implement from

1. [pr-plan.md](pr-plan.md) — order
2. [repo.md](repo.md) — PR1 gates
3. [domain.md](domain.md) + [theme.md](theme.md) + [layout.md](layout.md) — PR2–4
4. [providers.md](providers.md) — PR5+

[decisions.md](decisions.md) is the lock table. If a later note disagrees with it, the table wins until a new product decision.
