# Schematic mockups

**Not** column-accurate. **Not** the PR3 snapshot oracle. Oracle: `layout.rs` numeric constraints + TestBackend at the sizes below. PR3 must not “match the mockups” character-for-character.

Card height in every mockup: **7 rows including borders**, including z.ai / Muse. Inner card width = terminal width − 2 padding, split with 1-col gaps (`Fill(1)` remainder left-to-right): 80-col → 78; 120-col → 59+58; 140-col → 46+45+45.

| Backend | What it proves |
| --- | --- |
| `(80, 48)` | Full 1-col mixed board (all six cards; no scroll) |
| `(80, 24)` | First page only (Codex, Grok, Claude); scroll offset 0 |
| `(120, 24)` | 2-col full mixed board |
| `(140, 24)` | 3-col full mixed board |

Claude hero is **72%** (session), weekly 41% secondary. z.ai / Muse are **7 rows** (status, one hint, dashed unknown bar, blank, footer). No `(no %)` row.

## 80-col — 1 column (`< 90`)

```
 token-balance  14:32  worst Codex 18%   12s ago
┌─ X Codex  [Plus] ──────────────────────────────────────────────────────────┐
│  18%  ███░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       5h   resets in 1h 12m                                                │
│  wk   ████████████████████░░░░░░░░░░░░░░░░  63%   in 4d 2h                 │
│                                                                            │
│  12s ago                                                                   │
└────────────────────────────────────────────────────────────────────────────┘
┌─ G Grok  [SuperGrok] ──────────────────────────────────────────────────────┐
│  27%  █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       wk   resets in 3d 4h                                                 │
│                                                                            │
│  extra  400 cr                                                             │
│  12s ago                                                                   │
└────────────────────────────────────────────────────────────────────────────┘
┌─ L Claude  [Max 20x] ──────────────────────────────────────────────────────┐
│  72%  ██████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       5h   resets in 2h 05m                                                │
│  wk   ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░  41%   in 5d 11h                │
│  extra  $12.40                                                             │
│  12s ago                                                                   │
└────────────────────────────────────────────────────────────────────────────┘
┌─ K Kimi  [Moderato] ───────────────────────────────────────────────────────┐
│  55%  ███████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       5h   resets in 3h 40m                                                │
│  wk   ████████████████████████░░░░░░░░░░░░  88%   in 6d 1h                 │
│                                                                            │
│  12s ago                                                                   │
└────────────────────────────────────────────────────────────────────────────┘
┌─ Z z.ai  [GLM Coding] ─────────────────────────────────────────────────────┐
│  auth missing                                                              │
│  export ZAI_API_KEY                                                        │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│                                                                            │
│  auth missing                                                              │
└────────────────────────────────────────────────────────────────────────────┘
┌─ M Muse  [Everyday] ───────────────────────────────────────────────────────┐
│  unsupported                                                               │
│  no remaining API; polling burns Everyday requests                         │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│                                                                            │
│  unsupported                                                               │
└────────────────────────────────────────────────────────────────────────────┘
 r refresh   o sort:risk   ? help   q quit
```

`(80, 24)` first page is the header + Codex + Grok + Claude + footer (3×7+2 = 23). **Clipped footer:**

```
 1–3 / 6   r refresh   o sort:risk   ? help   q quit
```

`(80, 48)` / 120-col / 140-col full boards omit `1–3 / 6` and keep keys only.

## 120-col — 2 columns

Fill(1) remainder → cards **59** and **58**. Full mixed board: 3 rows × 7 + header 1 + footer 1 = **23**.

