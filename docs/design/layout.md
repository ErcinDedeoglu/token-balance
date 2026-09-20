# Layout / placement

This is a **risk board**, not a data warehouse. Schematic drawings: [mockups.md](mockups.md). Layout oracle is TestBackend + `layout.rs`, not those drawings.

The **default scan surface** is a remaining comparison table (`t view:table`): one row per account, columns 5h remaining / weekly remaining / soonest reset / extra. Plan-remaining rows sit above `PrepaidWallet` rows. 7-row Ember Ledger cards are the optional 7-row t toggle view (`t view:cards`). Launch always starts in table (not persisted).

1. **Header strip:** app name `token-balance`, local clock `HH:MM`, **worst remaining** (`Codex 18%`), last refresh age (`12s ago`), spinner **only while fetching**. **1 row when `area.width >= 80`** (truncate age before wrapping). **2 rows when width < 80** (name+clock on row 1; worst+age+spinner on row 2). Recommended minimum terminal: **80×24**.
2. **Table (default):** one row per **account** (not per vendor). Glyph + unique label; 5h cell is the session-class window (`duration_mins <= 360` or `FiveHour`) or em dash; wk cell is `Weekly` or em dash; reset is soonest `resets_at` countdown (prepaid: `no reset`); extra is teal remaining. Prepaid rows have teal `$` / `cr`, no remaining-percent gauge. Unsigned / unsupported never numeric `0%`. Zero accounts: hint naming `.config/token-balance/accounts.toml` (no 5h/wk header). Table `j`/`k` move one row; `h`/`l` no-op. Sort by **risk** (below); prepaid sinks after plan inside the available group.
2b. **Card grid (optional `t` view):** one card per **account**. Title is vendor glyph + unique label. Same sort. Disabled / unsigned / unsupported sink to the bottom, dimmed, never fake 0%.
3. **Responsive columns** (1-col padding on each side; gap 1 col between cards). Remainder columns from `inner % cols` go **left to right** via `Layout::horizontal` of `Constraint::Fill(1)` repeated `cols` times:
   - `< 80` cols → 1 column
   - `80–139` → 2 columns (`--fixture mixed` at 80×24 still shows all six vendor cards, no pager; `--fixture multi` with n>6 pages)
   - `≥ 140` → 3 columns
   - **139→140 cliff:** a 2-col card at 139 is ~68 inner cols; at 140 a 3-col card is ~45. Titles truncate (`unicode-width`); bars shrink. Keep the 140 breakpoint (3-col is for wide terminals, not for denser 2-col).
4. **Card anatomy** (fixed — this is the product). **Card height is always 7 rows including borders** (title + 5 inner + bottom) for **every** status, including `NotConfigured` and `Unsupported`. 2-col/3-col rows align. Unused inner rows are blank, not omitted.

   **Available / Error-with-stale** inner rows (5):
   1. **Hero** — shortest session-class window: large `%` + eighth-block bar
   2. Hero caption — `5h   resets in 1h 12m` (or `15m` / `wk`)
   3. **Secondary** — at most one more window, or blank
   4. **Tertiary** — extra credits in **teal** (`extra  $12.40`, `400 cr`), or blank
   5. Footer — fetch age or `stale`

   **NotConfigured / Unsupported** inner rows (same 5 slots; never taller):
   1. Status word — `auth missing` / `unsupported` (no numeric %)
   2. Hint line — `export ZAI_API_KEY` / `no remaining API; polling burns Everyday requests` (wrap onto the next inner row when the column is narrow)
   3. **Dashed `unknown` bar** (theme `unknown`, not a remaining gauge, **never `0%`**)
   4. Blank
   5. Blank (do not repeat the status word)

   Longer copy is **overlay-only**. Do not add a sixth inner row at any breakpoint.
5. **Card face forbids ledger-3 spend, token counts, and model breakdowns.** Teal tertiary extra-usage remaining (`$12.40`) and extra credits (`400 cr`) **are allowed** — they are not ccusage spend.
6. **Selection (spatial / row-major, card view):** `hjkl` and arrows share one mapping on the **visible card grid** of `cols` columns. Cards are in sort order, row-major (index `i` is row `i / cols`, column `i % cols`). Table view: `j`/`k` are ±1; `h`/`l` no-op.
   - `h` / `←`: `i - 1` **within the current row** (clamp at the row’s first index; no wrap to the previous row).
   - `l` / `→`: `i + 1` **within the current row** (clamp at the row’s last occupied index; no wrap).
   - `j` / `↓`: `i + cols` if that index exists; else no-op (no wrap to the top).
   - `k` / `↑`: `i - cols` if that index exists; else no-op.
   Wrap is **off**. After any move, scroll so the selection’s **card-row** is in view. Store selected `id`, not index, so a re-sort does not jump focus.

   Mixed 2-col (`cols = 2`): Codex | Grok / Claude | Kimi / z.ai | Muse. From `codex`, `l` → `grok`, `j` → `claude` (not Grok). 1-col: `j`/`k` are ±1; `h`/`l` are no-ops.

   Enter or Space opens a **detail overlay** (not a second page). Overlay: reset timestamps, raw windows (used **and** remaining), error text, docs URL. While an overlay is open, **movement keys are swallowed**. Overlay closes on `Esc`, `q`, or `Enter`. **Space does not close**. `q`/`Esc` on the board (no overlay) quit.
