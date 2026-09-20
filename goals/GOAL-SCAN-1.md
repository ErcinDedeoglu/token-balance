---
id: GOAL-SCAN-1
title: Table-first remaining scan
status: done
created: 2026-09-20
source: discussion
---

# GOAL-SCAN-1: Table-first remaining scan

## Intent

A developer running `tb` sees one aligned table row per account by default (5h remaining, weekly remaining, soonest reset, extra) so they can compare remaining and reset across the listed accounts, with the 7-row Ember Ledger card grid one `t` key away.

## Problem

The live board is a risk-sorted 7-row card grid (`docs/design/layout.md`; user screenshot, 13 accounts). The user could not tell which provider sat where, which account had more remaining, or which reset sooner. Face `%` is `hero_window` (shortest session-class) while sort is `min_remaining` (`crates/token-balance/src/windows.rs`), so Kimi can show 100% and rank as 80%. Prepaid wallets have empty `windows` and `unwrap_or(100.0)` into the same group. `MAX_ACCOUNTS` is 24 (`crates/token-balance/src/accounts.rs`). `--fixture mixed` at 80×24 was designed for six 7-row cards; `--fixture multi` already pages.

## Outcome

Default `tb` paint is a remaining comparison table: one row per account, columns 5h / wk / reset / extra, plan rows above a prepaid section. `t` restores today’s 7-row cards. Detail overlay stays. Design SSOT records the card-grid lock as reopened.

## Scope

### In

- Product decision in `docs/design/layout.md`, `docs/design/decisions.md`, and `docs/design/alternatives.md`: table is the default scan surface; 7-row cards are an optional view
- Table paint: one row per account; columns 5h remaining, weekly remaining, soonest reset countdown, extra
- PrepaidWallet section below plan-remaining rows; teal remaining; no amber remaining-% gauge
- Default view is table on launch; `t` toggles table|cards; footer and help list `t`
- TestBackend oracles for default table, `t` card restore, dual-window face vs sort key, prepaid section, unsigned, empty registry
- `j`/`k` move one table row; card view keeps current spatial `hjkl`

### Out

- Do not auto-switch table vs cards by account count
- Do not delete the 7-row card anatomy or change the 80 / 140 column breakpoints for card view
- Do not change `hero_window` / session-class hero on the card face
- Do not add a third `SortMode` (reset); keep `o` as risk|name
- Do not persist view or sort to disk or `accounts.toml`
- Do not add statusline, `--json`, `--theme`, daemon, SQLite, or Prometheus
- Do not paint ledger-3 spend or token counts on the table or card face
- Do not mix live adapters with `--fixture`
- Do not rewrite adapter fetch or remaining mappers
- Do not clone quotas used-% cards or a used-% table
- Do not add a k9s-style split pane
- Do not change `MAX_ACCOUNTS`
- Do not rewrite overlay detail beyond listing `t` on the help overlay

## Context