```
 token-balance  14:32  worst Codex 18%   12s ago
┌─ X Codex  [Plus] ─────────────────────────────┐ ┌─ G Grok  [SuperGrok] ────────────────────────────┐
│  18%  ███░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │ │  27%  █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       5h   resets in 1h 12m                  │ │       wk   resets in 3d 4h                     │
│  wk   ████████████░░  63%  4d 2h             │ │                                                │
│                                              │ │  extra  400 cr                                 │
│  12s ago                                     │ │  12s ago                                       │
└──────────────────────────────────────────────┘ └────────────────────────────────────────────────┘
┌─ L Claude  [Max 20x] ─────────────────────────┐ ┌─ K Kimi  [Moderato] ────────────────────────────┐
│  72%  ██████████████░░░░░░░░░░░░░░░░░░░░░░░  │ │  55%  ███████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│       5h   resets in 2h 05m                  │ │       5h   resets in 3h 40m                    │
│  wk   ████████░░░░░░  41%  5d 11h            │ │  wk   ████████████████░░  88%  6d 1h           │
│  extra  $12.40                               │ │                                                │
│  12s ago                                     │ │  12s ago                                       │
└──────────────────────────────────────────────┘ └────────────────────────────────────────────────┘
┌─ Z z.ai  [GLM Coding] ────────────────────────┐ ┌─ M Muse  [Everyday] ────────────────────────────┐
│  auth missing                                │ │  unsupported                                   │
│  export ZAI_API_KEY                          │ │  no remaining API; polling burns Everyday req  │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│                                              │ │                                                │
│  auth missing                                │ │  unsupported                                   │
└──────────────────────────────────────────────┘ └────────────────────────────────────────────────┘
 r refresh   o sort:risk   ? help   q quit
```

## 140-col — 3 columns

Fill(1) remainder → cards **46 / 45 / 45**. Unsigned anatomy is the **same 7 rows**.

```
 token-balance  14:32  worst Codex 18%   12s ago
┌─ X Codex  [Plus] ─────────────────────┐ ┌─ G Grok  [SuperGrok] ──────────────────┐ ┌─ L Claude  [Max 20x] ─────────────────┐
│  18%  ███░░░░░░░░░░░░░░░░░░░░░░░░░░  │ │  27%  █████░░░░░░░░░░░░░░░░░░░░░░░░░  │ │  72%  ██████████████░░░░░░░░░░░░░░░░  │
│       5h   in 1h 12m                 │ │       wk   in 3d 4h                   │ │       5h   in 2h 05m                 │
│  wk   ████████░░  63%  4d 2h         │ │                                       │ │  wk   ████░░░░  41%  5d 11h         │
│                                      │ │  extra  400 cr                        │ │  extra  $12.40                       │
│  12s ago                             │ │  12s ago                              │ │  12s ago                             │
└──────────────────────────────────────┘ └───────────────────────────────────────┘ └──────────────────────────────────────┘
┌─ K Kimi  [Moderato] ──────────────────┐ ┌─ Z z.ai  [GLM Coding] ─────────────────┐ ┌─ M Muse  [Everyday] ──────────────────┐
│  55%  ███████████░░░░░░░░░░░░░░░░░░  │ │  auth missing                         │ │  unsupported                          │
│       5h   in 3h 40m                 │ │  export ZAI_API_KEY                   │ │  no remaining API; polling burns     │
│  wk   ████████████░░  88%  6d        │ │  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│                                      │ │                                       │ │                                      │
│  12s ago                             │ │  auth missing                         │ │  unsupported                          │
└──────────────────────────────────────┘ └───────────────────────────────────────┘ └──────────────────────────────────────┘
 r refresh   o sort:risk   ? help   q quit
```

z.ai is `NotConfigured`. Muse is `Unsupported`. Both sit at the bottom. Extra hint copy lives in the overlay.

## Detail overlay (~60-col centered)

```
┌──────── Codex  detail ─────────────────────────────────┐
│ plan          Plus                                     │
│ ledger        plan remaining                           │
│                                                        │
│ 5h            used 82.0%   remaining 18.0%             │
│               duration 300m                            │
│               resets 2026-09-15 15:44 UTC (in 1h 12m)  │
│ weekly        used 37.0%   remaining 63.0%             │
│               resets 2026-09-19 16:32 UTC (in 4d 2h)   │
│ extra         n/a                                      │
│ fetched       12s ago                                  │
│ docs          https://developers.openai.com/codex/     │
│                                                        │
│ enter/esc/q close                                      │
└────────────────────────────────────────────────────────┘
```

Overlay does not replace the grid. Dim the board behind it. `hjkl` / arrows do nothing until it closes. Space is ignored while open.

## Help overlay

```
┌ keys ──────────────────────────────────────────────────┐
│ h/l  arrows       move in row (clamp, no wrap)         │
│ j/k  arrows       move by column count (no wrap)       │
│ enter/space       open detail                          │
│ enter/esc/q       close overlay                        │
│ r                 refresh all                          │
│ o                 sort risk | name                     │
│ ?                 this help                            │
│ q / esc           close overlay, else quit             │
└────────────────────────────────────────────────────────┘
```
