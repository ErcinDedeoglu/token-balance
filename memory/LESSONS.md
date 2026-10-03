# Repository lessons

Index of verified reusable findings. Canonical lesson bodies live in
[lessons/](lessons/), grouped by domain; do not duplicate them here or in MEMORY.
Include Pattern-Key, confidence, and an evidence pointer in each index entry.

## adapters

- [grok-reread-auth-on-fetch](lessons/adapters/grok-reread-auth-on-fetch.md) — adapters/grok-reread-auth-on-fetch — Grok JWT is re-read on each fetch; 401 means `grok login` after curl check
- [grok-oidc-refresh-same-as-cli](lessons/adapters/grok-oidc-refresh-same-as-cli.md) — adapters/grok-oidc-refresh-same-as-cli — refresh_token grant to auth.x.ai; persist rotated RT
- [grok-omitted-percent-is-fresh-week](lessons/adapters/grok-omitted-percent-is-fresh-week.md) — adapters/grok-omitted-percent-is-fresh-week — omitted creditUsagePercent on a weekly period is 0% used
- [exa-wreq-http1-checkpoint](lessons/adapters/exa-wreq-http1-checkpoint.md) — adapters/exa-wreq-http1-checkpoint — Exa remaining uses wreq Chrome131 HTTP/1; stock reqwest 429s Vercel
- [muse-429-is-exhausted-everyday](lessons/adapters/muse-429-is-exhausted-everyday.md) — adapters/muse-429-is-exhausted-everyday — Muse 429 + resets_at is 0% Everyday, not a dead token

## tui

- [table-default-retargets-card-oracles](lessons/tui/table-default-retargets-card-oracles.md) — tui/table-default-retargets-card-oracles — table-first default; card TestBackend oracles run after `t`
- [paragraph-bg-punches-block-fill](lessons/tui/paragraph-bg-punches-block-fill.md) — tui/paragraph-bg-punches-block-fill — selected table `Paragraph` cells must set `.bg` after a row Block fill
- [table-reset-skips-session](lessons/tui/table-reset-skips-session.md) — tui/table-reset-skips-session — table reset is weekly/monthly only, never the 5h clock
- [event-reader-retry-after-idle](lessons/tui/event-reader-retry-after-idle.md) — tui/event-reader-retry-after-idle — crossterm poll Err must retry; do not draw on mouse-move

## cli

- [cargo-run-home-is-rustup-home](lessons/cli/cargo-run-home-is-rustup-home.md) — cli/cargo-run-home-is-rustup-home — do not override `HOME` for `cargo`/`rustup`; isolate `tb`'s home on the built binary

## docs

- [design-folder-by-concern](lessons/docs/design-folder-by-concern.md) — docs/design-docs-split-by-concern — oversized designs go in `docs/design/` by concern

## quality

- [kit-refresh-invalidates-cohesion-hashes](lessons/quality/kit-refresh-invalidates-cohesion-hashes.md) — quality/kit-refresh-invalidates-cohesion-hashes — recompute cohesion-review hashes for files a kit refresh rewrites
- [control-files-need-nested-agents](lessons/quality/control-files-need-nested-agents.md) — quality/nested-agents-before-top-level-files — `quality/` still needs nested AGENTS.md