- Constraints: Ledger-1 remaining-quota board; Ember Ledger amber remaining + teal extra/prepaid; never numeric `0%` for `NotConfigured` / `Unsupported`; QUALITY source split 300 / hard 500 (`tui.rs` is 375 lines — new table paint is a new file, not a dump into `tui.rs`); TestBackend + Fill(1) remains the layout oracle; `cargo test -p token-balance`; nested `AGENTS.md` already exists
- Files / systems: `docs/design/layout.md`; `docs/design/decisions.md`; `docs/design/alternatives.md`; `crates/token-balance/src/{tui,board,cards,windows,overlay,layout,accounts}.rs`; `crates/token-balance/src/tui_test.rs`; `goals/GOAL-QUOTA-1.md` (done card-grid lock this goal reopens)
- Commands: `cargo test -p token-balance`; `cargo build -p token-balance`; Lint `n/a`; Typecheck `n/a`; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`

## Decisions

| Decision | Provenance |
| --- | --- |
| Table is the default scan surface; 7-row cards stay as an explicit `t` toggle | user: picked "Table-first scan (Recommended)" |
| One row per account, not one row per window | user: "which provider where"; recommended option in `/your-call` |
| Columns are 5h remaining, weekly remaining, soonest reset, extra | user: "which has more token and less days to reset"; `/your-call` recommended option |
| PrepaidWallet sits in a section below plan rows | user screenshot mixed wallets; `windows.rs` empty windows `unwrap_or(100.0)` |
| Reject auto density by account count | user: rejected "Auto cards-or-table" |
| Reject card-only repair and leave-locked-grid | user: rejected "Repair cards only" and "Keep locked grid" |
| Reopen the v1 card-grid lock in design SSOT | `docs/design/decisions.md` (lock until a new product decision); this call |
| Launch always starts in table view (no persist) | `/your-call` recommended option; not a disk-config discussion |
| `o` remains risk\|name; no reset sort mode in this goal | current `SortMode` in `domain.rs`; rejected keep-locked-grid was the reset-sort-only path |

## Assumptions

- 80-col table clips the account label before dropping 5h, wk, reset, or extra
- Table 5h cell is the session-class window (`duration_mins <= 360` or `FiveHour`); em dash if none
- Table wk cell is `WindowLabel::Weekly`; em dash if none
- Table reset cell is `format_countdown` of the soonest `resets_at` on that account; prepaid shows `no reset`
- Prepaid remaining uses the existing teal extra line (`$` or `cr`), not a fake percent
- `--fixture mixed` remains the screenshot/oracle set; default paint assertions move to table, card assertions run after `t`

## Open questions

n/a

## Work

- [x] **T1** Record table-first as the default scan surface and 7-row cards as the `t` view in `docs/design/layout.md`, `docs/design/decisions.md`, and `docs/design/alternatives.md` → AC-7
- [x] **T2** Paint a table of one row per account with 5h, weekly, soonest-reset, and extra columns in a new source file (not by growing `tui.rs`) → AC-1, AC-4
- [x] **T3** Render `LedgerKind::PrepaidWallet` rows in a prepaid section below plan rows with teal remaining and no remaining-% gauge → AC-3
- [x] **T4** Default `App` to table view; toggle `t`; show `t view:table` or `t view:cards` on the footer; list `t` on the help overlay; table `j`/`k` move one row → AC-1, AC-2
- [x] **T5** Keep the empty-registry accounts.toml hint and do not draw table data rows when `snapshots` is empty → AC-5
- [x] **T6** Paint `NotConfigured` / `Unsupported` table cells without numeric `0%` → AC-6
- [x] **T7** Add or retarget TestBackend tests for mixed 80×24 table default, `t` 7-row card restore, 100% 5h / 80% wk both visible, prepaid section, unsigned, and empty registry → AC-1, AC-2, AC-3, AC-4, AC-5, AC-6

## Acceptance criteria

### Must

- [x] **AC-1** Given `--fixture mixed` at 80×24 on launch, when the first frame paints, then each account is one table row (not a 7-row bordered card), the row shows 5h remaining, weekly remaining, a reset countdown, and extra when present, and the footer contains `t view:table`.
      Verify: `cargo test -p token-balance --locked table_default_mixed_80x24 -- --nocapture`
- [x] **AC-2** Given AC-1’s mixed app, when the user presses `t`, then every visible vendor card is 7 rows including borders and mixed 2-col Codex\|Grok still holds at width 80–139; when they press `t` again, then the table from AC-1 returns.
      Verify: `cargo test -p token-balance --locked table_toggle_restores_seven_row_cards -- --nocapture`
- [x] **AC-3** Given a board with at least one `LedgerKind::PlanRemaining` snapshot and one `LedgerKind::PrepaidWallet` snapshot, when the table paints, then every prepaid row is below every plan row, prepaid remaining is teal `$` or `cr` with `no reset`, and that prepaid row has no remaining-percent gauge.
      Verify: `cargo test -p token-balance --locked table_prepaid_section_below_plan -- --nocapture`
- [x] **AC-4** Given an Available plan account whose 5h window is 100% remaining and whose weekly window is 80% remaining, when the table paints, then that row contains both `100%` and `80%`.
      Verify: `cargo test -p token-balance --locked table_shows_session_and_weekly_percent -- --nocapture`
- [x] **AC-5** Given zero snapshots (missing accounts file / empty registry), when the board paints in table view, then the body contains `accounts.toml` and does not contain a table header row of 5h/wk cells.
      Verify: `cargo test -p token-balance --locked table_empty_registry_hint -- --nocapture`
- [x] **AC-6** Given `--fixture unsigned` in table view, when z.ai and Muse paint, then those rows do not contain `0%`.
      Verify: `cargo test -p token-balance --locked table_unsigned_never_zero_percent -- --nocapture`
- [x] **AC-7** Given this product decision, when `docs/design/layout.md` and `docs/design/decisions.md` are read, then they name table as the default scan surface and 7-row cards as the optional `t` view (the v1 card-grid-only lock is no longer the scan SSOT).
      Verify: `rg -n "default scan|view:table|optional.*7-row|t toggle" docs/design/layout.md docs/design/decisions.md docs/design/alternatives.md`

### Should

- [x] **AC-S1** Given the help overlay, when `?` is pressed from table view, then the help text includes a `t` line for table|cards.
      Verify: `cargo test -p token-balance --locked help_lists_table_toggle -- --nocapture`
- [x] **AC-S2** Given `--fixture multi` at 80×24 in table view when not every row fits, when the footer paints, then it still prefixes `N–M / total` and stays one row.
      Verify: `cargo test -p token-balance --locked table_multi_pager_one_footer_row -- --nocapture`

## Definition of done

- [x] Every Must AC is `[x]` with evidence under Evidence
- [x] Work items that those AC require are `[x]`
- [x] Applicable gates pass: `cargo test -p token-balance` / `n/a` lint / `n/a` typecheck / `cargo build -p token-balance` / `.githooks/check-quality.sh` / `tooling/agent-kit/check.sh`
- [x] No secrets in the file or the change
- [x] Leftovers filed as a later `GOAL-SCAN-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pass | `cargo test -p token-balance --locked table_default_mixed_80x24` ok; mixed 80×24 dump is one row per account, footer `t view:table`, no `┌` |
| AC-2 | pass | `cargo test -p token-balance --locked table_toggle_restores_seven_row_cards` ok |
| AC-3 | pass | `cargo test -p token-balance --locked table_prepaid_section_below_plan` ok |
| AC-4 | pass | `cargo test -p token-balance --locked table_shows_session_and_weekly_percent` ok |
| AC-5 | pass | `cargo test -p token-balance --locked table_empty_registry_hint` ok |
| AC-6 | pass | `cargo test -p token-balance --locked table_unsigned_never_zero_percent` ok |
| AC-7 | pass | `rg -n "default scan\|view:table\|optional.*7-row\|t toggle"` hits `docs/design/layout.md`, `decisions.md`, `alternatives.md` |

## Risks

- GOAL-QUOTA-1 froze 7-row cards as the product; this goal reopens that lock. Mitigation: card view after `t` must still satisfy the existing seven-row mixed oracles.
- Two paint paths can drift. Mitigation: table reads the same `QuotaWindow` fields; card view keeps `hero_window` / `secondary_window`.
- `tui.rs` (375) and `tui_test.rs` (448) are already over the 300 / 400 split-review lines. Mitigation: new table paint file and `table_test.rs` (or equivalent `*_test.rs`), not growth of those two files.
- 80-col labels plus four columns will clip. Mitigation: clip `display_name` first; keep the four data cells.
