# MEMORY.md

Index only. Keep ≤200 lines / 25 KB. Details live in linked files. Memory is a hint — verify against code.

## How to use

- Do not load this file every session. Open it when a Pattern-Key, path, or user correction matches the task.
- Read a linked lesson only when that match holds.
- After a verified outcome, copy `templates/lesson.md` to `lessons/<domain>/<slug>.md` and add one index bullet. Unknown domain → `lessons/_unsorted/`.
- After 3 recurrences of one Pattern-Key, copy `templates/pattern.md` to `patterns/<domain>/<key>.md` and propose one AGENTS.md rule (nested if domain-only). Do not write AGENTS.md without approval.
- Daily log: copy `templates/daily.md` to `daily/YYYY-MM-DD.md`.
- When a lesson is wrong, mark it superseded in place or delete it. Do not leave contradictions.

## Project

- Name: token-balance
- Stack: n/a (Rust 2024 + ratatui locked in `docs/design/`; no crate yet)
- Constraints: product design is `docs/design/` (split by concern)

## Lessons

- [control-files-need-nested-agents](lessons/quality/control-files-need-nested-agents.md) — quality/nested-agents-before-top-level-files — `quality/` still needs nested AGENTS.md
- [design-folder-by-concern](lessons/docs/design-folder-by-concern.md) — docs/design-docs-split-by-concern — oversized designs go in `docs/design/` by concern
- [grok-reread-auth-on-fetch](lessons/adapters/grok-reread-auth-on-fetch.md) — adapters/grok-reread-auth-on-fetch — Grok JWT is re-read on each fetch; 401 means `grok login` after curl check
- [exa-wreq-http1-checkpoint](lessons/adapters/exa-wreq-http1-checkpoint.md) — adapters/exa-wreq-http1-checkpoint — Exa remaining uses wreq Chrome131 HTTP/1; stock reqwest 429s Vercel
- [muse-429-is-exhausted-everyday](lessons/adapters/muse-429-is-exhausted-everyday.md) — adapters/muse-429-is-exhausted-everyday — Muse 429 + resets_at is 0% Everyday, not a dead token
- [table-default-retargets-card-oracles](lessons/tui/table-default-retargets-card-oracles.md) — tui/table-default-retargets-card-oracles — table-first default; card TestBackend oracles run after `t`
- [paragraph-bg-punches-block-fill](lessons/tui/paragraph-bg-punches-block-fill.md) — tui/paragraph-bg-punches-block-fill — selected table `Paragraph` cells must set `.bg` after a row Block fill
- [table-reset-skips-session](lessons/tui/table-reset-skips-session.md) — tui/table-reset-skips-session — table reset is weekly/monthly only, never the 5h clock

## Patterns

_None yet._

## Do not store

- Secrets, tokens, .env values
- Chat transcripts or daily logs (those belong in `daily/`)
- Commands already in AGENTS.md
- Unverified guesses
