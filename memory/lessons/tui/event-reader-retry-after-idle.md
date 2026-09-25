# event-reader-retry-after-idle

- **Domain:** tui
- **Date:** 2026-09-21
- **Pattern-Key:** event-reader-retry-after-idle
- **Confidence:** high
- **Evidence:** `crates/token-balance/src/event_loop.rs` `spawn_event_thread` / `classify`; `event_loop_test.rs`
- **Status:** active

## Situation

After the board sat unused, clicks and keys froze. The 1s tick still refreshed age/quotas, so the screen looked alive.

## Decision

Retry `poll`/`read` errors instead of exiting the reader thread. Respawn if the channel closes. Ignore mouse-move for redraw. Do **not** re-send `EnableMouseCapture` on every tick or FocusGained — that eats the click. Footer keys and a second click on the selected row are mouse hits. Rejected: EventStream (needs StreamExt).

## Reasoning

`Err(_) => break` killed the std reader after sleep/EINTR. Closed `rx` made `select!` spin on `None` while ticks only sometimes ran. Re-enabling mouse tracking every second reset button state so Down never arrived. Footer chrome looked clickable but was keys-only.

## Outcome

Reader stays up; clicks select rows, second click opens detail, footer `r/o/t/?/q` fire. Tick still refreshes without touching mouse mode.

## Lesson

Never exit the crossterm reader on poll/read error. Drain mouse-move without drawing. Never re-send EnableMouseCapture on a 1s timer.
