# Theme — Ember Ledger (locked)

Do **not** default to Catppuccin Mocha. The metaphor is an analog **fuel gauge** on a near-black ledger, not a pastel palette.

Canvas is near-black, **not** blue-black. Hero remaining is **warm amber** (`#E8A838`) — **locked**; do not switch the default to steel or cyan. Extra credits / prepaid are **teal** so ledger 2 cannot be mistaken for ledger 1. Typography is the terminal’s default font. Bars use eighth-blocks `▏▎▍▌▋▊▉█` (Claude Code usage-pane pattern) plus a large numeric remaining %.

`█` / `▏`…`▉` are East-Asian-Width Ambiguous. Lock **`unicode-width` ambiguous = 1** (same as ratatui’s usual count) and **clip the bar string to the ratatui `Rect` width** so a double-width `█` cannot wrap the card.

## Token table

| Role | Hex | When | 16-color | ANSI |
| --- | --- | --- | --- | --- |
| `bg` canvas | `#0B0C0F` | Screen | Black | 0 |
| `surface` card fill | `#14161A` | Card body | Black | 0 |
| `border` | `#2A2E35` | Idle card | DarkGray | 8 |
| `border_selected` | `#E8A838` | Focus | Yellow | 3 |
| `label` | `#9AA3AD` | Captions, footer keys | Gray | 7 |
| `text` | `#E6E8EB` | Primary copy | White | 15 |
| `dim` | `#6B7280` | Unsigned / unsupported | DarkGray | 8 |
| `hero` remaining ≥ 50 | `#E8A838` | Safe fuel | Yellow (normal) | 3 |
| `warn` remaining 20–49 | `#F0C14A` | Caution | Yellow **bold** (16-color Yellow vs LightYellow is weak) | 3 + bold |
| `danger` remaining < 20 | `#E24A3B` | Risk | Red | 1 |
| `extra` credits / prepaid | `#3DCEC2` | Ledger-1 extra / ledger-2 accent | Cyan | 6 |
| `error` / stale | `#B4554A` | Muted red | LightRed | 9 |
| `unknown` dashed | `#6B7280` | No window | DarkGray | 8 |
| `plan_pill_bg` | `#2A2E35` | Same value as `border` | DarkGray | 8 |
| `plan_pill_fg` | `#E8A838` | Same value as `hero` | Yellow | 3 |

Thresholds are on the **display integer**, never on a raw `f32` that disagrees with the label:

```text
display_pct = remaining_percent.round().clamp(0.0, 100.0) as u8
display_pct >= 50  → hero
display_pct >= 20  → warn
else               → danger
```

The large `%` text uses the same `display_pct`. 19.4 → `19%` danger; 19.5 → `20%` warn.

Unsigned / unsupported cards: `dim` + `unknown` dashed bar **without a numeric 0%**. Error-with-stale: last bars at full color, footer `stale` in `error`.

16-color fallback is selected when truecolor is unavailable (`TERM`/`COLORTERM` / crossterm capability). Same `Theme` struct, different constructor. SSH/tmux must still read risk vs safe (red vs yellow). Warn vs hero on 16-color is **bold**, not a second hue we cannot rely on.

v1 ships only Ember Ledger with the **amber** hero. Optional `--theme catppuccin|nord|system` is a later PR and does not reopen the amber default.

```rust
#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub border_selected: Color,
    pub label: Color,
    pub text: Color,
    pub dim: Color,
    pub hero: Color,
    pub warn: Color,
    pub danger: Color,
    pub extra: Color,
    pub error: Color,
    pub unknown: Color,
    /// Same RGB as `border` / `hero`; separate fields so the pill can change later.
    pub plan_pill_bg: Color,
    pub plan_pill_fg: Color,
}

impl Theme {
    pub fn ember_ledger() -> Self { /* truecolor hex via Color::Rgb */ }
    pub fn ember_ledger_ansi16() -> Self { /* mapped table; warn = Yellow + use bold in render */ }
    pub fn remaining_color(self, display_pct: u8) -> Color { /* integer thresholds */ }
}

/// Eighth-block bar. `frac` is remaining in 0.0..=1.0. Caller clips to area width.
pub fn eighth_bar(frac: f32, width: u16) -> String { /* ▏▎▍▌▋▊▉█ + empty ░ */ }

pub fn display_pct(remaining_percent: f32) -> u8 {
    remaining_percent.round().clamp(0.0, 100.0) as u8
}
```

Do **not** use ratatui’s `Gauge` widget for the hero. That is quotas’ visual language. Ours is a large `%` + eighth-block strip.