7. **Footer (exactly one row):** always the key hints `r refresh   o sort:risk   t view:table   ? help   q quit` (or `t view:cards`). When the viewport does not show every row (`visible_end < n`), **prefix** the pager: `1–3 / 6   r refresh   o sort:risk   t view:table   ? help   q quit`. Full boards **omit** the pager — keys only. The pager never replaces the keys and never wraps to a second footer row. The key row sits under the grid (not stranded at the bottom of a tall terminal).
8. **Empty / error states:** unsigned = `export ZAI_API_KEY` / `run claude` / `codex login` — never a red crash. Muse = explicit unsupported copy. Both paint the **dashed `unknown` bar** and **never a numeric `0%`**. “Never a fake bar” means never a 0% amber/red remaining gauge.
9. Help overlay lists keys. No mouse requirement.

## Overflow / scroll

Six cards × 7 rows + 1 header + 1 footer = **44 rows** in 1 column. At 80×24 the board is **2 columns × 3 rows** and all six cards fit.

- The card grid **scrolls vertically** by whole card rows when cards overflow the viewport.
- The **selected card-row is always in view**.
- **80×24** = full mixed board in 2 columns. Footer is keys only.
- **90×24** = same 2-col full board. Footer is keys only.
- **140×24** = full mixed board in 3 columns (2 rows × 7 + 2 = 16). Footer is keys only.
- **`(80, 48)`** same 2-col board with leftover terminal below the key row.

## Hero / secondary window (at most two rows on the card)

`SESSION_MAX_MINS = 360`. A window is session-class if `duration_mins <= 360`, or if `duration_mins` is `None` and `label == FiveHour`.

```text
hero      = shortest session-class window (by duration_mins, missing duration counts as 300)
            else Weekly if present
            else first Other
secondary = Weekly if present and not hero
            else the next-shortest session-class if any
            else none
further   = overlay only (e.g. Codex 15m + 60m + weekly → card shows 15m + weekly; 60m in overlay)
never invent a 5h bar when the only session window is 15m
caption   = duration_mins: 15 → "15m", 60 → "1h", 300 → "5h"; Weekly → "wk"
```

Grok SuperGrok often has weekly + extra credits and **no** session window. Hero is weekly. Do not draw an empty 5h gauge.

## Header vs card number

| Surface | Number |
| --- | --- |
| Header `worst` | `min(display_pct)` across snapshots that have windows (**Available and Error-with-stale**) |
| Card hero | shortest session-class, else weekly |
| Card secondary | Weekly if not hero, else next-shortest session-class |
| Sort key | `min(remaining_percent)` among snapshots that have windows, then soonest reset |

Claude 5h 72% / weekly 41%: header may say `Claude 41%`; card hero is **72%** (session fuel). Codex 18% still leads sort.

## Risk sort

Available and Error-with-stale are **one** remaining-sort group. A stale danger card is still on fire.

1. Group 0: has windows (`Available` **or** `Error` with stale Available) — **lowest** `min(windows.remaining_percent)` first, then soonest `resets_at` (missing reset last), then `id`
2. Group 1: `Error` without stale
3. Group 2: `NotConfigured`
4. Group 3: `Unsupported`

`--fixture error` (Codex 18% stale) **stays above** Kimi 55% Available. Name sort: `display_name` ascending, but still sinks groups 2–3. `o` toggles risk ↔ name.

## Refresh

- Manual `r` fetches **all** providers, including `OnDemand` (Muse later).
- Timer default **60s** fetches only `RefreshPolicy::Default` and `Interval` that are due. Muse `OnDemand` is **skipped** by the timer.
- **In-flight guard:** if `fetching`, **ignore the 60s timer**; **queue at most one extra `r`**. When the `JoinSet` drains, run that queued refresh. Never overlap fetches.
- **Per-provider timeout:** `10s`. Timeout → `Error { message: "timed out", stale: None }` then `apply_fetch`.
- **Panic:** `JoinError::is_panic()` → `Error { message: "adapter panicked", stale: None }` then `apply_fetch`. Do **not** tear down the alt screen.
- Fixture `fetch` uses `Default`. Interactive PR4+ (`SystemClock`, not `#[cfg(test)]`) adds **150 ms** `tokio::time::sleep` inside the boxed future so the spinner is screenshotable. Delay is **`Duration::ZERO` under `#[cfg(test)]` or when `Clock::is_frozen()`**.
- Per-provider isolation: `JoinSet` of `'static` boxed futures.

**Selection persistence:** store selected `id` (string), not index.

**Events:** do **not** `std::sync::mpsc::recv` on the runtime thread. Lock **`crossterm::EventStream` polled in `tokio::select!`** (PR4). Fallback: dedicated thread sending on `tokio::sync::mpsc` — never a blocking std channel on the UI task.

## Glyphs

Default glyphs are **unique 1-column ASCII** so Claude and Codex cannot collide.

| id | `glyph_ascii` | optional unicode (later, only if CI proves width 1) |
| --- | --- | --- |
| `claude` | `L` | `◆` |
| `codex` | `X` | `⌘` |
| `kimi` | `K` | `K` |
| `grok` | `G` | `✦` |
| `zai` | `Z` | `Z` |
| `muse` | `M` | `M` |
| `muse-web` | `W` | `W` |
| `fal` | `F` | `F` |
| `copilot` | `C` | `C` |
| `exa` | `E` | `E` |
| `firecrawl` | `N` | `N` |

v1 paints `glyph_ascii`. Unit test: every v1 id has a distinct `glyph_ascii`.
