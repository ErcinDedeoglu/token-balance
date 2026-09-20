# table-default-retargets-card-oracles

- **Domain:** tui
- **Date:** 2026-09-20
- **Pattern-Key:** table-default-retargets-card-oracles
- **Confidence:** high
- **Evidence:** `crates/token-balance/src/tui.rs` `view: ScanView::Table`; `crates/token-balance/src/tui_test.rs` `show_cards`; `crates/token-balance/src/table_test.rs`
- **Status:** active

## Situation

GOAL-SCAN-1 made the remaining table the default scan surface. Existing TestBackend tests in `tui_test.rs` still asserted 7-row cards on the first `App.draw`.

## Decision

Keep card oracles on the `t` side (`show_cards` then draw). Put table oracles in `table_test.rs` driving the same `App.draw`. Rejected: changing default only in tests, or deleting 7-row mixed oracles.

## Reasoning

`docs/design/layout.md` still freezes 7-row card anatomy for the optional view. `tui.rs` / `tui_test.rs` sit at split-review with hash-bound cohesion reviews; growing those files without a hash update fails `.githooks/check-quality.sh`.

## Outcome

`table_default_mixed_80x24` and `table_toggle_restores_seven_row_cards` pass. `mixed_snapshots_four_sizes` still passes after `t`.

## Lesson

When the default scan view changes, retarget first-paint TestBackend tests to the new default and keep old anatomy tests behind the toggle key. Update cohesion-review sha256 in the same change as `tui.rs` / `tui_test.rs`.
