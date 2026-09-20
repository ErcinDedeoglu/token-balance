---
id: GOAL-SCAN-2
title: Pack table columns and show monthly remaining
status: done
created: 2026-09-20
source: discussion
---

# GOAL-SCAN-2: Pack table columns and show monthly remaining

## Intent

A developer running table-view `tb` reads 5h, weekly, monthly, reset, and extra in a packed group immediately after the account name so monthly-only plans still show remaining percent and the eye does not cross a wide blank to match a name to a number.

## Problem

After GOAL-SCAN-1, live `tb` is a table (user screenshot). `col_widths` in `crates/token-balance/src/table.rs` gives every leftover column to the account name, so on a wide terminal the percents sit on the far right. The 5h cell is session-class only and the wk cell is `Weekly` only, so Claude `10%` mo, Kiro `83%` mo, and Copilot `93%` mo from the card face become dashes; Claude’s row shows `$203.79` extra without the 10% that ranked it.

## Outcome

Table rows keep glyph + name, then 5h, wk, mo, reset, extra with one-column gutters and leftover blank to the right of extra. A monthly `Other` window remaining percent is visible in `mo`. Card view after `t` is unchanged.

## Scope

### In

- Pack table cells so leftover width is after extra, not between name and 5h
- Paint monthly remaining (`WindowLabel::Other` / non-session non-weekly) in a `mo` column
- Keep 5h as session-class remaining and wk as `Weekly` remaining
- TestBackend tests at 80 and ≥120 width for packing, monthly 10% plus extra $, monthly-only percent, and 5h+wk still both visible
- `docs/design/layout.md` table column list includes packed `mo`

### Out

- Do not put monthly remaining in the 5h cell
- Do not restore plan pills (`[Everyday]`, `[max]`, …) on the table row
- Do not change 7-row card anatomy, card `hero_window`, or the 80 / 140 card breakpoints
- Do not change `t` toggle, default table view, or `o` risk|name
- Do not persist view or sort
- Do not rewrite adapter fetch or remaining mappers
- Do not change prepaid section order, teal `$`/`cr`, or `no reset`
- Do not add a third `SortMode`
- Do not add statusline, `--json`, `--theme`, daemon, SQLite, or Prometheus
- Do not paint ledger-3 spend as remaining percent

## Context

