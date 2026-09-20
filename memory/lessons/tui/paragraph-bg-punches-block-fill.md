# paragraph-bg-punches-block-fill

- **Domain:** tui
- **Date:** 2026-09-20
- **Pattern-Key:** paragraph-bg-punches-block-fill
- **Confidence:** high
- **Evidence:** `crates/token-balance/src/table.rs` `paint_row` / `put`; `table_selected_row_highlights_full_width`
- **Status:** active

## Situation

Selected table rows are height 1, so a 3-row `Block` border is not usable. Filling the row `Rect` with `Block` background, then painting `Paragraph` cells without `.bg(...)`, reset those cells to `Color::Reset` and punched holes through the highlight.

## Decision

Fill the selected row with `theme.border`, then set that same `.bg` on every cell (`put`). Keep percent/extra foreground colors. Rejected: amber `│` side bars; a 3-row selected box.

## Reasoning

Ratatui `Paragraph` applies `Style::default()` background (`Reset`) unless `.bg` is set. Gutters between packed columns keep the `Block` fill; text cells must opt in. `Theme::select()` / `COLORTERM` pick RGB vs DarkGray; tests read `Buffer` cell `.bg`, not glyphs.

## Outcome

`table_selected_row_highlights_full_width` asserts Grok name and extra cells share `theme.border` and Kimi does not. PATH `tb` needs reinstall to show the band.

## Lesson

After filling a ratatui `Rect` with a `Block` background, every overlapping `Paragraph` must set the same `.bg` or it will punch a hole.
