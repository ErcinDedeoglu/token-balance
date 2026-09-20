# table-reset-skips-session

- **Domain:** tui
- **Date:** 2026-09-20
- **Pattern-Key:** table-reset-skips-session
- **Confidence:** high
- **Evidence:** `crates/token-balance/src/windows.rs` `calendar_reset_window`; `table_shows_session_and_weekly_percent`; live board aylin/kimi 5h clocks
- **Status:** active

## Situation

The table reset column followed `constraint_window` (lowest remaining % of any window). Accounts with a 5h session tighter than weekly painted 2h–4h countdowns. The user scans week/month refill time and does not want the 4h session clock in that column.

## Decision

Reset paints `calendar_reset_window`: lowest remaining among non-session windows (weekly, then monthly/Other). No weekly and no monthly → em dash. Prepaid stays `no reset`. Risk sort still uses `constraint_window` including session.

## Reasoning

5h remaining still lives in the `5h` cell. Reset answers “when does the week or month refill?” Cards and the detail overlay can still show session `resets_at`.

## Outcome

Codex mixed fixture reset is weekly `4d 2h`, not session `1h 12m`. Kimi with 40% session / 80% weekly still shows `5d`.

## Lesson

Do not put session-class countdowns in the table reset column. Use weekly or monthly only.