- Constraints: Ember Ledger; never numeric `0%` for `NotConfigured` / `Unsupported`; QUALITY source split 300 / hard 500 (`table.rs` is 296 lines — stay in that file or extract by concern, do not dump into `tui.rs`); TestBackend is the layout oracle; clip name before dropping data columns; `cargo install --path crates/token-balance --locked` is how PATH `tb` picks up the binary
- Files / systems: `crates/token-balance/src/table.rs`; `crates/token-balance/src/table_test.rs`; `crates/token-balance/src/windows.rs`; `docs/design/layout.md`; `goals/GOAL-SCAN-1.md`
- Commands: `cargo test -p token-balance`; `cargo build -p token-balance`; Lint `n/a`; Typecheck `n/a`; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`; `cargo install --path crates/token-balance --locked`

## Decisions

| Decision | Provenance |
| --- | --- |
| Pack 5h/wk/reset/extra against the name; leftover blank is right of extra | user screenshot after SCAN-1; agent: "The data group should sit right after the names" |
| Add a `mo` remaining column for non-session non-weekly windows | user live cards showed `mo` 10%/83%/93%; SCAN-1 5h is session-class only (`goals/GOAL-SCAN-1.md` Assumptions) |
| Do not write monthly percent into the 5h cell | SCAN-1: 5h cell is session-class or em dash |
| Do not restore plan pills in this goal | agent: "Maybe OK"; user did not ask to bring pills back |
| PATH `tb` is updated with `cargo install --path crates/token-balance --locked` after the change | user: "did you build it?!" when `~/.cargo/bin/tb` was still the Sep 19 binary |

## Assumptions

- Live Claude/Kiro/Copilot monthly remaining is a `QuotaWindow` with `WindowLabel::Other` (card caption `mo`), not only `ExtraCredits`
- If several Other windows exist, `mo` shows the one with minimum remaining percent
- Name column width is the clipped glyph+label (plus a small pad), not `area.width - data`

## Open questions

n/a

## Work

- [x] **T1** Record packed columns and the `mo` remaining column in `docs/design/layout.md` → AC-6
- [x] **T2** Change table `col_widths` / row split so leftover columns sit after extra, with a one-column gutter after the account name → AC-1
- [x] **T3** Paint `mo` remaining from the non-session non-weekly window; leave 5h and wk as session-class and `Weekly` → AC-2, AC-3, AC-4
- [x] **T4** Add or extend TestBackend tests for 120-col packing, monthly 10% plus extra $, monthly-only 83%, and 5h 100% / wk 80% still both on the row → AC-1, AC-2, AC-3, AC-4
- [x] **T5** Keep empty-registry (no 5h/wk/mo header) and unsigned rows without numeric `0%` → AC-5
- [x] **T6** Run `cargo install --path crates/token-balance --locked` after tests so PATH `tb` matches the repo → AC-1

## Acceptance criteria

### Must

- [x] **AC-1** Given `--fixture mixed` painted at width 120, when the table header and the Codex row are read, then `5h` starts within two columns after the account-name cell, leftover blank cells (if any) are to the right of `extra`, and the Codex row still contains `18%`, `63%`, and `1h 12m`.
      Verify: `cargo test -p token-balance --locked table_pack_columns_120 -- --nocapture`
- [x] **AC-2** Given an Available plan account whose only quota window is `WindowLabel::Other` at 10% remaining and extra `$203.79`, when the table paints, then that row contains `10%` and `$203.79`, and the 5h and wk cells are em dashes.
      Verify: `cargo test -p token-balance --locked table_monthly_percent_and_extra -- --nocapture`
- [x] **AC-3** Given an Available plan account whose only quota window is `WindowLabel::Other` at 83% remaining and no extra, when the table paints, then that row contains `83%` and the 5h and wk cells are em dashes.
      Verify: `cargo test -p token-balance --locked table_monthly_only_percent -- --nocapture`
- [x] **AC-4** Given an Available plan account whose 5h window is 100% remaining and whose weekly window is 80% remaining and who has no Other window, when the table paints, then that row contains both `100%` and `80%` and the `mo` cell is an em dash.
      Verify: `cargo test -p token-balance --locked table_shows_session_and_weekly_percent -- --nocapture`
- [x] **AC-5** Given zero snapshots, when table view paints, then the body contains `accounts.toml` and does not contain a header of `5h`/`wk`/`mo`; given `--fixture unsigned`, z.ai and Muse table rows still do not contain `0%`.
      Verify: `cargo test -p token-balance --locked table_empty_registry_hint -- --nocapture`
- [x] **AC-6** Given this product decision, when `docs/design/layout.md` is read, then the default table lists packed columns including `mo` remaining and leftover width after extra.
      Verify: `rg -n "mo remaining|after extra|packed" docs/design/layout.md`

### Should

- [x] **AC-S1** Given the change is on `main` (or the working tree), when `cargo install --path crates/token-balance --locked` finishes, then `~/.cargo/bin/tb` is newer than the pre-install mtime and a live table footer still contains `t view:table`.
      Verify: `ls -l ~/.cargo/bin/tb` after install; live `tb` footer contains `t view:table`
- [x] **AC-S2** Given `--fixture mixed` at width 80, when the table paints, then data columns 5h, wk, mo, reset, extra are still present (name clips first) and leftover blank is to the right of extra.
      Verify: `cargo test -p token-balance --locked table_pack_columns_80 -- --nocapture`

## Definition of done

- [x] Every Must AC is `[x]` with evidence under Evidence
- [x] Work items that those AC require are `[x]`
- [x] Applicable gates pass: `cargo test -p token-balance` / `n/a` lint / `n/a` typecheck / `cargo build -p token-balance` / `.githooks/check-quality.sh` / `tooling/agent-kit/check.sh`
- [x] No secrets in the file or the change
- [x] Leftovers filed as a later `GOAL-SCAN-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pass | `cargo test -p token-balance --locked table_pack_columns_120` ok |
| AC-2 | pass | `cargo test -p token-balance --locked table_monthly_percent_and_extra` ok |
| AC-3 | pass | `cargo test -p token-balance --locked table_monthly_only_percent` ok |
| AC-4 | pass | `cargo test -p token-balance --locked table_shows_session_and_weekly_percent` ok |
| AC-5 | pass | `cargo test -p token-balance --locked table_empty_registry_hint` ok; unsigned `table_unsigned_never_zero_percent` ok |
| AC-6 | pass | `rg -n "mo remaining|after extra|packed" docs/design/layout.md` hits lines 5 and 8 |

## Risks

- 80-col width is tighter with a sixth cell (`mo`). Mitigation: clip the account name before dropping 5h/wk/mo/reset/extra.
- `table.rs` is 296 lines (split-review 300). Mitigation: keep the change in `table.rs` under 300 or extract column layout by concern; do not grow `tui.rs`.
- PATH `tb` can stay stale after a green test run. Mitigation: T6 / AC-S1 install.
